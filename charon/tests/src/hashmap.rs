//! A hashmap implementation.
//! TODO: we will need function pointers/closures if we want to make the map
//! generic in the key type.
#![allow(dead_code)]

use std::vec::Vec;
pub type Key = usize; // TODO: make this generic
pub type Hash = usize;

pub enum List<T> {
    Cons(Key, T, Box<List<T>>),
    Nil,
}

/// A hash function for the keys.
/// Rk.: we use shared references because we anticipate on the generic
/// hash map version.
pub fn hash_key(k: &Key) -> Hash {
    // Do nothing for now, we might want to implement something smarter
    // in the future
    *k
}

/// A hash map from [u64] to values
pub struct HashMap<T> {
    /// The current number of values in the table
    num_values: usize,
    /// The max load factor, expressed as a fraction
    max_load_factor: (usize, usize),
    /// The max load factor applied to the current table length:
    /// gives the threshold at which to resize the table.
    max_load: usize,
    /// The table itself
    slots: Vec<List<T>>,
}

impl<T> HashMap<T> {
    /// Allocate a vector of slots of a given size.
    /// We would need a loop, but can't use loops for now...
    fn allocate_slots(mut slots: Vec<List<T>>, n: usize) -> Vec<List<T>> {
        if n == 0 {
            slots
        } else {
            slots.push(List::Nil);
            HashMap::allocate_slots(slots, n - 1)
        }
    }

    /// Create a new table, with a given capacity
    fn new_with_capacity(
        capacity: usize,
        max_load_dividend: usize,
        max_load_divisor: usize,
    ) -> Self {
        // TODO: better to use `Vec::with_capacity(32)` instead
        // of `Vec::new()`
        let slots = HashMap::allocate_slots(Vec::new(), capacity);
        HashMap {
            num_values: 0,
            max_load_factor: (max_load_dividend, max_load_divisor),
            max_load: (capacity * max_load_dividend) / max_load_divisor,
            slots,
        }
    }

    pub fn new() -> Self {
        // For now we create a table with 32 slots and a max load factor of 4/5
        HashMap::new_with_capacity(32, 4, 5)
    }

    /// TODO: we need a loop
    fn clear_slots(slots: &mut Vec<List<T>>, i: usize) {
        if i < slots.len() {
            slots[i] = List::Nil;
            HashMap::clear_slots(slots, i + 1)
        } else {
            ()
        }
    }

    pub fn clear(&mut self) {
        self.num_values = 0;
        HashMap::clear_slots(&mut self.slots, 0);
    }

    pub fn len(&self) -> usize {
        self.num_values
    }

    /// Insert in a list.
    /// Return `true` if we inserted an element, `false` if we simply updated
    /// a value.
    fn insert_in_list<'a>(key: Key, value: T, ls: &'a mut List<T>) -> bool {
        match ls {
            List::Nil => {
                *ls = List::Cons(key, value, Box::new(List::Nil));
                true
            }
            List::Cons(ckey, cvalue, ls) => {
                if *ckey == key {
                    *cvalue = value;
                    false
                } else {
                    HashMap::insert_in_list(key, value, ls)
                }
            }
        }
    }

    /// Auxiliary function to insert in the hashmap without triggering a resize
    fn insert_no_resize<'a>(&'a mut self, key: Key, value: T) {
        let hash = hash_key(&key);
        let hash_mod = hash % self.slots.len();
        // We may want to use slots[...] instead of get_mut...
        let inserted = HashMap::insert_in_list(key, value, &mut self.slots[hash_mod]);
        if inserted {
            self.num_values += 1;
        }
    }

    /// Insertion function.
    /// May trigger a resize of the hash table.
    pub fn insert<'a>(&'a mut self, key: Key, value: T) {
        // Insert
        self.insert_no_resize(key, value);
        // Resize if necessary
        if self.len() >= self.max_load {
            self.try_resize()
        }
    }

    /// The resize function, called if we need to resize the table after
    /// an insertion.
    fn try_resize<'a>(&'a mut self) {
        // Check that we can resize - note that we are conservative about
        // the upper bound. Also note that `as usize` is a trait, but we
        // apply it to a constant here, which gets compiled by the MIR
        // interpreter (so we don't see the conversion, actually).
        let max_usize = u32::MAX as usize;
        if self.slots.len() <= max_usize / 2 {
            // Create a new table with a higher capacity
            let mut ntable = HashMap::new_with_capacity(
                self.slots.len() * 2,
                self.max_load_factor.0,
                self.max_load_factor.1,
            );

            // Move the elements to the new table
            HashMap::move_elements(&mut ntable, &mut self.slots, 0);

            // Replace the current table with the new table
            let _ = std::mem::replace(&mut self.slots, ntable.slots);
            self.max_load = ntable.max_load;
        }
    }

    /// Auxiliary function called by [try_resize] to move all the elements
    /// from the table to a new table
    fn move_elements<'a>(ntable: &'a mut HashMap<T>, slots: &'a mut Vec<List<T>>, i: usize) {
        if i < slots.len() {
            // Move the elements out of the slot i
            let ls = std::mem::replace(&mut slots[i], List::Nil);
            // Move all those elements to the new table
            HashMap::move_elements_from_list(ntable, ls);
            // Do the same for slot i+1
            HashMap::move_elements(ntable, slots, i + 1);
        }
    }

    /// Auxiliary function.
    /// TODO: better with a loop
    fn move_elements_from_list<'a>(ntable: &'a mut HashMap<T>, ls: List<T>) {
        // As long as there are elements in the list, move them
        match ls {
            List::Nil => (), // We're done
            List::Cons(k, v, tl) => {
                // Insert the element in the new table
                ntable.insert_no_resize(k, v);
                // Move the elements out of the tail
                HashMap::move_elements_from_list(ntable, *tl);
            }
        }
    }

    /// We don't support borrows inside of enumerations for now, so we
    /// can't return an option...
    /// TODO: add support for that
    fn get_in_list<'a, 'k>(key: &'k Key, ls: &'a List<T>) -> &'a T {
        match ls {
            List::Nil => panic!(),
            List::Cons(ckey, cvalue, ls) => {
                if *ckey == *key {
                    cvalue
                } else {
                    HashMap::get_in_list(key, ls)
                }
            }
        }
    }

    pub fn get<'a, 'k>(&'a self, key: &'k Key) -> &'a T {
        let hash = hash_key(key);
        let hash_mod = hash % self.slots.len();
        HashMap::get_in_list(key, &self.slots[hash_mod])
    }

    /// Same remark as for [get_in_list]
    fn get_mut_in_list<'a, 'k>(key: &'k Key, ls: &'a mut List<T>) -> &'a mut T {
        match ls {
            List::Nil => panic!(),
            List::Cons(ckey, cvalue, ls) => {
                if *ckey == *key {
                    cvalue
                } else {
                    HashMap::get_mut_in_list(key, ls)
                }
            }
        }
    }

    /// Same remark as for [get].
    pub fn get_mut<'a, 'k>(&'a mut self, key: &'k Key) -> &'a mut T {
        let hash = hash_key(key);
        let hash_mod = hash % self.slots.len();
        HashMap::get_mut_in_list(key, &mut self.slots[hash_mod])
    }

    /// Remove an element from the list.
    /// Return the removed element.
    fn remove_from_list<'a>(key: &Key, ls: &'a mut List<T>) -> Option<T> {
        match ls {
            List::Nil => None,
            List::Cons(ckey, _, tl) => {
                if *ckey == *key {
                    // We have to move under borrows, so we need to use
                    // [std::mem::replace] in several steps.
                    // Retrieve the tail
                    let mv_ls = std::mem::replace(ls, List::Nil);
                    match mv_ls {
                        List::Nil => unreachable!(),
                        List::Cons(_, cvalue, tl) => {
                            // Make the list equal to its tail
                            *ls = *tl;
                            // Returned the dropped value
                            Some(cvalue)
                        }
                    }
                } else {
                    HashMap::remove_from_list(key, tl)
                }
            }
        }
    }

    /// Same remark as for [get].
    pub fn remove<'a>(&'a mut self, key: &Key) -> Option<T> {
        let hash = hash_key(key);
        let hash_mod = hash % self.slots.len();
        let x = HashMap::remove_from_list(key, &mut self.slots[hash_mod]);
        match x {
            Option::None => Option::None,
            Option::Some(x) => {
                self.num_values -= 1;
                Option::Some(x)
            }
        }
    }
}

fn test1() {
    let mut hm: HashMap<u64> = HashMap::new();
    hm.insert(0, 42);
    hm.insert(128, 18);
    hm.insert(1024, 138);
    hm.insert(1056, 256);
    // Rk.: `&128` introduces a ref constant value
    // TODO: add support for this
    let k = 128;
    assert!(*hm.get(&k) == 18);
    let k = 1024;
    let x = hm.get_mut(&k);
    *x = 56;
    assert!(*hm.get(&k) == 56);
    let x = hm.remove(&k);
    // If we write `x == Option::Some(56)` rust introduces
    // a call to `core::cmp::PartialEq::eq`, which is a trait
    // I don't support for now.
    // Also, I haven't implemented support for `unwrap` yet...
    match x {
        Option::None => panic!(),
        Option::Some(x) => assert!(x == 56),
    };
    let k = 0;
    assert!(*hm.get(&k) == 42);
    let k = 128;
    assert!(*hm.get(&k) == 18);
    let k = 1056;
    assert!(*hm.get(&k) == 256);
}

/// It is a bit stupid, but I can't retrieve functions marked as "tests",
/// while I want to extract the unit tests.
#[test]
fn tests() {
    test1();
}
