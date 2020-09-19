use crate::{sys::{SysScheduler, SysTimer}, persistence};
use common::{
    comp::{Inventory, Loadout, Player, Stats},
    span,
};
use specs::{Join, ReadStorage, System, Write};
use crate::persistence::connection::VelorenConnectionPool;

use tracing::{debug, error};
pub struct Sys;

impl<'a> System<'a> for Sys {
    #[allow(clippy::type_complexity)] // TODO: Pending review in #587
    type SystemData = (
        ReadStorage<'a, Player>,
        ReadStorage<'a, Stats>,
        ReadStorage<'a, Inventory>,
        ReadStorage<'a, Loadout>,
        Write<'a, VelorenConnectionPool>,
        Write<'a, SysScheduler<Self>>,
        Write<'a, SysTimer<Self>>,
    );

    fn run(
        &mut self,
        (
            players,
            player_stats,
            player_inventories,
            player_loadouts,
            mut connection_pool,
            mut scheduler,
            mut timer,
        ): Self::SystemData,
    ) {
        span!(_guard, "run", "persistence::Sys::run");


        if scheduler.should_run() {
            timer.start();
            debug!("Starting persistence update");
            persistence::character_updater::execute_batch_update(&mut connection_pool,
                (
                    &players,
                    &player_stats,
                    &player_inventories,
                    &player_loadouts,
                )
                    .join()
                    .filter_map(|(player, stats, inventory, loadout)| {
                        player
                            .character_id
                            .map(|id| (id, (stats, inventory, loadout)))
                    }),
            );
            debug!("Finished persistence update");
            timer.end();
        }
    }
}
