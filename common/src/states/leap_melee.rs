use crate::{
    comp::{Attacking, CharacterState, StateUpdate},
    states::utils::*,
    sys::character_behavior::*,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use vek::Vec3;

//const LEAP_SPEED: f32 = 24.0;

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Data {
    /// How long the state is moving
    pub movement_duration: Duration,
    /// How long until state should deal damage
    pub buildup_duration: Duration,
    /// How long the state has until exiting
    pub recover_duration: Duration,
    /// How fast the leap should go
    pub leap_speed: f32,
    /// How fast the leap should go up
    pub leap_vert_speed: f32,
    /// Base damage
    pub base_damage: u32,
    /// How far back the attack should knock targets
    pub knockback: f32,
    /// How large the range of attack is
    pub range: f32,
    /// Whether the attack can deal more damage
    pub exhausted: bool,
    pub initialize: bool,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        if self.initialize {
            update.vel.0 = *data.inputs.look_dir * 20.0;
            if let Some(dir) = Vec3::from(data.inputs.look_dir.xy()).try_normalized() {
                update.ori.0 = dir.into();
            }
        }

        if self.movement_duration != Duration::default() {
            // Jumping
            update.vel.0 = Vec3::new(
                data.inputs.look_dir.x,
                data.inputs.look_dir.y,
                self.leap_vert_speed,
            ) * ((self.movement_duration.as_millis() as f32) / 250.0)
                + (update.vel.0 * Vec3::new(2.0, 2.0, 0.0)
                    + 0.25 * data.inputs.move_dir.try_normalized().unwrap_or_default())
                .try_normalized()
                .unwrap_or_default()
                    * self.leap_speed
                    * (1.0 - data.inputs.look_dir.z.abs());

            update.character = CharacterState::LeapMelee(Data {
                movement_duration: self
                    .movement_duration
                    .checked_sub(Duration::from_secs_f32(data.dt.0))
                    .unwrap_or_default(),
                buildup_duration: self.buildup_duration,
                recover_duration: self.recover_duration,
                leap_speed: self.leap_speed,
                leap_vert_speed: self.leap_vert_speed,
                base_damage: self.base_damage,
                knockback: self.knockback,
                range: self.range,
                exhausted: false,
                initialize: false,
            });
        } else if self.buildup_duration != Duration::default() && !data.physics.on_ground {
            // Falling
            update.character = CharacterState::LeapMelee(Data {
                movement_duration: Duration::default(),
                buildup_duration: self
                    .buildup_duration
                    .checked_sub(Duration::from_secs_f32(data.dt.0))
                    .unwrap_or_default(),
                recover_duration: self.recover_duration,
                leap_speed: self.leap_speed,
                leap_vert_speed: self.leap_vert_speed,
                base_damage: self.base_damage,
                knockback: self.knockback,
                range: self.range,
                exhausted: false,
                initialize: false,
            });
        } else if !self.exhausted {
            // Hit attempt
            data.updater.insert(data.entity, Attacking {
                base_healthchange: -(self.base_damage as i32),
                range: self.range,
                max_angle: 360_f32.to_radians(),
                applied: false,
                hit_count: 0,
                knockback: self.knockback,
            });

            update.character = CharacterState::LeapMelee(Data {
                movement_duration: self.movement_duration,
                buildup_duration: Duration::default(),
                recover_duration: self.recover_duration,
                leap_speed: self.leap_speed,
                leap_vert_speed: self.leap_vert_speed,
                base_damage: self.base_damage,
                knockback: self.knockback,
                range: self.range,
                exhausted: true,
                initialize: false,
            });
        } else if self.recover_duration != Duration::default() {
            // Recovery
            handle_move(data, &mut update, 0.7);
            update.character = CharacterState::LeapMelee(Data {
                movement_duration: self.movement_duration,
                buildup_duration: self.buildup_duration,
                recover_duration: self
                    .recover_duration
                    .checked_sub(Duration::from_secs_f32(data.dt.0))
                    .unwrap_or_default(),
                leap_speed: self.leap_speed,
                leap_vert_speed: self.leap_vert_speed,
                base_damage: self.base_damage,
                knockback: self.knockback,
                range: self.range,
                exhausted: true,
                initialize: false,
            });
        } else {
            // Done
            update.character = CharacterState::Wielding;
            // Make sure attack component is removed
            data.updater.remove::<Attacking>(data.entity);
        }

        update
    }
}
