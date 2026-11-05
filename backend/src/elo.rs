//! Elo maths for head-to-head voting.
//!
//! Both tracks start at 1000. K decays from 40 towards a floor of 16 as the
//! less-experienced track accumulates comparisons, so early votes move the
//! chart and later votes settle it.

/// Both tracks start here; this is also the database default.
pub const INITIAL_RATING: f64 = 1000.0;

const BASE_K: f64 = 40.0;
const MIN_K: f64 = 16.0;
const DECAY_PER_COMPARISON: f64 = 0.5;

#[derive(Debug, Clone, Copy)]
pub struct EloUpdate {
    pub k: f64,
    pub expected_low: f64,
    pub delta_low: f64,
    pub delta_high: f64,
}

pub fn k_factor(comparisons_low: i32, comparisons_high: i32) -> f64 {
    let fewer = comparisons_low.min(comparisons_high).max(0) as f64;
    (BASE_K - fewer * DECAY_PER_COMPARISON).max(MIN_K)
}

pub fn expected_score(rating_low: f64, rating_high: f64) -> f64 {
    1.0 / (1.0 + 10f64.powf((rating_high - rating_low) / 400.0))
}

pub fn update(
    rating_low: f64,
    rating_high: f64,
    comparisons_low: i32,
    comparisons_high: i32,
    low_won: bool,
) -> EloUpdate {
    let k = k_factor(comparisons_low, comparisons_high);
    let expected_low = expected_score(rating_low, rating_high);
    let score_low = if low_won { 1.0 } else { 0.0 };
    let delta_low = k * (score_low - expected_low);

    EloUpdate {
        k,
        expected_low,
        delta_low,
        delta_high: -delta_low,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(left: f64, right: f64) -> bool {
        (left - right).abs() < 1e-9
    }

    #[test]
    fn k_factor_decays_to_a_floor() {
        assert!(close(k_factor(0, 0), 40.0));
        assert!(close(k_factor(20, 0), 40.0));
        assert!(close(k_factor(20, 20), 30.0));
        assert!(close(k_factor(48, 48), 16.0));
        assert!(close(k_factor(100, 100), 16.0));
    }

    #[test]
    fn initial_rating_is_one_thousand() {
        assert!(close(INITIAL_RATING, 1000.0));
    }

    #[test]
    fn equal_ratings_are_a_coin_flip() {
        assert!(close(expected_score(1000.0, 1000.0), 0.5));
        let outcome = update(1000.0, 1000.0, 0, 0, true);
        assert!(close(outcome.delta_low, 20.0));
        assert!(close(outcome.delta_high, -20.0));
        assert!(close(outcome.delta_low + outcome.delta_high, 0.0));
    }

    #[test]
    fn upsets_move_more_than_expected_wins() {
        let upset = update(1000.0, 1200.0, 0, 0, true);
        let expected = update(1000.0, 1200.0, 0, 0, false);
        assert!(upset.delta_low > expected.delta_low.abs());
        assert!(upset.delta_low > 20.0);
    }

    #[test]
    fn deltas_are_zero_sum() {
        for low_won in [true, false] {
            let outcome = update(1123.4, 987.6, 11, 7, low_won);
            assert!(close(outcome.delta_low + outcome.delta_high, 0.0));
        }
    }

    #[test]
    fn favourite_win_moves_less_than_underdog_win() {
        let favourite_wins = update(1200.0, 1000.0, 0, 0, true);
        let underdog_wins = update(1000.0, 1200.0, 0, 0, true);
        assert!(underdog_wins.delta_low > favourite_wins.delta_low);
    }
}
