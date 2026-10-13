//! Port of source/app/probability.h + source/app/probability.cpp
//!
//! Programmer inputs his probabilities, then calls `get_value` which returns one of
//! the possible inputs based on the programmed percentages.

#![allow(unused_variables, unused_imports, unused_mut, dead_code)]

use crate::app::miscutils::{rand_i32, RAND_MAX};
use crate::tosser::DArray;

#[derive(Debug, Clone)]
pub struct Probability {
    pub chances: DArray<i32>,
}

impl Default for Probability {
    fn default() -> Self {
        Self::new()
    }
}

impl Probability {
    pub fn new() -> Self {
        // Initialise the first value to 0, just in case we don't insert a zero value
        let mut chances = DArray::new();
        chances.put_data(0);
        Probability { chances }
    }

    pub fn input_probability(&mut self, value: i32, percentagechance: i32) {
        if value >= self.chances.size() {
            self.chances.set_size(value + 1);
        }
        self.chances.put_data_at(percentagechance, value);
    }

    pub fn change_probability(&mut self, value: i32, percentagechance: i32) {
        if self.chances.valid_index(value) {
            self.chances.change_data(percentagechance, value);
        } else {
            println!("WARNING: Probability::ChangeProbability, invalid value passed");
        }
    }

    /// True if all percentages sum to 100.
    pub fn validate(&self) -> bool {
        let mut sum = 0;
        for i in 0..self.chances.size() {
            match self.chances.get_data(i) {
                Some(&value) if (0..=100).contains(&value) => sum += value,
                _ => return false,
            }
        }
        sum == 100
    }

    pub fn get_value(&self) -> i32 {
        let r = (100.0 * (rand_i32() as f32 / RAND_MAX as f32)) as i32;
        let mut result = 0;
        let mut totalscore = 0;
        for i in 0..self.chances.size() {
            let c = self.chances.get_data(i).copied().unwrap_or(0);
            if r >= totalscore && r < totalscore + c {
                result = i;
                break;
            }
            totalscore += c;
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_wants_percentages_that_sum_to_100() {
        let mut p = Probability::new();
        assert!(!p.validate());
        p.input_probability(0, 30);
        assert!(!p.validate());
        p.input_probability(1, 70);
        assert!(p.validate());
        p.change_probability(1, 60);
        assert!(!p.validate());
        p.change_probability(1, 71);
        assert!(!p.validate());
        // out-of-range percentages are rejected even when the sum is right
        p.change_probability(0, 150);
        p.change_probability(1, -50);
        assert!(!p.validate());
    }

    #[test]
    fn a_certain_outcome_is_always_returned() {
        let mut p = Probability::new();
        p.input_probability(0, 0);
        p.input_probability(1, 0);
        p.input_probability(2, 100);
        p.input_probability(3, 0);
        assert!(p.validate());
        for _ in 0..200 {
            assert_eq!(p.get_value(), 2);
        }
    }

    #[test]
    fn outcomes_follow_the_programmed_split() {
        let mut p = Probability::new();
        p.input_probability(0, 50);
        p.input_probability(1, 50);
        let ones = (0..4000).filter(|_| p.get_value() == 1).count();
        // 4000 fair coin flips: far outside any plausible fluctuation (sigma is about 32)
        assert!((1600..=2400).contains(&ones), "{ones}");
    }
}
