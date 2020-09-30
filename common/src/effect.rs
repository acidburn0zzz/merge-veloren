use crate::comp;
use serde::{Deserialize, Serialize};

/// An effect that may be applied to an entity
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Effect {
    Health(comp::HealthChange),
    InventorySlotIncrease { tier: u8, slots: u16 },
    Xp(i64),
}

impl Effect {
    pub fn info(&self) -> String {
        match self {
            Effect::Health(c) => format!("{:+} health", c.amount),
            Effect::Xp(n) => format!("{:+} exp", n),
            Effect::InventorySlotIncrease { tier, slots } => {
                format!("{:+} additional inventory slots in tier {:+}", slots, tier)
            },
        }
    }
}
