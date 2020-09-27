use crate::{client::Client, comp::quadruped_small, Server, SpawnPoint, StateExt};
use common::{
    assets::Asset,
    comp::{
        self,
        chat::{KillSource, KillType},
        object, Alignment, Body, Damage, DamageSource, Group, HealthChange, HealthSource, Item,
        Player, Pos, Stats,
    },
    lottery::Lottery,
    msg::{PlayerListUpdate, ServerMsg},
    outcome::Outcome,
    state::BlockChange,
    sync::{Uid, UidAllocator, WorldSyncExt},
    sys::combat::BLOCK_ANGLE,
    terrain::{Block, BlockKind, TerrainGrid},
    vol::ReadVol,
    util::Dir
};
use comp::item::Reagent;
use rand::prelude::*;
use specs::{join::Join, saveload::MarkerAllocator, Builder, Entity as EcsEntity, WorldExt};
use tracing::error;
use vek::Vec3;

pub fn handle_damage(server: &Server, uid: Uid, change: HealthChange) {
    let state = &server.state;
    let ecs = state.ecs();
    if let Some(entity) = ecs.entity_from_uid(uid.into()) {
        if let Some(stats) = ecs.write_storage::<Stats>().get_mut(entity) {
            stats.health.change_by(change);
            ecs.write_resource::<Vec<Outcome>>()
                .push(Outcome::Damage { uid: uid.0, change });
        }
    }
}

pub fn handle_knockback(server: &Server, entity: EcsEntity, impulse: Vec3<f32>) {
    let state = &server.state;
    let mut velocities = state.ecs().write_storage::<comp::Vel>();
    if let Some(vel) = velocities.get_mut(entity) {
        vel.0 = impulse;
    }
    let mut clients = state.ecs().write_storage::<Client>();
    if let Some(client) = clients.get_mut(entity) {
        client.notify(ServerMsg::Knockback(impulse));
    }
}

/// Handle an entity dying. If it is a player, it will send a message to all
/// other players. If the entity that killed it had stats, then give it exp for
/// the kill. Experience given is equal to the level of the entity that was
/// killed times 10.
// NOTE: Clippy incorrectly warns about a needless collect here because it does not
// understand that the pet count (which is computed during the first iteration over the
// members in range) is actually used by the second iteration over the members in range;
// since we have no way of knowing the pet count before the first loop finishes, we
// definitely need at least two loops.   Then (currently) our only options are to store
// the member list in temporary space (e.g. by collecting to a vector), or to repeat
// the loop; but repeating the loop would currently be very inefficient since it has to
// rescan every entity on the server again.
#[allow(clippy::needless_collect)]
pub fn handle_destroy(server: &mut Server, entity: EcsEntity, cause: HealthSource) {
    let state = server.state_mut();

    // TODO: Investigate duplicate `Destroy` events (but don't remove this).
    // If the entity was already deleted, it can't be destroyed again.
    if !state.ecs().is_alive(entity) {
        return;
    }

    if let Some(uid) = state.ecs().read_storage::<Uid>().get(entity) {
        state
            .ecs()
            .write_resource::<Vec<Outcome>>()
            .push(Outcome::Destruction { uid: uid.0, cause });

        tracing::warn!("Created Outcome::Destruction {} {:?}", uid.0, cause);
    }

    // Chat message
    // If it was a player that died
    if let Some(_player) = state.ecs().read_storage::<Player>().get(entity) {
        if let Some(uid) = state.ecs().read_storage::<Uid>().get(entity) {
            let kill_source = match cause {
                HealthSource::Attack { by } => {
                    // Get attacker entity
                    if let Some(char_entity) = state.ecs().entity_from_uid(by.into()) {
                        // Check if attacker is another player or entity with stats (npc)
                        if state
                            .ecs()
                            .read_storage::<Player>()
                            .get(char_entity)
                            .is_some()
                        {
                            KillSource::Player(by, KillType::Melee)
                        } else if let Some(stats) =
                            state.ecs().read_storage::<Stats>().get(char_entity)
                        {
                            KillSource::NonPlayer(stats.name.clone(), KillType::Melee)
                        } else {
                            KillSource::NonPlayer("<?>".to_string(), KillType::Melee)
                        }
                    } else {
                        KillSource::NonPlayer("<?>".to_string(), KillType::Melee)
                    }
                },
                HealthSource::Projectile { owner: Some(by) } => {
                    // Get projectile owner entity TODO: add names to projectiles and send in
                    // message
                    if let Some(char_entity) = state.ecs().entity_from_uid(by.into()) {
                        // Check if attacker is another player or entity with stats (npc)
                        if state
                            .ecs()
                            .read_storage::<Player>()
                            .get(char_entity)
                            .is_some()
                        {
                            KillSource::Player(by, KillType::Projectile)
                        } else if let Some(stats) =
                            state.ecs().read_storage::<Stats>().get(char_entity)
                        {
                            KillSource::NonPlayer(stats.name.clone(), KillType::Projectile)
                        } else {
                            KillSource::NonPlayer("<?>".to_string(), KillType::Projectile)
                        }
                    } else {
                        KillSource::NonPlayer("<?>".to_string(), KillType::Projectile)
                    }
                },
                HealthSource::Explosion { owner: Some(by) } => {
                    // Get explosion owner entity
                    if let Some(char_entity) = state.ecs().entity_from_uid(by.into()) {
                        // Check if attacker is another player or entity with stats (npc)
                        if state
                            .ecs()
                            .read_storage::<Player>()
                            .get(char_entity)
                            .is_some()
                        {
                            KillSource::Player(by, KillType::Explosion)
                        } else if let Some(stats) =
                            state.ecs().read_storage::<Stats>().get(char_entity)
                        {
                            KillSource::NonPlayer(stats.name.clone(), KillType::Explosion)
                        } else {
                            KillSource::NonPlayer("<?>".to_string(), KillType::Explosion)
                        }
                    } else {
                        KillSource::NonPlayer("<?>".to_string(), KillType::Explosion)
                    }
                },
                HealthSource::Energy { owner: Some(by) } => {
                    // Get energy owner entity
                    if let Some(char_entity) = state.ecs().entity_from_uid(by.into()) {
                        // Check if attacker is another player or entity with stats (npc)
                        if state
                            .ecs()
                            .read_storage::<Player>()
                            .get(char_entity)
                            .is_some()
                        {
                            KillSource::Player(by, KillType::Energy)
                        } else if let Some(stats) =
                            state.ecs().read_storage::<Stats>().get(char_entity)
                        {
                            KillSource::NonPlayer(stats.name.clone(), KillType::Energy)
                        } else {
                            KillSource::NonPlayer("<?>".to_string(), KillType::Energy)
                        }
                    } else {
                        KillSource::NonPlayer("<?>".to_string(), KillType::Energy)
                    }
                },
                HealthSource::World => KillSource::FallDamage,
                HealthSource::Suicide => KillSource::Suicide,
                HealthSource::Projectile { owner: None }
                | HealthSource::Explosion { owner: None }
                | HealthSource::Energy { owner: None }
                | HealthSource::Revive
                | HealthSource::Command
                | HealthSource::LevelUp
                | HealthSource::Item
                | HealthSource::Healing { by: _ }
                | HealthSource::Unknown => KillSource::Other,
            };
            state.notify_registered_clients(
                comp::ChatType::Kill(kill_source, *uid).server_msg("".to_string()),
            );
        }
    }

    // Give EXP to the killer if entity had stats
    (|| {
        let mut stats = state.ecs().write_storage::<Stats>();
        let by = if let HealthSource::Attack { by }
        | HealthSource::Projectile { owner: Some(by) }
        | HealthSource::Energy { owner: Some(by) }
        | HealthSource::Explosion { owner: Some(by) } = cause
        {
            by
        } else {
            return;
        };
        let attacker = if let Some(attacker) = state.ecs().entity_from_uid(by.into()) {
            attacker
        } else {
            return;
        };
        let entity_stats = if let Some(entity_stats) = stats.get(entity) {
            entity_stats
        } else {
            return;
        };

        let groups = state.ecs().read_storage::<Group>();
        let attacker_group = groups.get(attacker);
        let destroyed_group = groups.get(entity);
        // Don't give exp if attacker destroyed themselves or one of their group members
        if (attacker_group.is_some() && attacker_group == destroyed_group) || attacker == entity {
            return;
        }

        // Maximum distance for other group members to receive exp
        const MAX_EXP_DIST: f32 = 150.0;
        // Attacker gets same as exp of everyone else
        const ATTACKER_EXP_WEIGHT: f32 = 1.0;
        let mut exp_reward = (entity_stats.body_type.base_exp()
            + entity_stats.level.level() * entity_stats.body_type.base_exp_increase())
            as f32;

        // Distribute EXP to group
        let positions = state.ecs().read_storage::<Pos>();
        let alignments = state.ecs().read_storage::<Alignment>();
        let uids = state.ecs().read_storage::<Uid>();
        if let (Some(attacker_group), Some(pos)) = (attacker_group, positions.get(entity)) {
            // TODO: rework if change to groups makes it easier to iterate entities in a
            // group
            let mut num_not_pets_in_range = 0;
            let members_in_range = (
                &state.ecs().entities(),
                &groups,
                &positions,
                alignments.maybe(),
                &uids,
            )
                .join()
                .filter(|(entity, group, member_pos, _, _)| {
                    // Check if: in group, not main attacker, and in range
                    *group == attacker_group
                        && *entity != attacker
                        && pos.0.distance_squared(member_pos.0) < MAX_EXP_DIST.powi(2)
                })
                .map(|(entity, _, _, alignment, uid)| {
                    if !matches!(alignment, Some(Alignment::Owned(owner)) if owner != uid) {
                        num_not_pets_in_range += 1;
                    }

                    entity
                })
                .collect::<Vec<_>>();
            let exp = exp_reward / (num_not_pets_in_range as f32 + ATTACKER_EXP_WEIGHT);
            exp_reward = exp * ATTACKER_EXP_WEIGHT;
            members_in_range.into_iter().for_each(|e| {
                if let Some(stats) = stats.get_mut(e) {
                    stats.exp.change_by(exp.ceil() as i64);
                }
            });
        }

        if let Some(attacker_stats) = stats.get_mut(attacker) {
            // TODO: Discuss whether we should give EXP by Player
            // Killing or not.
            attacker_stats.exp.change_by(exp_reward.ceil() as i64);
        }
    })();

    if state
        .ecs()
        .write_storage::<Client>()
        .get_mut(entity)
        .is_some()
    {
        state
            .ecs()
            .write_storage()
            .insert(entity, comp::Vel(Vec3::zero()))
            .err()
            .map(|e| error!(?e, ?entity, "Failed to set zero vel on dead client"));
        state
            .ecs()
            .write_storage()
            .insert(entity, comp::ForceUpdate)
            .err()
            .map(|e| error!(?e, ?entity, "Failed to insert ForceUpdate on dead client"));
        state
            .ecs()
            .write_storage::<comp::LightEmitter>()
            .remove(entity);
        state
            .ecs()
            .write_storage::<comp::Energy>()
            .get_mut(entity)
            .map(|energy| energy.set_to(energy.maximum(), comp::EnergySource::Revive));
        let _ = state
            .ecs()
            .write_storage::<comp::CharacterState>()
            .insert(entity, comp::CharacterState::default());
    } else if state.ecs().read_storage::<comp::Agent>().contains(entity) {
        use specs::Builder;

        // Decide for a loot drop before turning into a lootbag
        let old_body = state.ecs().write_storage::<Body>().remove(entity);
        let mut rng = rand::thread_rng();
        let mut lottery = || {
            Lottery::<String>::load_expect(match old_body {
                Some(common::comp::Body::Humanoid(_)) => match rng.gen_range(0, 4) {
                    0 => "common.loot_tables.loot_table_humanoids",
                    1 => "common.loot_tables.loot_table_armor_light",
                    2 => "common.loot_tables.loot_table_armor_cloth",
                    3 => "common.loot_tables.loot_table_weapon_common",
                    _ => "common.loot_tables.loot_table_humanoids",
                },
                Some(common::comp::Body::QuadrupedSmall(quadruped_small)) => {
                    match quadruped_small.species {
                        quadruped_small::Species::Dodarock => match rng.gen_range(0, 6) {
                            0 => "common.loot_tables.loot_table_armor_misc",
                            1 => "common.loot_tables.loot_table_rocks",
                            _ => "common.loot_tables.loot_table_rocks",
                        },
                        _ => match rng.gen_range(0, 4) {
                            0 => "common.loot_tables.loot_table_food",
                            1 => "common.loot_tables.loot_table_armor_misc",
                            2 => "common.loot_tables.loot_table_animal_parts",
                            _ => "common.loot_tables.loot_table_animal_parts",
                        },
                    }
                },
                Some(common::comp::Body::QuadrupedMedium(_)) => match rng.gen_range(0, 4) {
                    0 => "common.loot_tables.loot_table_food",
                    1 => "common.loot_tables.loot_table_armor_misc",
                    2 => "common.loot_tables.loot_table_animal_parts",
                    _ => "common.loot_tables.loot_table_animal_parts",
                },
                Some(common::comp::Body::BirdMedium(_)) => match rng.gen_range(0, 3) {
                    0 => "common.loot_tables.loot_table_food",
                    1 => "common.loot_tables.loot_table_armor_misc",
                    _ => "common.loot_tables.loot_table",
                },
                Some(common::comp::Body::BipedLarge(_)) => match rng.gen_range(0, 8) {
                    0 => "common.loot_tables.loot_table_food",
                    1 => "common.loot_tables.loot_table_armor_nature",
                    3 => "common.loot_tables.loot_table_armor_heavy",
                    5 => "common.loot_tables.loot_table_weapon_uncommon",
                    6 => "common.loot_tables.loot_table_weapon_rare",
                    _ => "common.loot_tables.loot_table_cave_large",
                },
                Some(common::comp::Body::Golem(_)) => match rng.gen_range(0, 9) {
                    0 => "common.loot_tables.loot_table_food",
                    1 => "common.loot_tables.loot_table_armor_misc",
                    2 => "common.loot_tables.loot_table_armor_light",
                    3 => "common.loot_tables.loot_table_armor_heavy",
                    4 => "common.loot_tables.loot_table_armor_misc",
                    5 => "common.loot_tables.loot_table_weapon_common",
                    6 => "common.loot_tables.loot_table_weapon_uncommon",
                    7 => "common.loot_tables.loot_table_weapon_rare",
                    _ => "common.loot_tables.loot_table",
                },
                Some(common::comp::Body::Theropod(_)) => {
                    "common.loot_tables.loot_table_animal_parts"
                },
                Some(common::comp::Body::Dragon(_)) => "common.loot_tables.loot_table_weapon_rare",
                Some(common::comp::Body::QuadrupedLow(_)) => match rng.gen_range(0, 3) {
                    0 => "common.loot_tables.loot_table_food",
                    1 => "common.loot_tables.loot_table_animal_parts",
                    _ => "common.loot_tables.loot_table",
                },
                _ => "common.loot_tables.loot_table",
            })
        };

        let item = {
            let mut item_drops = state.ecs().write_storage::<comp::ItemDrop>();
            item_drops.remove(entity).map_or_else(
                || Item::new_from_asset_expect(lottery().choose()),
                |item_drop| item_drop.0,
            )
        };

        let pos = state.ecs().read_storage::<comp::Pos>().get(entity).cloned();
        if let Some(pos) = pos {
            let _ = state
                .create_object(
                    comp::Pos(pos.0 + Vec3::unit_z() * 0.25),
                    object::Body::Pouch,
                )
                .with(item)
                .build();
        } else {
            error!(
                ?entity,
                "Entity doesn't have a position, no bag is being dropped"
            )
        }

        let _ = state
            .delete_entity_recorded(entity)
            .map_err(|e| error!(?e, ?entity, "Failed to delete destroyed entity"));
    } else {
        let _ = state
            .delete_entity_recorded(entity)
            .map_err(|e| error!(?e, ?entity, "Failed to delete destroyed entity"));
    }

    // TODO: Add Delete(time_left: Duration) component
    /*
    // If not a player delete the entity
    if let Err(err) = state.delete_entity_recorded(entity) {
        error!(?e, "Failed to delete destroyed entity");
    }
    */
}

pub fn handle_land_on_ground(server: &Server, entity: EcsEntity, vel: Vec3<f32>) {
    let state = &server.state;
    if vel.z <= -30.0 {
        if let Some(stats) = state.ecs().write_storage::<comp::Stats>().get_mut(entity) {
            let falldmg = (vel.z.powi(2) / 20.0 - 40.0) * 10.0;
            let mut damage = Damage {
                healthchange: -falldmg,
                source: DamageSource::Falling,
            };
            if let Some(loadout) = state.ecs().read_storage::<comp::Loadout>().get(entity) {
                damage.modify_damage(false, loadout);
            }
            stats.health.change_by(comp::HealthChange {
                amount: damage.healthchange as i32,
                cause: comp::HealthSource::World,
            });
        }
    }
}

pub fn handle_respawn(server: &Server, entity: EcsEntity) {
    let state = &server.state;

    // Only clients can respawn
    if state
        .ecs()
        .write_storage::<Client>()
        .get_mut(entity)
        .is_some()
    {
        let respawn_point = state
            .read_component_copied::<comp::Waypoint>(entity)
            .map(|wp| wp.get_pos())
            .unwrap_or(state.ecs().read_resource::<SpawnPoint>().0);

        state
            .ecs()
            .write_storage::<comp::Stats>()
            .get_mut(entity)
            .map(|stats| stats.revive());
        state
            .ecs()
            .write_storage::<comp::Pos>()
            .get_mut(entity)
            .map(|pos| pos.0 = respawn_point);
        state
            .ecs()
            .write_storage()
            .insert(entity, comp::ForceUpdate)
            .err()
            .map(|e| {
                error!(
                    ?e,
                    "Error inserting ForceUpdate component when respawning client"
                )
            });
    }
}

pub fn handle_explosion(
    server: &mut Server,
    pos: Vec3<f32>,
    power: f32,
    owner: Option<Uid>,
    friendly_damage: bool,
    reagent: Option<Reagent>,
    percent_damage: f32,
) {
    // Go through all other entities
    let hit_range = 3.0 * power;
    let ecs = server.state.ecs_mut();

    let owner_entity = owner.and_then(|uid| {
        ecs.read_resource::<UidAllocator>()
            .retrieve_entity_internal(uid.into())
    });

    for (entity_b, pos_b, ori_b, character_b, stats_b, loadout_b) in (
        &ecs.entities(),
        &ecs.read_storage::<comp::Pos>(),
        &ecs.read_storage::<comp::Ori>(),
        ecs.read_storage::<comp::CharacterState>().maybe(),
        &mut ecs.write_storage::<comp::Stats>(),
        ecs.read_storage::<comp::Loadout>().maybe(),
    )
        .join()
    {
        let groups = ecs.read_storage::<comp::Group>();

        let distance_squared = pos.distance_squared(pos_b.0);
        // Check if it is a hit
        if !stats_b.is_dead
            // RADIUS
            && distance_squared < hit_range.powi(2)
        {
            // See if entities are in the same group
            let mut same_group = owner_entity
                .and_then(|e| groups.get(e))
                .map_or(false, |group_a| Some(group_a) == groups.get(entity_b));
            if let Some(entity) = owner_entity {
                if entity == entity_b {
                    same_group = true;
                }
            }
            // Don't heal if outside group
            // Don't damage in the same group
            let is_damage = (friendly_damage || !same_group) && (percent_damage > 0.0);
            let is_heal = same_group && (percent_damage < 1.0);
            if !is_heal && !is_damage {
                continue;
            }

            // Weapon gives base damage
            let source = if is_heal {
                DamageSource::Healing
            } else {
                DamageSource::Explosion
            };
            let strength = (1.0 - distance_squared / hit_range.powi(2)) * power * 130.0;
            let healthchange = if is_heal {
                strength * (1.0 - percent_damage)
            } else {
                -strength * percent_damage
            };

            let mut damage = Damage {
                healthchange,
                source,
            };

            let block = character_b.map(|c_b| c_b.is_block()).unwrap_or(false)
                && ori_b.0.angle_between(pos - pos_b.0) < BLOCK_ANGLE.to_radians() / 2.0;

            if let Some(loadout) = loadout_b {
                damage.modify_damage(block, loadout);
            }

            if damage.healthchange != 0.0 {
                let cause = if is_heal {
                    HealthSource::Healing { by: owner }
                } else {
                    HealthSource::Explosion { owner }
                };
                stats_b.health.change_by(HealthChange {
                    amount: damage.healthchange as i32,
                    cause,
                });
            }
        }
    }

    let mut exploded_blocks = hashbrown::HashMap::new();

    const RAYS: usize = 500;

    if percent_damage > 0.9 {
        // Color terrain
        let mut touched_blocks = Vec::new();
        let color_range = power * 2.7;
        for _ in 0..RAYS {
            let dir = Vec3::new(
                rand::random::<f32>() - 0.5,
                rand::random::<f32>() - 0.5,
                rand::random::<f32>() - 0.5,
            )
            .normalized();

            let _ = ecs
                .read_resource::<TerrainGrid>()
                .ray(pos, pos + dir * color_range)
                // TODO: Faster RNG
                .until(|_| rand::random::<f32>() < 0.05)
                .for_each(|_: &Block, pos| touched_blocks.push(pos))
                .cast();
        }

        let terrain = ecs.read_resource::<TerrainGrid>();
        let mut block_change = ecs.write_resource::<BlockChange>();
        for block_pos in touched_blocks {
            if let Ok(block) = terrain.get(block_pos) {
                let diff2 = block_pos.map(|b| b as f32).distance_squared(pos);
                let fade = (1.0 - diff2 / color_range.powi(2)).max(0.0);
                if let Some(mut color) = block.get_color() {
                    let r = color[0] as f32 + (fade * (color[0] as f32 * 0.5 - color[0] as f32));
                    let g = color[1] as f32 + (fade * (color[1] as f32 * 0.3 - color[1] as f32));
                    let b = color[2] as f32 + (fade * (color[2] as f32 * 0.3 - color[2] as f32));
                    color[0] = r as u8;
                    color[1] = g as u8;
                    color[2] = b as u8;
                    block_change.set(block_pos, Block::new(block.kind(), color));
                }
            }
        }

        // Destroy terrain
        for _ in 0..RAYS {
            let dir = Vec3::new(
                rand::random::<f32>() - 0.5,
                rand::random::<f32>() - 0.5,
                rand::random::<f32>() - 0.15,
            )
            .normalized();

            let terrain = ecs.read_resource::<TerrainGrid>();
            let _ = terrain
                .ray(pos, pos + dir * power)
                // TODO: Faster RNG
                .until(|block| block.is_liquid() || rand::random::<f32>() < 0.05)
                .for_each(|block: &Block, pos| {
                    if block.is_explodable() {
                        exploded_blocks.push(pos, block.clone());
                        block_change.set(pos, block.into_vacant());
                    }
                })
                .cast();
        }
    }

    for (pos, block) in &exploded_blocks {
        match rand::thread_rng().gen_range(0, 9) {
            0 => {
                match block.kind() {
                    BlockKind::Leaves => {
                        let body = match rand::thread_rng().gen_range(0, 2) {
                            0 => object::Body::Twigs0,
                            1 => object::Body::Twigs1,
                            2 => object::Body::Twigs2,
                            _ => object::Body::Twigs0,
                        };
        
                        // TODO: use create_object function instead.
                        ecs
                        .create_entity_synced()
                        .with(comp::Pos(pos.map(|x| x as f32)))
                        .with(comp::Vel(Vec3::zero()))
                        .with(
                            comp::Ori(
                                Dir::from_unnormalized(
                                    Vec3::new(
                                        rand::thread_rng().gen_range(-1.0, 1.0),
                                        rand::thread_rng().gen_range(-1.0, 1.0),
                                        0.0,
                                    )
                                ).unwrap_or_default()
                            )
                        )
                        .with(comp::Mass(5.0))
                        .with(comp::Collider::Box {
                            radius: comp::Body::Object(body).radius(),
                            z_min: 0.0,
                            z_max: comp::Body::Object(body).height(),
                        })
                        .with(comp::Body::Object(body))
                        .with(comp::Gravity(1.0))
                        .with(Item::new_from_asset_expect("common.items.crafting_ing.twigs"));
                    },
                    BlockKind::WeakRock => {
                        let body = object::Body::Rock0;
        
                        // TODO: use create_object function instead.
                        ecs
                        .create_entity_synced()
                        .with(comp::Pos(pos.map(|x| x as f32)))
                        .with(comp::Vel(Vec3::new()))
                        .with(
                            comp::Ori(
                                Dir::from_unnormalized(
                                    Vec3::new(
                                        rand::thread_rng().gen_range(-1.0, 1.0),
                                        rand::thread_rng().gen_range(-1.0, 1.0),
                                        0.0,
                                    )
                                ).unwrap_or_default()
                            )
                        )
                        .with(comp::Mass(5.0))
                        .with(comp::Collider::Box {
                            radius: comp::Body::Object(body).radius(),
                            z_min: 0.0,
                            z_max: comp::Body::Object(body).height(),
                        })
                        .with(comp::Body::Object(body))
                        .with(comp::Gravity(1.0))
                        .with(Item::new_from_asset_expect("common.items.crafting_ing.stones"));
                    },
                    _ => {},
                };
            },
            _ => {},
        }
    }

    // Add an outcome
    ecs.write_resource::<Vec<Outcome>>()
        .push(Outcome::Explosion {
            pos,
            power,
            reagent,
            percent_damage,
            exploded_blocks,
        });
}

pub fn handle_level_up(server: &mut Server, entity: EcsEntity, new_level: u32) {
    let uids = server.state.ecs().read_storage::<Uid>();
    let uid = uids
        .get(entity)
        .expect("Failed to fetch uid component for entity.");

    // Add an outcome
    server
        .state
        .ecs()
        .write_resource::<Vec<Outcome>>()
        .push(Outcome::LevelUp {
            uid: uid.0,
            level: new_level,
        });
    // TODO: determine which event system we should use.
    server
        .state
        .notify_registered_clients(ServerMsg::PlayerListUpdate(PlayerListUpdate::LevelChange(
            *uid, new_level,
        )));
}
