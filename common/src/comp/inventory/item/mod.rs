pub mod armor;
pub mod tool;

// Reexports
pub use tool::{Hands, Tool, ToolCategory, ToolKind};

use crate::{
    assets::{self, Asset, Error},
    effect::Effect,
    lottery::Lottery,
    terrain::{Block, BlockKind},
};
use crossbeam::atomic::AtomicCell;
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use specs::{Component, FlaggedStorage};
use specs_idvs::IdvStorage;
use std::{
    fs::File,
    io::BufReader,
    num::{NonZeroU32, NonZeroU64},
    sync::Arc,
};
use vek::Rgb;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Throwable {
    Bomb,
    TrainingDummy,
    Firework(Reagent),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Reagent {
    Blue,
    Green,
    Purple,
    Red,
    Yellow,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Utility {
    Collar,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Lantern {
    pub kind: String,
    color: Rgb<u32>,
    strength_thousandths: u32,
    flicker_thousandths: u32,
}

impl Lantern {
    pub fn strength(&self) -> f32 { self.strength_thousandths as f32 / 1000_f32 }

    pub fn color(&self) -> Rgb<f32> { self.color.map(|c| c as f32 / 255.0) }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ItemKind {
    /// Something wieldable
    Tool(tool::Tool),
    Lantern(Lantern),
    Armor(armor::Armor),
    Consumable {
        kind: String,
        effect: Effect,
    },
    Throwable {
        kind: Throwable,
    },
    Utility {
        kind: Utility,
    },
    Ingredient {
        kind: String,
    },
}

// TODO: Remove/move to Item
// impl ItemKind {
//     pub fn stack_size(&self) -> Option<u32> {
//         match self {
//             ItemKind::Consumable {
//                 kind: _,
//                 effect: _,
//                 amount,
//             } => Some(*amount),
//             ItemKind::Throwable { kind: _, amount } => Some(*amount),
//             ItemKind::Utility { kind: _, amount } => Some(*amount),
//             ItemKind::Ingredient { kind: _, amount } => Some(*amount),
//             _ => None,
//         }
//     }
// }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Item {
    #[serde(skip)]
    pub item_id: Arc<AtomicCell<Option<NonZeroU64>>>,
    pub inner_item: Arc<InnerItem>,
    amount: NonZeroU32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InnerItem {
    #[serde(skip)]
    item_definition_id: String,
    name: String,
    description: String,
    pub kind: ItemKind,
}

impl InnerItem {
    pub fn is_stackable(&self) -> bool {
        match self.kind {
            ItemKind::Consumable { .. }
            | ItemKind::Ingredient { .. }
            | ItemKind::Throwable { .. }
            | ItemKind::Utility { .. } => true,
            _ => false,
        }
    }
}

impl PartialEq for Item {
    fn eq(&self, other: &Self) -> bool {
        self.inner_item.item_definition_id == other.inner_item.item_definition_id
    }
}

//pub type ItemAsset = Ron<Item>;

impl Asset for InnerItem {
    const ENDINGS: &'static [&'static str] = &["ron"];

    fn parse(buf_reader: BufReader<File>, specifier: &str) -> Result<Self, assets::Error> {
        let item: Result<Self, Error> =
            ron::de::from_reader(buf_reader).map_err(Error::parse_error);

        item.map(|item| InnerItem {
            item_definition_id: specifier.to_owned(),
            ..item
        })
    }
}

impl Item {
    // TODO: consider alternatives such as default abilities that can be added to a
    // loadout when no weapon is present
    pub fn empty() -> Self { Item::new(InnerItem::load_expect("common.items.weapons.empty.empty")) }

    pub fn new(inner_item: Arc<InnerItem>) -> Self {
        Item {
            item_id: Arc::new(AtomicCell::new(None)),
            inner_item,
            amount: NonZeroU32::new(0).unwrap(),
        }
    }

    /// Creates a new instance of an `Item` from the provided asset identifier
    /// Panics if the asset does not exist.
    pub fn new_from_asset_expect(asset_specifier: &str) -> Self {
        let inner_item = InnerItem::load_expect(asset_specifier);
        Item::new(inner_item)
    }

    /// Creates a Vec containing one of each item that matches the provided
    /// asset glob pattern
    pub fn new_from_asset_glob(asset_glob: &str) -> Result<Vec<Self>, Error> {
        let items = InnerItem::load_glob(asset_glob)?;

        let result = items
            .iter()
            .map(|item| Item::new(item.clone()))
            .collect::<Vec<_>>();

        Ok(result)
    }

    /// Creates a new instance of an `Item from the provided asset identifier if
    /// it exists
    pub fn new_from_asset(asset: &str) -> Result<Self, Error> {
        println!("Loading item asset: {}", asset);
        let inner_item = InnerItem::load(asset)?;
        Ok(Item::new(inner_item))
    }

    /// Duplicates an item, creating an exact copy but with a new item ID
    pub fn duplicate(&self) -> Self { Item::new(self.inner_item.clone()) }

    /// Resets the item's item ID to None, giving it a new identity. Used when
    /// dropping items into the world so that a new database record is
    /// created when they are picked up again.
    ///
    /// NOTE: The creation of a new `Arc` when resetting the item ID is critical
    /// because every time a new `Item` instance is created, it is cloned from
    /// a single asset which results in an `Arc` pointing to the same value in
    /// memory. Therefore, every time an item instance is created this
    /// method must be called in order to give it a unique identity.
    fn reset_item_id(&mut self) {
        if let Some(item_id) = Arc::get_mut(&mut self.item_id) {
            *item_id = AtomicCell::new(None);
        } else {
            self.item_id = Arc::new(AtomicCell::new(None));
        }
    }

    /// Removes the unique identity of an item - used when dropping an item on
    /// the floor. In the future this will need to be changed if we want to
    /// maintain a unique ID for an item even when it's dropped and picked
    /// up by another player.
    pub fn put_in_world(&mut self) { self.reset_item_id() }

    // pub fn change_amount_expect(&mut self, modifier: i32) {
    //     let mut current_amount = self.amount as u32;
    //     current_amount += modifier;
    //     self.amount = NonZeroU32::try_from(current_amount).expect("invalid
    // amount"); }

    pub fn increase_amount(&mut self, increase_by: u32) {
        let mut amount = u32::from(self.amount);
        amount += increase_by;
        // TODO make this return Result and prevent modifying amount for non-stackables
        self.amount = NonZeroU32::new(amount).unwrap();
    }

    pub fn decrease_amount(&mut self, decrease_by: u32) {
        let mut amount = u32::from(self.amount);
        amount -= decrease_by;
        // TODO make this return Result and prevent modifying amount for non-stackables
        self.amount = NonZeroU32::new(amount).unwrap(); // TODO make this return Result
    }

    pub fn set_amount(&mut self, give_amount: u32) -> Result<(), assets::Error> {
        if self.inner_item.is_stackable() {
            self.amount = NonZeroU32::new(give_amount).unwrap(); // TODO: remove unwrap
            Ok(())
        } else {
            Err(assets::Error::InvalidType)
        }
    }

    pub fn item_definition_id(&self) -> &str { &self.inner_item.item_definition_id }

    pub fn name(&self) -> &str { &self.inner_item.name }

    pub fn description(&self) -> &str { &self.inner_item.description }

    pub fn kind(&self) -> &ItemKind { &self.inner_item.kind }

    pub fn amount(&self) -> u32 { u32::from(self.amount) }

    pub fn try_reclaim_from_block(block: Block) -> Option<Self> {
        let chosen;
        let mut rng = rand::thread_rng();
        Some(Item::new_from_asset_expect(match block.kind() {
            BlockKind::Apple => "common.items.food.apple",
            BlockKind::Mushroom => "common.items.food.mushroom",
            BlockKind::Velorite => "common.items.ore.velorite",
            BlockKind::VeloriteFrag => "common.items.ore.veloritefrag",
            BlockKind::BlueFlower => "common.items.flowers.blue",
            BlockKind::PinkFlower => "common.items.flowers.pink",
            BlockKind::PurpleFlower => "common.items.flowers.purple",
            BlockKind::RedFlower => "common.items.flowers.red",
            BlockKind::WhiteFlower => "common.items.flowers.white",
            BlockKind::YellowFlower => "common.items.flowers.yellow",
            BlockKind::Sunflower => "common.items.flowers.sun",
            BlockKind::LongGrass => "common.items.grasses.long",
            BlockKind::MediumGrass => "common.items.grasses.medium",
            BlockKind::ShortGrass => "common.items.grasses.short",
            BlockKind::Coconut => "common.items.food.coconut",
            BlockKind::Chest => {
                chosen = Lottery::<String>::load_expect(match rng.gen_range(0, 7) {
                    0 => "common.loot_tables.loot_table_weapon_uncommon",
                    1 => "common.loot_tables.loot_table_weapon_common",
                    2 => "common.loot_tables.loot_table_armor_light",
                    3 => "common.loot_tables.loot_table_armor_cloth",
                    4 => "common.loot_tables.loot_table_armor_heavy",
                    _ => "common.loot_tables.loot_table_armor_misc",
                });
                chosen.choose()
            },
            BlockKind::Crate => {
                chosen = Lottery::<String>::load_expect("common.loot_tables.loot_table_food");
                chosen.choose()
            },
            BlockKind::Stones => "common.items.crafting_ing.stones",
            BlockKind::Twigs => "common.items.crafting_ing.twigs",
            BlockKind::ShinyGem => "common.items.crafting_ing.shiny_gem",
            _ => return None,
        }))
    }

    /// Determines whether two items are superficially equivalent to one another
    /// (i.e: one may be substituted for the other in crafting recipes or
    /// item possession checks).
    pub fn superficially_eq(&self, other: &Self) -> bool {
        match (&self.kind(), &other.kind()) {
            (ItemKind::Tool(a), ItemKind::Tool(b)) => a.superficially_eq(b),
            // TODO: Differentiate between lantern colors?
            (ItemKind::Lantern(_), ItemKind::Lantern(_)) => true,
            (ItemKind::Armor(a), ItemKind::Armor(b)) => a.superficially_eq(b),
            (ItemKind::Consumable { kind: a, .. }, ItemKind::Consumable { kind: b, .. }) => a == b,
            (ItemKind::Throwable { kind: a, .. }, ItemKind::Throwable { kind: b, .. }) => a == b,
            (ItemKind::Utility { kind: a, .. }, ItemKind::Utility { kind: b, .. }) => a == b,
            (ItemKind::Ingredient { kind: a, .. }, ItemKind::Ingredient { kind: b, .. }) => a == b,
            _ => false,
        }
    }
}

impl Component for Item {
    type Storage = FlaggedStorage<Self, IdvStorage<Self>>;
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ItemDrop(pub Item);

impl Component for ItemDrop {
    type Storage = FlaggedStorage<Self, IdvStorage<Self>>;
}
