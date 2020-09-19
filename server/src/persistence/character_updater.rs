use crate::comp;
use common::{character::CharacterId, comp::item::ItemId};

use std::sync::Arc;
use tracing::{error};
use crate::persistence::connection::VelorenConnectionPool;

pub type CharacterUpdateData<'a> = (&'a comp::Stats, &'a comp::Inventory, &'a comp::Loadout);

pub fn update(
    pool: &VelorenConnectionPool,
    character_id: CharacterId,
    stats: &comp::Stats,
    inventory: &comp::Inventory,
    loadout: &comp::Loadout,
) {
    execute_batch_update(pool, std::iter::once((character_id, (stats, inventory, loadout))));
}

pub fn execute_batch_update<'a, U>(pool: &VelorenConnectionPool, updates: U)
    where U: Iterator<Item = (CharacterId, CharacterUpdateData<'a>)>
{
    let mut inserted_items = Vec::<Arc<ItemId>>::new();

    let mut conn = pool.get_connection(); //TODO: error handling .expect("failed to get connection from pool");
    if let Err(e) = conn.transaction::<_, super::error::Error, _>(|txn| {
        for (character_id, (stats, inventory, loadout)) in updates {
            inserted_items.append(&mut super::character::update(
                character_id,
                stats,
                inventory,
                loadout,
                txn,
            )?);
        }

        Ok(())
    })
    {
        error!(?e, "Error during character batch update transaction");
    }

    // NOTE: On success, updating thee atomics is already taken care of
    // internally.
}
