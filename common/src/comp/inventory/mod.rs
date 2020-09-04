pub mod item;
pub mod slot;

use crate::recipe::Recipe;
use item::Item;
use serde::{Deserialize, Serialize};
use specs::{Component, FlaggedStorage, HashMapStorage};
use specs_idvs::IdvStorage;
use std::{convert::TryFrom, ops::Not};

// The limit on distance between the entity and a collectible (squared)
pub const MAX_PICKUP_RANGE_SQR: f32 = 64.0;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Inventory {
    pub slots: Vec<Option<Item>>,
    pub amount: u32,
}

/// Errors which the methods on `Inventory` produce
#[derive(Debug)]
pub enum Error {
    /// The inventory is full and items could not be added. The extra items have
    /// been returned.
    Full(Vec<Item>),
}

#[allow(clippy::len_without_is_empty)] // TODO: Pending review in #587
impl Inventory {
    pub fn new_empty() -> Inventory {
        Inventory {
            slots: vec![None; 36],
            amount: 0,
        }
    }

    pub fn slots(&self) -> &[Option<Item>] { &self.slots }

    pub fn len(&self) -> usize { self.slots.len() }

    pub fn recount_items(&mut self) {
        self.amount = self.slots.iter().filter(|i| i.is_some()).count() as u32;
    }

    /// Adds a new item to the first fitting group of the inventory or starts a
    /// new group. Returns the item again if no space was found.
    pub fn push(&mut self, item: Item) -> Option<Item> {
        if item.inner_item.is_stackable() {
            for slot in &mut self.slots {
                if slot.as_ref().map(|s| s == &item).unwrap_or(false) {
                    let slot_item = slot.as_mut().unwrap(); // TODO: remove unwrap
                    slot_item.increase_amount(u32::from(item.amount()));
                    // self.recount_items(); // Why? Item count hasn't changed.
                    return None;
                }
            }
        }

        // No existing item to stack with or item not stackable, put the item in a new
        // slot
        let item = self.add_to_first_empty(item);
        self.recount_items();
        item
    }

    /// Adds a new item to the first empty slot of the inventory. Returns the
    /// item again if no free slot was found.
    fn add_to_first_empty(&mut self, item: Item) -> Option<Item> {
        let item = match self.slots.iter_mut().find(|slot| slot.is_none()) {
            Some(slot) => {
                *slot = Some(item);
                None
            },
            None => Some(item),
        };
        self.recount_items();
        item
    }

    /// Add a series of items to inventory, returning any which do not fit as an
    /// error.
    pub fn push_all<I: Iterator<Item = Item>>(&mut self, items: I) -> Result<(), Error> {
        // Vec doesn't allocate for zero elements so this should be cheap
        let mut leftovers = Vec::new();
        for item in items {
            if let Some(item) = self.push(item) {
                leftovers.push(item);
            }
        }
        self.recount_items();
        if !leftovers.is_empty() {
            Err(Error::Full(leftovers))
        } else {
            Ok(())
        }
    }

    /// Add a series of items to an inventory without giving duplicates.
    /// (n * m complexity)
    ///
    /// Error if inventory cannot contain the items (is full), returning the
    /// un-added items. This is a lazy inefficient implementation, as it
    /// iterates over the inventory more times than necessary (n^2) and with
    /// the proper structure wouldn't need to iterate at all, but because
    /// this should be fairly cold code, clarity has been favored over
    /// efficiency.
    pub fn push_all_unique<I: Iterator<Item = Item>>(&mut self, mut items: I) -> Result<(), Error> {
        let mut leftovers = Vec::new();
        for item in &mut items {
            if self.contains(&item).not() {
                self.push(item).map(|overflow| leftovers.push(overflow));
            } // else drop item if it was already in
        }
        if !leftovers.is_empty() {
            Err(Error::Full(leftovers))
        } else {
            Ok(())
        }
    }

    /// Replaces an item in a specific slot of the inventory. Returns the old
    /// item or the same item again if that slot was not found.
    pub fn insert(&mut self, cell: usize, item: Item) -> Result<Option<Item>, Item> {
        match self.slots.get_mut(cell) {
            Some(slot) => {
                let old = slot.take();
                *slot = Some(item);
                self.recount_items();
                Ok(old)
            },
            None => Err(item),
        }
    }

    /// Checks if inserting item exists in given cell. Inserts an item if it
    /// exists.
    pub fn insert_or_stack(&mut self, cell: usize, item: Item) -> Result<Option<Item>, Item> {
        if item.inner_item.is_stackable() {
            match self.slots.get_mut(cell) {
                Some(Some(slot_item)) => {
                    if slot_item == &item {
                        slot_item.increase_amount(u32::from(item.amount()));
                        Ok(None)
                    } else {
                        let old_item = std::mem::replace(slot_item, item);
                        self.recount_items();
                        Ok(Some(old_item))
                    }
                },
                Some(None) => self.insert(cell, item),
                None => Err(item),
            }
        } else {
            self.insert(cell, item)
        }
    }

    pub fn is_full(&self) -> bool { self.slots.iter().all(|slot| slot.is_some()) }

    /// O(n) count the number of items in this inventory.
    pub fn count(&self) -> usize { self.slots.iter().filter_map(|slot| slot.as_ref()).count() }

    /// O(n) check if an item is in this inventory.
    pub fn contains(&self, item: &Item) -> bool {
        self.slots.iter().any(|slot| slot.as_ref() == Some(item))
    }

    /// Get content of a slot
    pub fn get(&self, cell: usize) -> Option<&Item> {
        self.slots.get(cell).and_then(Option::as_ref)
    }

    /// Swap the items inside of two slots
    pub fn swap_slots(&mut self, a: usize, b: usize) {
        if a.max(b) < self.slots.len() {
            self.slots.swap(a, b);
        }
    }

    /// Remove an item from the slot
    pub fn remove(&mut self, cell: usize) -> Option<Item> {
        let item = self.slots.get_mut(cell).and_then(|item| item.take());
        self.recount_items();
        item
    }

    /// Remove just one item from the slot
    pub fn take(&mut self, cell: usize) -> Option<Item> {
        if let Some(Some(item)) = self.slots.get_mut(cell) {
            let mut return_item = item.duplicate();

            if item.inner_item.is_stackable() && item.amount() > 1 {
                item.decrease_amount(1);
                return_item.set_amount(1).unwrap(); // TODO: remove unwrap
                self.recount_items();
                Some(return_item)
            } else {
                self.remove(cell)
            }
        } else {
            None
        }
    }

    /// Determine how many of a particular item there is in the inventory.
    pub fn item_count(&self, item: &Item) -> usize {
        self.slots()
            .iter()
            .flatten()
            .filter(|it| it.superficially_eq(item))
            .map(|it| usize::try_from(it.amount()).unwrap()) // TODO: remove unwrap
            .sum()
    }

    /// Determine whether the inventory contains the ingredients for a recipe.
    /// If it does, return a vector of numbers, where is number corresponds
    /// to an inventory slot, along with the number of items that need
    /// removing from it. It items are missing, return the missing items, and
    /// how many are missing.
    pub fn contains_ingredients<'a>(
        &self,
        recipe: &'a Recipe,
    ) -> Result<Vec<usize>, Vec<(&'a Item, usize)>> {
        let mut slot_claims = vec![0; self.slots.len()];
        let mut missing = Vec::new();

        for (input, mut needed) in recipe.inputs() {
            let mut contains_any = false;

            for (i, slot) in self.slots().iter().enumerate() {
                if let Some(item) = slot.as_ref().filter(|item| item.superficially_eq(input)) {
                    let can_claim = (item.amount() as usize - slot_claims[i]).min(needed);
                    slot_claims[i] += can_claim;
                    needed -= can_claim;
                    contains_any = true;
                }
            }

            if needed > 0 || !contains_any {
                missing.push((input, needed));
            }
        }

        if missing.is_empty() {
            Ok(slot_claims)
        } else {
            Err(missing)
        }
    }
}

impl Default for Inventory {
    fn default() -> Inventory {
        let mut inventory = Inventory {
            slots: vec![None; 36],
            amount: 0,
        };
        inventory.push(Item::new_from_asset_expect("common.items.food.cheese"));
        inventory.push(Item::new_from_asset_expect("common.items.food.apple"));
        inventory
    }
}

impl Component for Inventory {
    type Storage = HashMapStorage<Self>;
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum InventoryUpdateEvent {
    Init,
    Used,
    Consumed(String),
    Gave,
    Given,
    Swapped,
    Dropped,
    Collected(Item),
    CollectFailed,
    Possession,
    Debug,
    Craft,
}

impl Default for InventoryUpdateEvent {
    fn default() -> Self { Self::Init }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct InventoryUpdate {
    event: InventoryUpdateEvent,
}

impl InventoryUpdate {
    pub fn new(event: InventoryUpdateEvent) -> Self { Self { event } }

    pub fn event(&self) -> InventoryUpdateEvent { self.event.clone() }
}

impl Component for InventoryUpdate {
    type Storage = FlaggedStorage<Self, IdvStorage<Self>>;
}

#[cfg(test)] mod test;
