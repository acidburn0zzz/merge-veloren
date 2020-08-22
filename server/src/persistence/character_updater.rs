use crate::comp;
use common::character::CharacterId;

use crossbeam::channel;
use diesel::Connection;
use tracing::{error, info};

pub type CharacterUpdateData = (comp::Stats, comp::Inventory, comp::Loadout);

/// A unidirectional messaging resource for saving characters in a
/// background thread.
///
/// This is used to make updates to a character and their persisted components,
/// such as inventory, loadout, etc...
pub struct CharacterUpdater {
    update_tx: Option<channel::Sender<Vec<(CharacterId, CharacterUpdateData)>>>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl CharacterUpdater {
    pub fn new(db_dir: String) -> Self {
        let (update_tx, update_rx) =
            channel::unbounded::<Vec<(CharacterId, CharacterUpdateData)>>();
        let handle = std::thread::spawn(move || {
            while let Ok(updates) = update_rx.recv() {
                info!("Persistence batch update starting");
                execute_batch_update(updates.into_iter(), &db_dir);
                info!("Persistence batch update finished");
            }
        });

        Self {
            update_tx: Some(update_tx),
            handle: Some(handle),
        }
    }

    /// Updates a collection of characters based on their id and components
    pub fn batch_update<'a>(
        &self,
        updates: impl Iterator<
            Item = (
                CharacterId,
                &'a comp::Stats,
                &'a comp::Inventory,
                &'a comp::Loadout,
            ),
        >,
    ) {
        let updates = updates
            .map(|(character_id, stats, inventory, loadout)| {
                (
                    character_id,
                    (stats.clone(), inventory.clone(), loadout.clone()),
                )
            })
            .collect::<Vec<(CharacterId, (comp::Stats, comp::Inventory, comp::Loadout))>>();

        if let Err(e) = self.update_tx.as_ref().unwrap().send(updates) {
            error!(?e, "Could not send stats updates");
        }
    }

    /// Updates a single character based on their id and components
    pub fn update(
        &self,
        character_id: CharacterId,
        stats: &comp::Stats,
        inventory: &comp::Inventory,
        loadout: &comp::Loadout,
    ) {
        self.batch_update(std::iter::once((character_id, stats, inventory, loadout)));
    }
}

fn execute_batch_update(
    updates: impl Iterator<Item = (CharacterId, CharacterUpdateData)>,
    db_dir: &str,
) {
    let connection = super::establish_connection(db_dir);

    if let Err(e) = connection.and_then(|connection| {
        connection.transaction::<_, diesel::result::Error, _>(|| {
            updates.for_each(|(character_id, (stats, inventory, loadout))| {
                // Create a nested transaction (savepoint) per character update so that a single
                // error for a particular character doesn't prevent all other characters being
                // saved
                if let Err(e) = connection.transaction::<_, super::error::Error, _>(|| {
                    super::character::update(character_id, stats, inventory, loadout, &connection)
                }) {
                    error!(?character_id, ?e, "Persistence update failed for character");
                }
            });

            Ok(())
        })
    }) {
        error!(?e, "Error during stats batch update transaction");
    }
}

impl Drop for CharacterUpdater {
    fn drop(&mut self) {
        drop(self.update_tx.take());
        if let Err(e) = self.handle.take().unwrap().join() {
            error!(?e, "Error from joining character update thread");
        }
    }
}
