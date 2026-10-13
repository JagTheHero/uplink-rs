//! Port of source/world/generator/numbergenerator.h + source/world/generator/numbergenerator.cpp

use crate::app::miscutils::{rand_i32, RAND_MAX};

// C: `(float) rand () / (float) RAND_MAX`, a single-precision value in [0, 1].
fn rand_unit() -> f32 {
    rand_i32() as f32 / RAND_MAX as f32
}

pub struct NumberGenerator;

impl NumberGenerator {
    pub fn initialise() {}

    /// result ~ U ( 0, range ),      0 <= result < range
    pub fn random_number(range: i32) -> i32 {
        // C: int result = ( (float) rand () / (float) RAND_MAX ) * range;   (float product, truncated)
        let mut result = (rand_unit() * range as f32) as i32;
        if result < 0 {
            result = 0;
        }
        if result >= range {
            result = range - 1;
        }
        result
    }

    /// result ~ N ( mean, range/3 ), mean - range < result < mean + range
    pub fn random_normal_number(mean: f32, range: f32) -> f32 {
        // result ~ N ( mean, range/3 )
        // C: sums 12 float uniforms, then `s = (s-6.0) * (range/3.0) + mean;` (double arithmetic,
        // stored in a float).
        let mut s: f32 = 0.0;
        for _ in 0..12 {
            s += rand_unit();
        }
        s = ((s as f64 - 6.0) * (range as f64 / 3.0) + mean as f64) as f32;

        if s < mean - range {
            s = mean - range;
        }
        if s > mean + range {
            s = mean + range;
        }
        s
    }

    pub fn random_uniform_number() -> f32 {
        rand_unit()
    }

    /// Applies +-percentage variance to num
    pub fn apply_variance(num: i32, variance: i32) -> i32 {
        // C: float variancefactor = 100.0 + RandomNormalNumber ( 0, variance );  (stored as float)
        let variancefactor = (100.0 + Self::random_normal_number(0.0, variance as f32) as f64) as f32;
        // C: num *= (variancefactor / 100.0);  (double product, truncated to int)
        (num as f64 * (variancefactor as f64 / 100.0)) as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_number_is_below_the_range() {
        for range in [1, 2, 5, 100, 1_000_000] {
            for _ in 0..500 {
                let n = NumberGenerator::random_number(range);
                assert!((0..range).contains(&n), "range {range} gave {n}");
            }
        }
        assert_eq!(NumberGenerator::random_number(1), 0);
    }

    #[test]
    fn random_number_covers_the_whole_range() {
        let mut seen = [false; 6];
        for _ in 0..2000 {
            seen[NumberGenerator::random_number(6) as usize] = true;
        }
        assert!(seen.iter().all(|&s| s));
    }

    #[test]
    fn normal_numbers_stay_within_mean_plus_minus_range() {
        for _ in 0..2000 {
            let n = NumberGenerator::random_normal_number(50.0, 10.0);
            assert!((40.0..=60.0).contains(&n), "{n}");
        }
        let mean: f32 = (0..4000).map(|_| NumberGenerator::random_normal_number(0.0, 30.0)).sum::<f32>() / 4000.0;
        assert!(mean.abs() < 2.0, "{mean}");
    }

    #[test]
    fn uniform_number_is_a_unit_fraction() {
        for _ in 0..1000 {
            let n = NumberGenerator::random_uniform_number();
            assert!((0.0..=1.0).contains(&n));
        }
    }

    #[test]
    fn variance_stays_near_the_original_number() {
        for _ in 0..1000 {
            // +-10 percent at most (the normal number is clamped to its range)
            let n = NumberGenerator::apply_variance(1000, 10);
            assert!((899..=1101).contains(&n), "{n}");
        }
        assert_eq!(NumberGenerator::apply_variance(0, 50), 0);
        assert_eq!(NumberGenerator::apply_variance(777, 0), 777);
    }
}
