// Modified version of https://github.com/AFLplusplus/lain/blob/main/lain/src/mutator.rs

use crate::dangerous_numbers::DangerousNumber;
use num::{Bounded, NumCast};
use num_traits::{WrappingAdd, WrappingSub};
use rand::Rng;
use rand::distr::StandardUniform;
use rand::prelude::Distribution;
use rand::prelude::SliceRandom;
use std::collections::HashMap;
use std::ops::{Add, BitXor, Sub};

#[derive(Debug)]
/// The mutator is initialized in the fuzzing engine with an deterministic RNG. When the engine
/// calls the `mutate` function from the `Mutable` trait this is given as parameter to the
/// function. When mutating this mutator should be used. It can also be used to generate a chance
/// which can be used to decide if a field should be mutated in the current mutation.
///
/// To see a example usage please take a look into [profuzz_common/mutable/pnet](https://github.com/otsmr/profuzz/blob/main/profuzz_common/src/mutable/pnet.rs).
pub struct Mutator<R: Rng> {
    rng: R,
    chances: Vec<bool>,
    /// Stored indexes are used to cache the last index
    /// so the returned index is not always chanced
    stored_indexes: HashMap<&'static str, usize>,
    fake_rng: bool,
}

#[repr(u8)]
#[derive(Debug, Copy, Clone)]
enum MutatorOperation {
    BitFlip,
    Flip,
    Arithmetic,
}

// Which direction to weigh ranges towards (min bound, upper bound, or none).
// #[derive(Debug, PartialEq, Clone, Copy, Default)]
// pub enum Weighted {
//     #[default]
//     None,
//     Min,
//     Max,
// }

// Implement the Distribution trait for MutatorOperation
impl Distribution<MutatorOperation> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> MutatorOperation {
        match rng.random_range(0..3) {
            // There are 3 variants
            0 => MutatorOperation::BitFlip,
            1 => MutatorOperation::Flip,
            2 => MutatorOperation::Arithmetic,
            _ => unreachable!(), // This should never happen
        }
    }
}

impl<R: Rng> Mutator<R> {
    /// creates a new Mutator
    pub fn new(rng: R) -> Mutator<R> {
        Mutator {
            rng,
            chances: vec![],
            stored_indexes: HashMap::new(),
            fake_rng: false,
        }
    }

    /// Mutates a number after randomly selecting a mutation strategy (see `MutatorOperation` for a list of strategies)
    /// If a min/max is specified then a new number in this range is chosen instead of performing
    /// a bit/arithmetic mutation
    pub fn mutate<T>(&mut self, num: &mut T)
    where
        T: BitXor<Output = T>
            + Add<Output = T>
            + Sub<Output = T>
            + NumCast
            + Bounded
            + Copy
            + WrappingAdd<Output = T>
            + WrappingSub<Output = T>
            + DangerousNumber<T>
            + std::fmt::Debug,
    {
        #[allow(clippy::eq_op)]
        if self.fake_rng {
            // this will set the value to 0
            *num = (*num) ^ (*num);
            return;
        }

        if self.gen_chance(0.001) {
            *num = T::select_dangerous_number(&mut self.rng);
            return;
        }

        let operation: MutatorOperation = self.rng.random();

        match operation {
            MutatorOperation::BitFlip => self.bit_flip(num),
            MutatorOperation::Flip => self.flip(num),
            MutatorOperation::Arithmetic => self.arithmetic(num),
        }
    }

    /// Flip a single bit in the given number.
    fn bit_flip<T>(&mut self, num: &mut T)
    where
        T: BitXor<Output = T> + Add<Output = T> + Sub<Output = T> + NumCast + Copy,
    {
        #[allow(clippy::cast_possible_truncation)]
        let num_bits = (std::mem::size_of::<T>() * 8) as u8;
        let idx: u8 = self.rng.random_range(0..num_bits);

        if let Some(cast) = num::cast(1u64 << idx) {
            *num = (*num) ^ cast;
        }
    }

    /// Flip more than 1 bit in this number. This is a flip potentially up to
    /// the max bits in the number
    fn flip<T>(&mut self, num: &mut T)
    where
        T: BitXor<Output = T> + Add<Output = T> + Sub<Output = T> + NumCast + Copy,
    {
        #[allow(clippy::cast_possible_truncation)]
        let num_bits = (std::mem::size_of::<T>() * 8) as u8;
        // let bits_to_flip = self.rng.random_range(1..=num_bits) as usize;

        // 64 is chosen here as it's the the max primitive size (in bits) that we support
        // we choose to do this approach over a vec to avoid an allocation
        assert!(num_bits <= 64);
        let mut potential_bit_indices = [0u8; 64];
        for i in 0..num_bits {
            potential_bit_indices[i as usize] = i;
        }

        // debug!("flipping {bits_to_flip} bits");
        let (bit_indices, _) = potential_bit_indices[0..num_bits as usize]
            .partial_shuffle(&mut self.rng, num_bits as usize);

        for idx in bit_indices {
            if let Some(cast) = num::cast(1u64 << *idx) {
                *num = (*num) ^ cast;
            }
        }
    }

    /// Perform a simple arithmetic operation on the number (+ or -)
    fn arithmetic<T>(&mut self, num: &mut T)
    where
        T: Add<Output = T>
            + Sub<Output = T>
            + NumCast
            + Copy
            + WrappingAdd<Output = T>
            + WrappingSub<Output = T>,
    {
        let added_num: i64 = self.rng.random_range(1..=0x10);

        if self.rng.random::<bool>() {
            // debug!("adding {added_num}");
            if let Some(cast) = num::cast(added_num) {
                *num = num.wrapping_add(&cast);
            }
        } else {
            // debug!("subtracting {added_num}");
            if let Some(cast) = num::cast(added_num) {
                *num = num.wrapping_sub(&cast);
            }
        }
    }

    /// Returns a value between min and max
    pub fn gen_range(&mut self, min: usize, max: usize) -> usize {
        self.rng.random_range(min..max)
    }

    /// Can be used to select a random element from a given Vec
    pub fn gen_index(&mut self, key: &'static str, max: usize) -> usize {
        if self.stored_indexes.contains_key(key) {
            let change = self.gen_chance(0.4);
            if let Some(index) = self.stored_indexes.get_mut(key) {
                if change {
                    *index = self.rng.random_range(0..max);
                }
                return *index;
            }
        }
        let index = self.rng.random_range(0..max);
        self.stored_indexes.insert(key, index);
        index
    }

    /// Returns a boolean value indicating whether or not the chance event occurred
    pub fn gen_chance(&mut self, chance_percentage: f64) -> bool {
        let chance = {
            if chance_percentage <= 0.0 {
                false
            } else if chance_percentage >= 1.0 {
                true
            } else {
                self.rng.random_bool(chance_percentage)
            }
        };
        self.chances.push(chance);
        chance
    }
}
