//! Database operations related to character data
//!
//! Methods in this module should remain private to the persistence module -
//! database updates and loading are communicated via requests to the
//! [`CharacterLoader`] and [`CharacterUpdater`] while results/responses are
//! polled and handled each server tick.
extern crate diesel;

use super::{error::Error, establish_connection, models::*, schema};
use crate::{
    comp,
    persistence::{
        character_loader::{CharacterDataResult, CharacterListResult},
        conversions::{
            convert_body_from_database, convert_body_to_database_json,
            convert_character_from_database, convert_inventory_from_database_items,
            convert_inventory_to_database_items, convert_loadout_from_database_items,
            convert_loadout_to_database_items, convert_stats_from_database,
            convert_stats_to_database, ItemModelPair,
        },
        error::Error::DatabaseError,
        PersistedComponents,
    },
};
use common::character::{CharacterId, CharacterItem, MAX_CHARACTERS_PER_PLAYER};
use diesel::prelude::*;
use lazy_static::lazy_static;
use std::{collections::HashMap, sync::Mutex};
use tracing::{error, info};

pub(crate) type EntityId = i64;

const CHARACTER_PSEUDO_CONTAINER_DEF_ID: &str = "veloren.core.pseudo_containers.character";
const INVENTORY_PSEUDO_CONTAINER_DEF_ID: &str = "veloren.core.pseudo_containers.inventory";
const LOADOUT_PSEUDO_CONTAINER_DEF_ID: &str = "veloren.core.pseudo_containers.loadout";
const WORLD_PSEUDO_CONTAINER_ID: EntityId = 1;

#[derive(Clone, Copy)]
struct CharacterContainers {
    inventory_container_id: EntityId,
    loadout_container_id: EntityId,
}

// Cache of pseudo-container IDs per character to avoid further lookups after
// login
lazy_static! {
    static ref CHARACTER_CONTAINER_IDS: Mutex<HashMap<CharacterId, CharacterContainers>> =
        Mutex::new(HashMap::new());
}

/// Load stored data for a character.
///
/// After first logging in, and after a character is selected, we fetch this
/// data for the purpose of inserting their persisted data for the entity.
pub fn load_character_data(
    requesting_player_uuid: String,
    char_id: CharacterId,
    db_dir: &str,
) -> CharacterDataResult {
    use schema::{body::dsl::*, character::dsl::*, item::dsl::*, stats::dsl::*};
    let connection = establish_connection(db_dir)?;

    let character_containers = get_pseudo_containers(&connection, char_id)?;

    let inventory_items = item
        .filter(parent_container_item_id.eq(character_containers.inventory_container_id))
        .load::<Item>(&connection)?;

    let loadout_items = item
        .filter(parent_container_item_id.eq(character_containers.loadout_container_id))
        .load::<Item>(&connection)?;

    let (character_data, stats_data) = character
        .filter(
            schema::character::dsl::character_id
                .eq(char_id)
                .and(player_uuid.eq(requesting_player_uuid)),
        )
        .inner_join(stats)
        .first::<(Character, Stats)>(&connection)?;

    let char_body = body
        .filter(schema::body::dsl::body_id.eq(character_data.body_id))
        .first::<Body>(&connection)?;

    Ok((
        convert_body_from_database(&char_body)?,
        convert_stats_from_database(&stats_data, character_data.alias),
        convert_inventory_from_database_items(&inventory_items),
        convert_loadout_from_database_items(&loadout_items),
    ))
}

/// Loads a list of characters belonging to the player. This data is a small
/// subset of the character's data, and is used to render the character and
/// their level in the character list.
///
/// In the event that a join fails, for a character (i.e. they lack an entry for
/// stats, body, etc...) the character is skipped, and no entry will be
/// returned.
pub fn load_character_list(player_uuid_: &str, db_dir: &str) -> CharacterListResult {
    use schema::{body::dsl::*, character::dsl::*, item::dsl::*, stats::dsl::*};

    let connection = establish_connection(db_dir)?;

    let result = character
        .filter(player_uuid.eq(player_uuid_))
        .inner_join(stats)
        .order(schema::character::dsl::character_id.desc())
        .load::<(Character, Stats)>(&connection);

    match result {
        Ok(data) => Ok(data
            .iter()
            .map(|(character_data, char_stats)| {
                // TODO: Database failures here should skip the character, not crash the server
                let char = convert_character_from_database(character_data);

                let bodyx = body
                    .filter(schema::body::dsl::body_id.eq(character_data.body_id))
                    .first::<Body>(&connection)
                    .expect("failed to fetch body for character");

                let char_body = convert_body_from_database(&bodyx)
                    .expect("failed to convert body for character");

                let loadout_container_id = get_pseudo_container_id(
                    &connection,
                    char.id.unwrap(),
                    LOADOUT_PSEUDO_CONTAINER_DEF_ID,
                )
                .expect("failed to get loadout container for character");
                let loadout_items = item
                    .filter(parent_container_item_id.eq(loadout_container_id))
                    .load::<Item>(&connection)
                    .expect("failed to fetch loadout items for character");

                let loadout = convert_loadout_from_database_items(&loadout_items);

                CharacterItem {
                    character: char,
                    body: char_body,
                    level: char_stats.level as usize,
                    loadout,
                }
            })
            .collect()),
        Err(e) => {
            error!(?e, ?player_uuid, "Failed to load character list for player");
            Err(Error::CharacterDataError)
        },
    }
}

/// Create a new character with provided comp::Character and comp::Body data.
///
/// Note that sqlite does not support returning the inserted data after a
/// successful insert. To workaround, we wrap this in a transaction which
/// inserts, queries for the newly created character id, then uses the character
/// id for subsequent insertions
pub fn create_character(
    uuid: &str,
    character_alias: &str,
    persisted_components: PersistedComponents,
    db_dir: &str,
) -> CharacterListResult {
    use schema::item::dsl::*;

    check_character_limit(uuid, db_dir)?;

    let connection = establish_connection(db_dir)?;

    connection.transaction::<_, diesel::result::Error, _>(|| {
        use schema::{body, character, stats};

        let (body, stats, inventory, loadout) = persisted_components;

        // Insert body record
        let new_body = NewBody {
            body_id: None,
            body_data: convert_body_to_database_json(&body)
                .map_err(|x| diesel::result::Error::SerializationError(Box::new(x)))?,
            variant: "humanoid".to_string(),
        };

        diesel::insert_into(body::table)
            .values(&new_body)
            .execute(&connection)?;

        let body_id = body::table
            .order(schema::body::dsl::body_id.desc())
            .select(schema::body::dsl::body_id)
            .first::<i32>(&connection)?;

        // Insert character record
        let character_id = get_new_entity_id(&connection)?;
        let new_character = NewCharacter {
            character_id,
            body_id,
            player_uuid: uuid,
            alias: &character_alias,
        };
        diesel::insert_into(character::table)
            .values(&new_character)
            .execute(&connection)?;

        // Create pseudo-container items for character
        let inventory_container_id = get_new_entity_id(&connection)?;
        let loadout_container_id = get_new_entity_id(&connection)?;
        let pseudo_containers = vec![
            NewItem {
                stack_size: None,
                item_id: Some(character_id),
                parent_container_item_id: WORLD_PSEUDO_CONTAINER_ID,
                item_definition_id: CHARACTER_PSEUDO_CONTAINER_DEF_ID.to_owned(),
                position: None,
            },
            NewItem {
                stack_size: None,
                item_id: Some(inventory_container_id),
                parent_container_item_id: character_id,
                item_definition_id: INVENTORY_PSEUDO_CONTAINER_DEF_ID.to_owned(),
                position: None,
            },
            NewItem {
                stack_size: None,
                item_id: Some(loadout_container_id),
                parent_container_item_id: character_id,
                item_definition_id: LOADOUT_PSEUDO_CONTAINER_DEF_ID.to_owned(),
                position: None,
            },
        ];
        diesel::insert_into(item)
            .values(pseudo_containers)
            .execute(&connection)?;

        // Insert stats record
        let db_stats = convert_stats_to_database(character_id, &stats);
        diesel::insert_into(stats::table)
            .values(&db_stats)
            .execute(&connection)?;

        // Insert default inventory and loadout item records
        let mut item_pairs = convert_inventory_to_database_items(inventory, inventory_container_id);
        item_pairs.extend(convert_loadout_to_database_items(
            loadout,
            loadout_container_id,
        ));

        for mut item_pair in item_pairs.into_iter() {
            let id = get_new_entity_id(&connection)?;
            item_pair.model.item_id = Some(id);
            diesel::insert_into(item)
                .values(item_pair.model)
                .execute(&connection)?;
        }

        Ok(())
    })?;

    load_character_list(uuid, db_dir)
}

/// Delete a character. Returns the updated character list.
pub fn delete_character(uuid: &str, char_id: CharacterId, db_dir: &str) -> CharacterListResult {
    use schema::character::dsl::*;

    let connection = establish_connection(db_dir)?;
    connection.transaction::<_, diesel::result::Error, _>(|| {
        diesel::delete(
            character
                .filter(character_id.eq(char_id))
                .filter(player_uuid.eq(uuid)),
        )
        .execute(&connection)?;

        Ok(())
    })?;

    load_character_list(uuid, db_dir)
}

/// Before creating a character, we ensure that the limit on the number of
/// characters has not been exceeded
pub fn check_character_limit(uuid: &str, db_dir: &str) -> Result<(), Error> {
    use diesel::dsl::count_star;
    use schema::character::dsl::*;

    let character_count = character
        .select(count_star())
        .filter(player_uuid.eq(uuid))
        .load::<i64>(&establish_connection(db_dir)?)?;

    match character_count.first() {
        Some(count) => {
            if count < &(MAX_CHARACTERS_PER_PLAYER as i64) {
                Ok(())
            } else {
                Err(Error::CharacterLimitReached)
            }
        },
        _ => Ok(()),
    }
}

fn get_new_entity_id(conn: &SqliteConnection) -> Result<EntityId, diesel::result::Error> {
    use super::schema::entity::dsl::*;

    diesel::insert_into(entity).default_values().execute(conn)?;

    let new_entity_id = entity
        .order(entity_id.desc())
        .select(entity_id)
        .first::<EntityId>(conn)?;

    info!("Created new persistence entity_id: {}", new_entity_id);
    Ok(new_entity_id)
}

/// Fetches the pseudo_container IDs for a character, caching them in
/// CHARACTER_CONTAINER_IDS after the first lookup.
fn get_pseudo_containers(
    connection: &SqliteConnection,
    character_id: CharacterId,
) -> Result<CharacterContainers, Error> {
    let mut ids = CHARACTER_CONTAINER_IDS.lock().unwrap();
    if let Some(containers) = ids.get(&character_id) {
        return Ok(*containers);
    }

    let character_containers = CharacterContainers {
        loadout_container_id: get_pseudo_container_id(
            connection,
            character_id,
            LOADOUT_PSEUDO_CONTAINER_DEF_ID,
        )?,
        inventory_container_id: get_pseudo_container_id(
            connection,
            character_id,
            INVENTORY_PSEUDO_CONTAINER_DEF_ID,
        )?,
    };
    ids.insert(character_id, character_containers);

    Ok(character_containers)
}

fn get_pseudo_container_id(
    connection: &SqliteConnection,
    character_id: CharacterId,
    pseudo_container_id: &str,
) -> Result<EntityId, Error> {
    use super::schema::item::dsl::*;
    match item
        .select(item_id)
        .filter(
            parent_container_item_id
                .eq(character_id)
                .and(item_definition_id.eq(pseudo_container_id)),
        )
        .first::<EntityId>(connection)
    {
        Ok(id) => Ok(id),
        Err(e) => {
            error!(
                ?e,
                ?character_id,
                ?pseudo_container_id,
                "Failed to retrieve pseudo container ID"
            );
            Err(DatabaseError(e))
        },
    }
}

/// NOTE: Only call while a transaction is held!
pub fn update(
    char_id: CharacterId,
    char_stats: comp::Stats,
    inventory: comp::Inventory,
    loadout: comp::Loadout,
    connection: &SqliteConnection,
) -> Result<Vec<ItemModelPair>, Error> {
    use super::schema::{item::dsl::*, stats::dsl::*};

    let pseudo_containers = get_pseudo_containers(connection, char_id)?;

    let mut inserts =
        convert_inventory_to_database_items(inventory, pseudo_containers.inventory_container_id);
    inserts.extend(convert_loadout_to_database_items(
        loadout,
        pseudo_containers.loadout_container_id,
    ));

    // Move any items that already have an item_id to the update list
    let updates = inserts
        .drain_filter(|item_pair| item_pair.model.item_id.is_some())
        .collect::<Vec<ItemModelPair>>();

    // Fetch all existing items from the database for the character so that we can
    // use it to determine which items to delete (because they no longer exist in
    // the character's inventory or loadout)
    let mut existing_items = item
        .filter(
            parent_container_item_id
                .eq(pseudo_containers.inventory_container_id)
                .or(parent_container_item_id.eq(pseudo_containers.loadout_container_id)),
        )
        .load::<Item>(connection)?;
    // Any items that exist in the database and don't exist in the updates must have
    // been removed from the player and need deleting.
    existing_items.retain(|x| {
        !updates
            .iter()
            .any(|y| y.model.item_id.unwrap() == x.item_id)
    });

    for item_pair in updates.iter() {
        // This unwrap is safe because updates only contains models with Some(item_id)
        diesel::update(item.filter(item_id.eq(item_pair.model.item_id.unwrap())))
            .set(&item_pair.model)
            .execute(connection)?;
    }

    for mut item_pair in inserts.iter_mut() {
        let id = get_new_entity_id(connection)?;
        item_pair.model.item_id = Some(id);
        item_pair.new_item_id = id;
        diesel::insert_into(item)
            .values(item_pair.model.clone())
            .execute(connection)?;
    }

    let delete_ids = existing_items
        .iter()
        .map(|x| x.item_id)
        .collect::<Vec<i64>>();
    diesel::delete(item.filter(item_id.eq_any(delete_ids))).execute(connection)?;

    let db_stats = convert_stats_to_database(char_id, &char_stats);
    diesel::update(stats.filter(character_id.eq(char_id)))
        .set(db_stats)
        .execute(connection)?;

    Ok(inserts)
}
