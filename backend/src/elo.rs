//! Elo maths for head-to-head voting.
//!
//! Both tracks start at 1000. K decays from 40 towards a floor of 16 as the
//! less-experienced track accumulates comparisons, so early votes move the
//! standings and later picks settle it.

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

    #[test]
    fn k_factor_uses_the_less_experienced_track() {
        assert!(close(k_factor(0, 100), 40.0));
        assert!(close(k_factor(100, 0), 40.0));
        assert!(close(k_factor(10, 10), 35.0));
        assert!(close(k_factor(10, 90), 35.0));
        assert!(close(k_factor(90, 10), 35.0));
    }

    #[test]
    fn k_factor_treats_negative_comparisons_as_zero() {
        assert!(close(k_factor(-1, 0), 40.0));
        assert!(close(k_factor(-100, -100), 40.0));
    }

    #[test]
    fn k_factor_is_monotonic_between_the_floor_and_base() {
        let mut previous = k_factor(0, 0);
        for comparisons in 1..=80 {
            let current = k_factor(comparisons, comparisons);
            assert!(
                current <= previous + 1e-12,
                "K increased at {comparisons}: {previous} -> {current}"
            );
            assert!((MIN_K..=BASE_K).contains(&current));
            previous = current;
        }
    }

    #[test]
    fn expected_score_is_symmetric_around_fifty_percent() {
        for (low, high) in [
            (1000.0, 1000.0),
            (900.0, 1200.0),
            (1500.0, 1000.0),
            (1000.0, 400.0),
        ] {
            let forward = expected_score(low, high);
            let reverse = expected_score(high, low);
            assert!(close(forward + reverse, 1.0));
            assert!((0.0..=1.0).contains(&forward));
        }
    }

    #[test]
    fn expected_score_matches_known_elo_probabilities() {
        assert!(close(expected_score(1200.0, 1000.0), 0.7597469266479578));
        assert!(close(expected_score(1000.0, 1200.0), 0.2402530733520422));
        assert!(close(expected_score(1400.0, 1000.0), 0.9090909090909091));
    }

    #[test]
    fn a_low_win_always_increases_low_and_decreases_high() {
        let outcome = update(900.0, 1200.0, 0, 0, true);
        assert!(outcome.delta_low > 0.0);
        assert!(outcome.delta_high < 0.0);
        assert!(outcome.expected_low < 0.5);
    }

    #[test]
    fn a_high_win_always_decreases_low_and_increases_high() {
        let outcome = update(1200.0, 900.0, 0, 0, false);
        assert!(outcome.delta_low < 0.0);
        assert!(outcome.delta_high > 0.0);
        assert!(outcome.expected_low > 0.5);
    }

    #[test]
    fn the_reported_expected_score_matches_the_delta() {
        let low_won = update(1111.0, 987.0, 4, 9, true);
        assert!(close(
            low_won.delta_low,
            low_won.k * (1.0 - low_won.expected_low)
        ));
        let high_won = update(1111.0, 987.0, 4, 9, false);
        assert!(close(
            high_won.delta_low,
            high_won.k * (0.0 - high_won.expected_low)
        ));
    }

    #[test]
    fn deltas_never_exceed_the_k_factor() {
        for low in [0.0, 500.0, 1000.0, 2000.0, 10_000.0] {
            for high in [0.0, 500.0, 1000.0, 2000.0, 10_000.0] {
                for low_won in [true, false] {
                    let outcome = update(low, high, 0, 0, low_won);
                    assert!(outcome.delta_low.abs() <= outcome.k + 1e-9);
                    assert!(outcome.delta_high.abs() <= outcome.k + 1e-9);
                }
            }
        }
    }

    #[test]
    fn zero_sum_property_holds_across_a_grid_of_inputs() {
        for low in [0.0, 250.0, 1000.0, 1333.0, 3000.0] {
            for high in [0.0, 250.0, 1000.0, 1333.0, 3000.0] {
                for comparisons in [(0, 0), (7, 3), (48, 48), (500, 1)] {
                    let outcome = update(low, high, comparisons.0, comparisons.1, true);
                    assert!(close(outcome.delta_low + outcome.delta_high, 0.0));
                }
            }
        }
    }

    #[test]
    fn initially_equal_tracks_swing_twenty_points_at_full_k() {
        let winner = update(INITIAL_RATING, INITIAL_RATING, 0, 0, true);
        let loser = update(INITIAL_RATING, INITIAL_RATING, 0, 0, false);
        assert!(close(winner.delta_low, 20.0));
        assert!(close(loser.delta_low, -20.0));
    }

    #[test]
    fn settled_tracks_swing_only_eight_points() {
        let outcome = update(1000.0, 1000.0, 48, 48, true);
        assert!(close(outcome.k, MIN_K));
        assert!(close(outcome.delta_low, 8.0));
        assert!(close(outcome.delta_high, -8.0));
    }

    #[test]
    fn extreme_rating_differences_produce_saturating_but_finite_updates() {
        let outcome = update(0.0, 10_000.0, 0, 0, true);
        assert!(outcome.expected_low.is_finite());
        assert!(outcome.delta_low.is_finite());
        assert!(outcome.delta_low >= 0.0);
        assert!(outcome.delta_low <= outcome.k);
    }

    #[test]
    fn update_does_not_modify_the_inputs() {
        let low = 1000.0;
        let high = 1000.0;
        let _ = update(low, high, 0, 0, true);
        assert!(close(low, 1000.0));
        assert!(close(high, 1000.0));
    }
}
