use crate::persistence::{
    character::EntityId,
    models::{Body, Character, Item, NewItem, Stats},
};

use crate::persistence::{error::Error, json_models::HumanoidBody};
use common::{
    character::CharacterId,
    comp::{Body as CompBody, *},
    loadout_builder,
};
use std::{convert::TryFrom, num::NonZeroU64};

#[derive(PartialEq)]
pub struct ItemModelPair {
    pub comp: common::comp::item::Item,
    pub model: NewItem,
    pub new_item_id: EntityId,
}

pub fn convert_inventory_to_database_items(
    inventory: Inventory,
    inventory_container_id: EntityId,
) -> Vec<ItemModelPair> {
    inventory
        .slots
        .into_iter()
        .enumerate()
        .map(|(slot, item)| {
            if let Some(item) = item {
                Some(ItemModelPair {
                    model: NewItem {
                        item_definition_id: item.item_definition_id().to_owned(),
                        position: Some(slot.to_string()),
                        parent_container_item_id: inventory_container_id,
                        item_id: match item.item_id.load() {
                            Some(item_id) => Some(u64::from(item_id) as EntityId),
                            _ => None,
                        },
                        stack_size: Some(item.amount() as i32),
                    },
                    comp: item,
                    new_item_id: 0,
                })
            } else {
                None
            }
        })
        .filter_map(|x| x)
        .collect()
}

pub fn convert_loadout_to_database_items(
    loadout: Loadout,
    loadout_container_id: EntityId,
) -> Vec<ItemModelPair> {
    vec![
        loadout.active_item.map(|x| ("active_item", x.item)),
        loadout.second_item.map(|x| ("second_item", x.item)),
        loadout.lantern.map(|x| ("lantern", x)),
        loadout.shoulder.map(|x| ("shoulder", x)),
        loadout.chest.map(|x| ("chest", x)),
        loadout.belt.map(|x| ("belt", x)),
        loadout.hand.map(|x| ("hand", x)),
        loadout.pants.map(|x| ("pants", x)),
        loadout.foot.map(|x| ("foot", x)),
        loadout.back.map(|x| ("back", x)),
        loadout.ring.map(|x| ("ring", x)),
        loadout.neck.map(|x| ("neck", x)),
        loadout.head.map(|x| ("head", x)),
        loadout.tabard.map(|x| ("tabard", x)),
    ]
    .into_iter()
    .filter_map(|x| x)
    .map(move |x| {
        let (slot, item) = x;
        ItemModelPair {
            model: NewItem {
                item_definition_id: item.item_definition_id().to_owned(),
                position: Some((*slot).to_owned()),
                parent_container_item_id: loadout_container_id,
                item_id: match item.item_id.load() {
                    Some(item_id) => Some(u64::from(item_id) as EntityId),
                    _ => None,
                },
                stack_size: None, // Armor/weapons cannot have stack sizes
            },
            comp: item,
            new_item_id: 0,
        }
    })
    .collect()
}

pub fn convert_body_to_database_json(body: &CompBody) -> Result<String, serde_json::Error> {
    let json_model = match body {
        common::comp::Body::Humanoid(humanoid_body) => HumanoidBody::from(humanoid_body),
        _ => unimplemented!("Only humanoid bodies are currently supported for persistence"),
    };

    serde_json::to_string(&json_model)
}

pub fn convert_stats_to_database(character_id: CharacterId, stats: &common::comp::Stats) -> Stats {
    Stats {
        character_id,
        level: stats.level.level() as i32,
        exp: stats.exp.current() as i32,
        endurance: stats.endurance as i32,
        fitness: stats.fitness as i32,
        willpower: stats.willpower as i32,
        skills: Some("".to_owned()), // TODO: actual skillset
    }
}

pub fn convert_inventory_from_database_items(database_items: &[Item]) -> Result<Inventory, Error> {
    let mut inventory = Inventory::new_empty();
    for db_item in database_items.iter() {
        let mut item = common::comp::Item::new_from_asset(db_item.item_definition_id.as_str())
            .map_err(|_| {
                Error::ConversionError(format!(
                    "Error loading item asset: {}",
                    db_item.item_definition_id
                ))
            })?;

        // Item ID
        item.item_id
            .store(Some(NonZeroU64::try_from(db_item.item_id as u64).map_err(
                |_| Error::ConversionError("Item with zero item_id".to_owned()),
            )?));

        // Stack Size
        if let Some(amount) = db_item.stack_size {
            item.set_amount(amount as u32)
                .map_err(|_| Error::ConversionError("Error setting amount for item".to_owned()))?;
        }

        // Insert item into inventory
        if let Some(slot_str) = &db_item.position {
            // Slot position
            let slot = slot_str.parse::<usize>().map_err(|_| {
                Error::ConversionError(format!("Failed to parse item position: {}", slot_str))
            })?;

            match inventory.insert(slot, item).map_err(|_| {
                // If this happens there were too many items in the database for the current
                // inventory size
                Error::ConversionError("Error inserting item into inventory".to_string())
            })? {
                Some(_) => {
                    // If inventory.insert returns an item, it means it was swapped for an item that
                    // already occupied the slot. Multiple items being stored in the database for
                    // the same slot is an error.
                    Err(Error::ConversionError(
                        "Inserted an item into the same slot twice".to_string(),
                    ))
                },
                _ => Ok(()),
            }
        } else {
            Err(Error::ConversionError(
                "Item without slot position".to_string(),
            ))
        }?
    }

    Ok(inventory)
}

pub fn convert_loadout_from_database_items(database_items: &[Item]) -> Result<Loadout, Error> {
    let mut loadout = loadout_builder::LoadoutBuilder::new();
    for db_item in database_items.iter() {
        let item = common::comp::Item::new_from_asset_expect(db_item.item_definition_id.as_str());
        item.item_id
            .store(Some(NonZeroU64::try_from(db_item.item_id as u64).map_err(
                |_| Error::ConversionError("Item with zero item_id".to_owned()),
            )?));
        if let Some(position) = &db_item.position {
            match position.as_str() {
                "active_item" => loadout = loadout.active_item(Some(ItemConfig::from(item))),
                "second_item" => loadout = loadout.second_item(Some(ItemConfig::from(item))),
                "lantern" => loadout = loadout.lantern(Some(item)),
                "shoulder" => loadout = loadout.shoulder(Some(item)),
                "chest" => loadout = loadout.chest(Some(item)),
                "belt" => loadout = loadout.belt(Some(item)),
                "hand" => loadout = loadout.hand(Some(item)),
                "pants" => loadout = loadout.pants(Some(item)),
                "foot" => loadout = loadout.foot(Some(item)),
                "back" => loadout = loadout.back(Some(item)),
                "ring" => loadout = loadout.ring(Some(item)),
                "neck" => loadout = loadout.neck(Some(item)),
                "head" => loadout = loadout.head(Some(item)),
                "tabard" => loadout = loadout.tabard(Some(item)),
                _ => {
                    return Err(Error::ConversionError(format!(
                        "Unknown loadout position on item: {}",
                        position.as_str()
                    )));
                },
            }
        }
    }

    Ok(loadout.build())
}

pub fn convert_body_from_database(body: &Body) -> Result<CompBody, serde_json::Error> {
    Ok(match body.variant.as_str() {
        "humanoid" => {
            let json_model = serde_json::de::from_str::<HumanoidBody>(&body.body_data)?;
            CompBody::Humanoid(common::comp::humanoid::Body {
                species: common::comp::humanoid::ALL_SPECIES[json_model.species as usize],
                body_type: common::comp::humanoid::ALL_BODY_TYPES[json_model.body_type as usize],
                hair_style: json_model.hair_style,
                beard: json_model.beard,
                eyes: json_model.eyes,
                accessory: json_model.accessory,
                hair_color: json_model.hair_color,
                skin: json_model.skin,
                eye_color: json_model.eye_color,
            })
        },
        _ => unimplemented!("x"),
    })
}

pub fn convert_character_from_database(character: &Character) -> common::character::Character {
    common::character::Character {
        id: Some(character.character_id),
        alias: String::from(&character.alias),
    }
}

pub fn convert_stats_from_database(stats: &Stats, alias: String) -> common::comp::Stats {
    let mut new_stats = common::comp::Stats::default();
    new_stats.name = alias;
    new_stats.level.set_level(stats.level as u32);
    new_stats.exp.set_current(stats.exp as u32);
    new_stats.update_max_hp(new_stats.body_type);
    new_stats.health.set_to(
        new_stats.health.maximum(),
        common::comp::HealthSource::Revive,
    );
    new_stats.endurance = stats.endurance as u32;
    new_stats.fitness = stats.fitness as u32;
    new_stats.willpower = stats.willpower as u32;

    // TODO: Skillset

    new_stats
}
