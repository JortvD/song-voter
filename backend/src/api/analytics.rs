//! Pure maths shared by every admin statistic.
//!
//! Everything in this module is deliberately free of database access so the
//! definitions used by the dashboard (win chance, upset rule, side mapping,
//! sessions, evenness, the settling forecast and the flag rules) can be unit
//! tested directly.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::settings::StatsConfig;

/// The two screen sides. A is left (left arrow / key 1), B is right.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Side {
    A,
    B,
}

impl Side {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::A => "A",
            Self::B => "B",
        }
    }
}

/// Which side the normalised low-id track was shown on.
pub fn side_of_low(served_flip: bool) -> Side {
    if served_flip {
        Side::B
    } else {
        Side::A
    }
}

/// Which side the normalised high-id track was shown on.
pub fn side_of_high(served_flip: bool) -> Side {
    if served_flip {
        Side::A
    } else {
        Side::B
    }
}

/// Map a winner using the same low/high + served_flip rule as the duel screen.
pub fn winner_side(winner_is_low: bool, served_flip: bool) -> Side {
    if winner_is_low {
        side_of_low(served_flip)
    } else {
        side_of_high(served_flip)
    }
}

/// The probability (0-100) that `rating_x` beats `rating_y`.
pub fn win_chance(rating_x: f64, rating_y: f64) -> f64 {
    100.0 / (1.0 + 10f64.powf((rating_y - rating_x) / 400.0))
}

/// An upset is a win by a song whose chance was below the configured cut-off.
pub fn is_upset(winner_win_chance_pct: f64, threshold_pct: f64) -> bool {
    winner_win_chance_pct < threshold_pct
}

/// The rating a track had immediately after a vote.
pub fn rating_after(
    is_low: bool,
    rating_low_before: f64,
    rating_high_before: f64,
    delta_low: f64,
    delta_high: f64,
) -> f64 {
    if is_low {
        rating_low_before + delta_low
    } else {
        rating_high_before + delta_high
    }
}

/// Rank-weight for a 0-based rank. Higher-rated songs carry more weight.
pub fn rank_weight(rank0: i64, strength: f64, scale: f64) -> f64 {
    if scale <= 0.0 {
        return 1.0;
    }
    1.0 + strength * (-(rank0 as f64) / scale).exp()
}

/// 1-based rank bucket used by the pairing-health table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RankBucket {
    Top50,
    Ranks51To100,
    Ranks101To300,
    Ranks301To1000,
    Ranks1001Plus,
}

pub const RANK_BUCKETS: [(RankBucket, i64, i64, &str); 5] = [
    (RankBucket::Top50, 1, 50, "#1–50"),
    (RankBucket::Ranks51To100, 51, 100, "#51–100"),
    (RankBucket::Ranks101To300, 101, 300, "#101–300"),
    (RankBucket::Ranks301To1000, 301, 1000, "#301–1000"),
    (RankBucket::Ranks1001Plus, 1001, i64::MAX, "#1001+"),
];

pub fn rank_bucket(rank: i64) -> RankBucket {
    match rank {
        i64::MIN..=50 => RankBucket::Top50,
        51..=100 => RankBucket::Ranks51To100,
        101..=300 => RankBucket::Ranks101To300,
        301..=1000 => RankBucket::Ranks301To1000,
        _ => RankBucket::Ranks1001Plus,
    }
}

pub fn rank_bucket_index(rank: i64) -> usize {
    RANK_BUCKETS
        .iter()
        .position(|(bucket, _, _, _)| *bucket == rank_bucket(rank))
        .unwrap_or(RANK_BUCKETS.len() - 1)
}

/// Linear-interpolated percentile of an ascending slice, `p` in 0..=1.
pub fn percentile_sorted(sorted: &[f64], p: f64) -> Option<f64> {
    if sorted.is_empty() {
        return None;
    }
    if sorted.len() == 1 {
        return Some(sorted[0]);
    }
    let p = p.clamp(0.0, 1.0);
    let position = p * (sorted.len() - 1) as f64;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    if lower == upper {
        return Some(sorted[lower]);
    }
    let fraction = position - lower as f64;
    Some(sorted[lower] * (1.0 - fraction) + sorted[upper] * fraction)
}

/// Percentile of unsorted values. Returns `None` for an empty sample.
pub fn percentile(values: &[f64], p: f64) -> Option<f64> {
    let mut sorted = values.to_vec();
    sorted.sort_by(|left, right| left.total_cmp(right));
    percentile_sorted(&sorted, p)
}

pub fn median(values: &[f64]) -> Option<f64> {
    percentile(values, 0.5)
}

/// A session is a contiguous index range (start inclusive, end exclusive).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionRange {
    pub start: usize,
    pub end: usize,
}

impl SessionRange {
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }
}

/// Split ascending timestamps wherever the gap exceeds `gap_seconds`.
///
/// A gap exactly equal to the threshold stays inside the session; the setting
/// is described to admins as "longer than this starts a new session".
pub fn split_sessions(times: &[DateTime<Utc>], gap_seconds: i64) -> Vec<SessionRange> {
    if times.is_empty() {
        return Vec::new();
    }

    let mut sessions = Vec::new();
    let mut start = 0usize;
    for index in 1..times.len() {
        let gap = (times[index] - times[index - 1]).num_seconds();
        if gap > gap_seconds {
            sessions.push(SessionRange { start, end: index });
            start = index;
        }
    }
    sessions.push(SessionRange {
        start,
        end: times.len(),
    });
    sessions
}

/// Decision time in seconds, excluding idle gaps and clock-skewed values.
pub fn decision_seconds(
    seen_at: DateTime<Utc>,
    voted_at: DateTime<Utc>,
    gap_seconds: i64,
) -> Option<f64> {
    let seconds = (voted_at - seen_at).num_milliseconds() as f64 / 1000.0;
    if seconds < 0.0 || seconds > gap_seconds as f64 {
        None
    } else {
        Some(seconds)
    }
}

/// Evenness of the comparison distribution: P10 / P90. Closer to 1 is better.
pub fn evenness(ratios: &[f64]) -> Option<f64> {
    let mut sorted = ratios.to_vec();
    sorted.sort_by(|left, right| left.total_cmp(right));
    let p10 = percentile_sorted(&sorted, 0.10)?;
    let p90 = percentile_sorted(&sorted, 0.90)?;
    if p90 <= 0.0 {
        None
    } else {
        Some((p10 / p90).clamp(0.0, 1.0))
    }
}

/// Named thresholds for the settling status. 0.5 and 0.8 split "catching up"
/// into early/late; 0.9 is the asymptote used by the forecast formula.
pub const EVENNESS_EARLY_CUTOFF: f64 = 0.5;
pub const EVENNESS_SETTLED_CUTOFF: f64 = 0.8;
pub const EVENNESS_TARGET: f64 = 0.9;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SettlingStatus {
    NotStarted,
    CatchingUpEarly,
    CatchingUpLate,
    Settled,
}

pub fn settling_status(
    average_comparisons: f64,
    min_avg: f64,
    even_now: Option<f64>,
) -> SettlingStatus {
    if average_comparisons < min_avg {
        return SettlingStatus::NotStarted;
    }
    match even_now {
        Some(value) if value < EVENNESS_EARLY_CUTOFF => SettlingStatus::CatchingUpEarly,
        Some(value) if value < EVENNESS_SETTLED_CUTOFF => SettlingStatus::CatchingUpLate,
        _ => SettlingStatus::Settled,
    }
}

/// Estimated duels left until evenness reaches the 0.9 asymptote.
///
/// `duels_in_window` is the number of duels observed in the lookback window.
/// The formula only makes sense while evenness is still improving below 0.8.
pub fn duels_left(even_now: f64, even_then: f64, duels_in_window: f64) -> Option<f64> {
    if !(even_now < EVENNESS_SETTLED_CUTOFF && even_then < even_now) {
        return None;
    }
    if duels_in_window <= 0.0 {
        return None;
    }
    let remaining_now = EVENNESS_TARGET - even_now;
    let remaining_then = EVENNESS_TARGET - even_then;
    if remaining_now <= 0.0 || remaining_then <= 0.0 || remaining_then <= remaining_now {
        return None;
    }
    let numerator = (0.1 / remaining_now).ln();
    let denominator = (remaining_now / remaining_then).ln();
    if !denominator.is_finite() || denominator.abs() < 1e-12 {
        return None;
    }
    let estimate = duels_in_window * numerator / denominator;
    (estimate.is_finite() && estimate >= 0.0).then_some(estimate)
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct VoterFlags {
    pub fast_clicker: bool,
    pub side_bias: bool,
    pub low_agreement: bool,
}

impl VoterFlags {
    pub fn any(self) -> bool {
        self.fast_clicker || self.side_bias || self.low_agreement
    }
}

/// Informational flags. They never change what a voter is allowed to do.
pub fn voter_flags(
    votes: i64,
    median_decision_seconds: Option<f64>,
    side_bias_pct: Option<f64>,
    agreement_pct: Option<f64>,
    config: &StatsConfig,
) -> VoterFlags {
    VoterFlags {
        fast_clicker: median_decision_seconds
            .is_some_and(|value| value < config.fast_decision_seconds),
        side_bias: side_bias_pct
            .is_some_and(|value| (value - 50.0).abs() > config.side_bias_margin_pct),
        low_agreement: votes >= config.min_votes_for_flags
            && agreement_pct.is_some_and(|value| value < config.low_agreement_pct),
    }
}

/// A track's rating timeline, used by rank-as-of and rating-chart queries.
#[derive(Debug, Clone, Default)]
pub struct RatingTimeline {
    points: Vec<(DateTime<Utc>, f64)>,
}

impl RatingTimeline {
    pub fn push(&mut self, at: DateTime<Utc>, rating: f64) {
        self.points.push((at, rating));
    }

    /// Latest rating at or before `at`, or the initial rating.
    pub fn rating_at(&self, at: DateTime<Utc>, initial: f64) -> f64 {
        let index = self.points.partition_point(|(point_at, _)| *point_at <= at);
        if index == 0 {
            initial
        } else {
            self.points[index - 1].1
        }
    }

    #[allow(dead_code)]
    pub fn last_rating(&self, initial: f64) -> f64 {
        self.points
            .last()
            .map(|(_, rating)| *rating)
            .unwrap_or(initial)
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.points.len()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }
}

/// Current rank (1-based) for every track, ordered by rating descending.
pub fn rank_map(ratings: &HashMap<Uuid, f64>) -> HashMap<Uuid, i64> {
    let mut entries: Vec<(Uuid, f64)> = ratings.iter().map(|(id, rating)| (*id, *rating)).collect();
    entries.sort_by(|left, right| {
        right
            .1
            .total_cmp(&left.1)
            .then_with(|| left.0.cmp(&right.0))
    });
    entries
        .into_iter()
        .enumerate()
        .map(|(index, (id, _))| (id, index as i64 + 1))
        .collect()
}

/// Rank every track as it stood at `at`, with tracks that had no vote yet at
/// 1000. `timelines` may only contain tracks that have vote history.
pub fn rank_map_at(
    track_ids: &[Uuid],
    timelines: &HashMap<Uuid, RatingTimeline>,
    at: DateTime<Utc>,
    initial: f64,
) -> HashMap<Uuid, i64> {
    let ratings = track_ids
        .iter()
        .map(|id| {
            let rating = timelines
                .get(id)
                .map(|timeline| timeline.rating_at(at, initial))
                .unwrap_or(initial);
            (*id, rating)
        })
        .collect::<HashMap<_, _>>();
    rank_map(&ratings)
}

/// Signed rank movement. Positive means the song climbed.
pub fn rank_change(rank_now: i64, rank_then: Option<i64>) -> Option<i64> {
    rank_then.map(|then| then - rank_now)
}

/// Decision-time histogram buckets used on the voter detail page.
pub const DECISION_BUCKETS: [(&str, f64, f64); 8] = [
    ("<1s", 0.0, 1.0),
    ("1–2s", 1.0, 2.0),
    ("2–3s", 2.0, 3.0),
    ("3–5s", 3.0, 5.0),
    ("5–10s", 5.0, 10.0),
    ("10–20s", 10.0, 20.0),
    ("20–60s", 20.0, 60.0),
    ("60s+", 60.0, f64::INFINITY),
];

pub fn decision_bucket(seconds: f64) -> usize {
    DECISION_BUCKETS
        .iter()
        .position(|(_, low, high)| seconds >= *low && seconds < *high)
        .unwrap_or(DECISION_BUCKETS.len() - 1)
}

/// Position-in-session buckets for the tiredness curve.
pub const POSITION_BUCKETS: [(&str, usize, usize); 5] = [
    ("1–10", 1, 10),
    ("11–25", 11, 25),
    ("26–50", 26, 50),
    ("51–100", 51, 100),
    ("100+", 101, usize::MAX),
];

pub fn position_bucket(position: usize) -> usize {
    POSITION_BUCKETS
        .iter()
        .position(|(_, low, high)| position >= *low && position <= *high)
        .unwrap_or(POSITION_BUCKETS.len() - 1)
}

/// A compact 7×24 (weekday × hour) grid.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Heatmap {
    /// `cells[weekday][hour]`; weekday 0 is Sunday, matching PostgreSQL's DOW.
    pub cells: Vec<Vec<i64>>,
    pub max: i64,
}

impl Heatmap {
    pub fn new(cells: Vec<Vec<i64>>) -> Self {
        let max = cells
            .iter()
            .flat_map(|row| row.iter())
            .copied()
            .max()
            .unwrap_or(0);
        Self { cells, max }
    }

    #[allow(dead_code)]
    pub fn empty() -> Self {
        Self {
            cells: vec![vec![0; 24]; 7],
            max: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(second: i64) -> DateTime<Utc> {
        Utc.timestamp_opt(second, 0).unwrap()
    }

    #[test]
    fn win_chance_is_fifty_fifty_for_equal_ratings() {
        assert!((win_chance(1000.0, 1000.0) - 50.0).abs() < 1e-9);
        assert!((win_chance(1200.0, 1000.0) - 75.9746926648).abs() < 1e-6);
        assert!((win_chance(1000.0, 1200.0) - 24.0253073352).abs() < 1e-6);
    }

    #[test]
    fn upset_rule_uses_the_winner_chance() {
        assert!(is_upset(34.9, 35.0));
        assert!(!is_upset(35.0, 35.0));
        assert!(!is_upset(80.0, 35.0));
    }

    #[test]
    fn sides_follow_served_flip() {
        assert_eq!(side_of_low(false), Side::A);
        assert_eq!(side_of_high(false), Side::B);
        assert_eq!(side_of_low(true), Side::B);
        assert_eq!(side_of_high(true), Side::A);
        assert_eq!(winner_side(true, false), Side::A);
        assert_eq!(winner_side(true, true), Side::B);
        assert_eq!(winner_side(false, false), Side::B);
        assert_eq!(winner_side(false, true), Side::A);
    }

    #[test]
    fn rating_after_uses_the_same_side_mapping() {
        assert!((rating_after(true, 1000.0, 1100.0, 12.0, -12.0) - 1012.0).abs() < 1e-9);
        assert!((rating_after(false, 1000.0, 1100.0, 12.0, -12.0) - 1088.0).abs() < 1e-9);
    }

    #[test]
    fn rank_weight_decreases_with_rank() {
        assert!((rank_weight(0, 6.0, 500.0) - 7.0).abs() < 1e-9);
        assert!(rank_weight(500, 6.0, 500.0) < rank_weight(0, 6.0, 500.0));
        assert!((rank_weight(0, 0.0, 500.0) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn boundaries_map_to_the_expected_buckets() {
        assert_eq!(rank_bucket_index(1), 0);
        assert_eq!(rank_bucket_index(50), 0);
        assert_eq!(rank_bucket_index(51), 1);
        assert_eq!(rank_bucket_index(100), 1);
        assert_eq!(rank_bucket_index(101), 2);
        assert_eq!(rank_bucket_index(300), 2);
        assert_eq!(rank_bucket_index(301), 3);
        assert_eq!(rank_bucket_index(1000), 3);
        assert_eq!(rank_bucket_index(1001), 4);
        assert_eq!(rank_bucket_index(50_000), 4);
    }

    #[test]
    fn percentiles_interpolate_and_handle_empty_samples() {
        assert_eq!(percentile(&[], 0.5), None);
        assert_eq!(percentile(&[4.0], 0.9), Some(4.0));
        assert_eq!(percentile(&[1.0, 2.0, 3.0], 0.5), Some(2.0));
        assert_eq!(percentile(&[1.0, 2.0, 3.0], 0.0), Some(1.0));
        assert_eq!(percentile(&[1.0, 2.0, 3.0], 1.0), Some(3.0));
        assert!((percentile(&[0.0, 10.0], 0.25).unwrap() - 2.5).abs() < 1e-9);
    }

    #[test]
    fn sessions_split_on_gaps_longer_than_the_threshold() {
        let times = vec![
            at(0),
            at(60),
            at(120),
            at(120 + 15 * 60),       // exactly the gap, same session
            at(120 + 15 * 60 + 901), // one second longer than the gap, new session
            at(120 + 15 * 60 + 931),
        ];
        let sessions = split_sessions(&times, 15 * 60);
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0], SessionRange { start: 0, end: 4 });
        assert_eq!(sessions[1], SessionRange { start: 4, end: 6 });
        assert_eq!(sessions[1].len(), 2);
    }

    #[test]
    fn empty_sessions_are_empty() {
        assert!(split_sessions(&[], 900).is_empty());
    }

    #[test]
    fn idle_decisions_are_excluded() {
        assert_eq!(decision_seconds(at(0), at(30), 900), Some(30.0));
        assert_eq!(decision_seconds(at(0), at(901), 900), None);
        assert_eq!(decision_seconds(at(30), at(0), 900), None);
    }

    #[test]
    fn evenness_is_p10_over_p90_and_clamped() {
        let value = evenness(&[1.0, 2.0, 3.0, 4.0, 5.0]).unwrap();
        assert!(value > 0.0 && value <= 1.0);
        assert_eq!(evenness(&[]), None);
        assert_eq!(evenness(&[0.0, 0.0]), None);
        assert!((evenness(&[2.0, 2.0, 2.0]).unwrap() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn settling_status_thresholds_are_named_and_ordered() {
        assert_eq!(
            settling_status(1.0, 6.0, Some(0.95)),
            SettlingStatus::NotStarted
        );
        assert_eq!(
            settling_status(10.0, 6.0, Some(0.4)),
            SettlingStatus::CatchingUpEarly
        );
        assert_eq!(
            settling_status(10.0, 6.0, Some(0.6)),
            SettlingStatus::CatchingUpLate
        );
        assert_eq!(
            settling_status(10.0, 6.0, Some(0.85)),
            SettlingStatus::Settled
        );
    }

    #[test]
    fn duels_left_guards_and_formula() {
        let estimate = duels_left(0.5, 0.4, 1000.0).unwrap();
        assert!(estimate > 0.0);
        assert_eq!(duels_left(0.8, 0.4, 1000.0), None);
        assert_eq!(duels_left(0.5, 0.6, 1000.0), None);
        assert_eq!(duels_left(0.5, 0.4, 0.0), None);
        assert_eq!(duels_left(0.5, 0.4, f64::NAN), None);
    }

    #[test]
    fn flag_rules_are_informational_and_use_the_thresholds() {
        let config = StatsConfig {
            session_gap_minutes: 15,
            fast_decision_seconds: 1.5,
            upset_threshold_pct: 35.0,
            side_bias_margin_pct: 15.0,
            low_agreement_pct: 55.0,
            min_votes_for_flags: 30,
            target_duels_per_song: 30.0,
        };

        let all = voter_flags(100, Some(1.0), Some(70.0), Some(50.0), &config);
        assert!(all.fast_clicker && all.side_bias && all.low_agreement);

        let none = voter_flags(100, Some(2.0), Some(55.0), Some(60.0), &config);
        assert!(!none.any());

        // Low agreement needs the minimum sample.
        let too_few = voter_flags(10, Some(2.0), Some(50.0), Some(10.0), &config);
        assert!(!too_few.low_agreement);
    }

    #[test]
    fn timelines_report_the_latest_rating_before_a_time() {
        let mut timeline = RatingTimeline::default();
        timeline.push(at(100), 1010.0);
        timeline.push(at(200), 1020.0);
        assert_eq!(timeline.rating_at(at(50), 1000.0), 1000.0);
        assert_eq!(timeline.rating_at(at(100), 1000.0), 1010.0);
        assert_eq!(timeline.rating_at(at(150), 1000.0), 1010.0);
        assert_eq!(timeline.rating_at(at(200), 1000.0), 1020.0);
        assert_eq!(timeline.last_rating(1000.0), 1020.0);
    }

    #[test]
    fn rank_map_is_deterministic_on_ties() {
        let low = Uuid::from_u128(1);
        let high = Uuid::from_u128(2);
        let ratings = HashMap::from([(high, 1200.0), (low, 1200.0)]);
        let ranks = rank_map(&ratings);
        assert_eq!(ranks[&low], 1);
        assert_eq!(ranks[&high], 2);
    }

    #[test]
    fn rank_as_of_puts_unseen_tracks_at_one_thousand() {
        let seen = Uuid::from_u128(1);
        let unseen = Uuid::from_u128(2);
        let mut timelines = HashMap::new();
        let mut timeline = RatingTimeline::default();
        timeline.push(at(100), 1100.0);
        timelines.insert(seen, timeline);

        let now = rank_map_at(&[seen, unseen], &timelines, at(200), 1000.0);
        assert_eq!(now[&seen], 1);
        assert_eq!(now[&unseen], 2);
    }

    #[test]
    fn decision_and_position_buckets_cover_every_value() {
        assert_eq!(decision_bucket(0.2), 0);
        assert_eq!(decision_bucket(1.0), 1);
        assert_eq!(decision_bucket(59.9), 6);
        assert_eq!(decision_bucket(60.0), 7);
        assert_eq!(decision_bucket(100_000.0), 7);
        assert_eq!(position_bucket(1), 0);
        assert_eq!(position_bucket(10), 0);
        assert_eq!(position_bucket(11), 1);
        assert_eq!(position_bucket(101), 4);
    }

    #[test]
    fn rank_change_positive_is_a_climb() {
        assert_eq!(rank_change(10, Some(20)), Some(10));
        assert_eq!(rank_change(10, Some(5)), Some(-5));
        assert_eq!(rank_change(10, None), None);
    }
}
