use crate::comp;
use crate::comp::HealthChange;
use comp::item::Reagent;
use serde::{Deserialize, Serialize};
use vek::*;

/// An outcome represents the final result of an instantaneous event. It implies
/// that said event has already occurred. It is not a request for that event to
/// occur, nor is it something that may be cancelled or otherwise altered. Its
/// primary purpose is to act as something for frontends (both server and
/// client) to listen to in order to receive feedback about events in the world.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Outcome {
    Explosion {
        pos: Vec3<f32>,
        power: f32,
        reagent: Option<Reagent>, // How can we better define this?
        percent_damage: f32,
    },
    ProjectileShot {
        pos: Vec3<f32>,
        body: comp::Body,
        vel: Vec3<f32>,
    },
    LevelUp {
        uid: u64, //Uid,
        level: u32,
    },
    Damage {
        uid: u64, //Uid,
        change: HealthChange,
    },
}

impl Outcome {
    pub fn get_pos(&self) -> Option<Vec3<f32>> {
        match self {
            Outcome::Explosion { pos, .. } => Some(*pos),
            Outcome::ProjectileShot { pos, .. } => Some(*pos),
            _ => {
                tracing::warn!("get_pos not implemented for Outcome, which is used for avoiding unnecessary network syncs.");
                None
            },
        }
    }
}
