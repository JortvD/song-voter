use std::collections::HashMap;

use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};
use sqlx::PgConnection;

use crate::auth::AdminUser;
use crate::error::ApiError;
use crate::state::AppState;

pub const DEFAULT_DAILY_STREAK_GOAL: i64 = 3;
pub const MIN_DAILY_STREAK_GOAL: i64 = 1;
pub const MAX_DAILY_STREAK_GOAL: i64 = 100;

// Live pairing defaults. These are the values `matchup.rs` used before they
// became configurable, and they are the only place the numbers live now.
pub const DEFAULT_RANK_WEIGHT_STRENGTH: f64 = 6.0;
pub const MIN_RANK_WEIGHT_STRENGTH: f64 = 0.0;
pub const MAX_RANK_WEIGHT_STRENGTH: f64 = 12.0;
pub const DEFAULT_RANK_WEIGHT_SCALE: f64 = 500.0;
pub const MIN_RANK_WEIGHT_SCALE: f64 = 100.0;
pub const MAX_RANK_WEIGHT_SCALE: f64 = 2_000.0;
pub const DEFAULT_RANK_WEIGHT_MIN_AVG: f64 = 6.0;
pub const MIN_RANK_WEIGHT_MIN_AVG: f64 = 0.0;
pub const MAX_RANK_WEIGHT_MIN_AVG: f64 = 30.0;

// Statistics-only defaults. None of these affect duel selection.
pub const DEFAULT_STATS_SESSION_GAP_MINUTES: i64 = 15;
pub const MIN_STATS_SESSION_GAP_MINUTES: i64 = 5;
pub const MAX_STATS_SESSION_GAP_MINUTES: i64 = 120;
pub const DEFAULT_STATS_FAST_DECISION_SECONDS: f64 = 1.5;
pub const MIN_STATS_FAST_DECISION_SECONDS: f64 = 0.5;
pub const MAX_STATS_FAST_DECISION_SECONDS: f64 = 10.0;
pub const DEFAULT_STATS_UPSET_THRESHOLD_PCT: f64 = 35.0;
pub const MIN_STATS_UPSET_THRESHOLD_PCT: f64 = 10.0;
pub const MAX_STATS_UPSET_THRESHOLD_PCT: f64 = 49.0;
pub const DEFAULT_STATS_SIDE_BIAS_MARGIN_PCT: f64 = 15.0;
pub const MIN_STATS_SIDE_BIAS_MARGIN_PCT: f64 = 5.0;
pub const MAX_STATS_SIDE_BIAS_MARGIN_PCT: f64 = 40.0;
pub const DEFAULT_STATS_LOW_AGREEMENT_PCT: f64 = 55.0;
pub const MIN_STATS_LOW_AGREEMENT_PCT: f64 = 50.0;
pub const MAX_STATS_LOW_AGREEMENT_PCT: f64 = 80.0;
pub const DEFAULT_STATS_MIN_VOTES_FOR_FLAGS: i64 = 30;
pub const MIN_STATS_MIN_VOTES_FOR_FLAGS: i64 = 5;
pub const MAX_STATS_MIN_VOTES_FOR_FLAGS: i64 = 500;
pub const DEFAULT_STATS_TARGET_DUELS_PER_SONG: f64 = 30.0;
pub const MIN_STATS_TARGET_DUELS_PER_SONG: f64 = 5.0;
pub const MAX_STATS_TARGET_DUELS_PER_SONG: f64 = 200.0;

const DAILY_STREAK_GOAL_KEY: &str = "daily_streak_goal";
const RANK_WEIGHTING_KEY: &str = "rank_weighting";
const RANK_WEIGHT_STRENGTH_KEY: &str = "rank_weight_strength";
const RANK_WEIGHT_SCALE_KEY: &str = "rank_weight_scale";
const RANK_WEIGHT_MIN_AVG_KEY: &str = "rank_weight_min_avg";
const STATS_SESSION_GAP_KEY: &str = "stats_session_gap_minutes";
const STATS_FAST_DECISION_KEY: &str = "stats_fast_decision_seconds";
const STATS_UPSET_THRESHOLD_KEY: &str = "stats_upset_threshold_pct";
const STATS_SIDE_BIAS_MARGIN_KEY: &str = "stats_side_bias_margin_pct";
const STATS_LOW_AGREEMENT_KEY: &str = "stats_low_agreement_pct";
const STATS_MIN_VOTES_FOR_FLAGS_KEY: &str = "stats_min_votes_for_flags";
const STATS_TARGET_DUELS_KEY: &str = "stats_target_duels_per_song";

const LIVE_KEYS: [&str; 4] = [
    RANK_WEIGHTING_KEY,
    RANK_WEIGHT_STRENGTH_KEY,
    RANK_WEIGHT_SCALE_KEY,
    RANK_WEIGHT_MIN_AVG_KEY,
];

const STATS_KEYS: [&str; 7] = [
    STATS_SESSION_GAP_KEY,
    STATS_FAST_DECISION_KEY,
    STATS_UPSET_THRESHOLD_KEY,
    STATS_SIDE_BIAS_MARGIN_KEY,
    STATS_LOW_AGREEMENT_KEY,
    STATS_MIN_VOTES_FOR_FLAGS_KEY,
    STATS_TARGET_DUELS_KEY,
];

#[derive(Debug, Clone, Copy, Serialize)]
pub struct RankWeightingConfig {
    pub enabled: bool,
    pub strength: f64,
    pub scale: f64,
    pub min_avg: f64,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct StatsConfig {
    pub session_gap_minutes: i64,
    pub fast_decision_seconds: f64,
    pub upset_threshold_pct: f64,
    pub side_bias_margin_pct: f64,
    pub low_agreement_pct: f64,
    pub min_votes_for_flags: i64,
    pub target_duels_per_song: f64,
}

impl StatsConfig {
    pub fn session_gap_seconds(self) -> i64 {
        self.session_gap_minutes * 60
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct SettingsDefaults {
    pub daily_streak_goal: i64,
    pub rank_weight_strength: f64,
    pub rank_weight_scale: f64,
    pub rank_weight_min_avg: f64,
    pub stats_session_gap_minutes: i64,
    pub stats_fast_decision_seconds: f64,
    pub stats_upset_threshold_pct: f64,
    pub stats_side_bias_margin_pct: f64,
    pub stats_low_agreement_pct: f64,
    pub stats_min_votes_for_flags: i64,
    pub stats_target_duels_per_song: f64,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct SettingsRanges {
    pub daily_streak_goal: [i64; 2],
    pub rank_weight_strength: [f64; 2],
    pub rank_weight_scale: [f64; 2],
    pub rank_weight_min_avg: [f64; 2],
    pub stats_session_gap_minutes: [i64; 2],
    pub stats_fast_decision_seconds: [f64; 2],
    pub stats_upset_threshold_pct: [f64; 2],
    pub stats_side_bias_margin_pct: [f64; 2],
    pub stats_low_agreement_pct: [f64; 2],
    pub stats_min_votes_for_flags: [i64; 2],
    pub stats_target_duels_per_song: [f64; 2],
}

impl SettingsDefaults {
    pub const fn current() -> Self {
        Self {
            daily_streak_goal: DEFAULT_DAILY_STREAK_GOAL,
            rank_weight_strength: DEFAULT_RANK_WEIGHT_STRENGTH,
            rank_weight_scale: DEFAULT_RANK_WEIGHT_SCALE,
            rank_weight_min_avg: DEFAULT_RANK_WEIGHT_MIN_AVG,
            stats_session_gap_minutes: DEFAULT_STATS_SESSION_GAP_MINUTES,
            stats_fast_decision_seconds: DEFAULT_STATS_FAST_DECISION_SECONDS,
            stats_upset_threshold_pct: DEFAULT_STATS_UPSET_THRESHOLD_PCT,
            stats_side_bias_margin_pct: DEFAULT_STATS_SIDE_BIAS_MARGIN_PCT,
            stats_low_agreement_pct: DEFAULT_STATS_LOW_AGREEMENT_PCT,
            stats_min_votes_for_flags: DEFAULT_STATS_MIN_VOTES_FOR_FLAGS,
            stats_target_duels_per_song: DEFAULT_STATS_TARGET_DUELS_PER_SONG,
        }
    }
}

impl SettingsRanges {
    pub const fn current() -> Self {
        Self {
            daily_streak_goal: [MIN_DAILY_STREAK_GOAL, MAX_DAILY_STREAK_GOAL],
            rank_weight_strength: [MIN_RANK_WEIGHT_STRENGTH, MAX_RANK_WEIGHT_STRENGTH],
            rank_weight_scale: [MIN_RANK_WEIGHT_SCALE, MAX_RANK_WEIGHT_SCALE],
            rank_weight_min_avg: [MIN_RANK_WEIGHT_MIN_AVG, MAX_RANK_WEIGHT_MIN_AVG],
            stats_session_gap_minutes: [
                MIN_STATS_SESSION_GAP_MINUTES,
                MAX_STATS_SESSION_GAP_MINUTES,
            ],
            stats_fast_decision_seconds: [
                MIN_STATS_FAST_DECISION_SECONDS,
                MAX_STATS_FAST_DECISION_SECONDS,
            ],
            stats_upset_threshold_pct: [
                MIN_STATS_UPSET_THRESHOLD_PCT,
                MAX_STATS_UPSET_THRESHOLD_PCT,
            ],
            stats_side_bias_margin_pct: [
                MIN_STATS_SIDE_BIAS_MARGIN_PCT,
                MAX_STATS_SIDE_BIAS_MARGIN_PCT,
            ],
            stats_low_agreement_pct: [MIN_STATS_LOW_AGREEMENT_PCT, MAX_STATS_LOW_AGREEMENT_PCT],
            stats_min_votes_for_flags: [
                MIN_STATS_MIN_VOTES_FOR_FLAGS,
                MAX_STATS_MIN_VOTES_FOR_FLAGS,
            ],
            stats_target_duels_per_song: [
                MIN_STATS_TARGET_DUELS_PER_SONG,
                MAX_STATS_TARGET_DUELS_PER_SONG,
            ],
        }
    }
}

#[derive(Debug, Serialize)]
pub struct AdminSettingsResponse {
    pub daily_streak_goal: i64,
    /// Kept for compatibility with the original settings response.
    pub min: i64,
    pub max: i64,
    pub rank_weighting: bool,
    pub rank_weight_strength: f64,
    pub rank_weight_scale: f64,
    pub rank_weight_min_avg: f64,
    pub stats_session_gap_minutes: i64,
    pub stats_fast_decision_seconds: f64,
    pub stats_upset_threshold_pct: f64,
    pub stats_side_bias_margin_pct: f64,
    pub stats_low_agreement_pct: f64,
    pub stats_min_votes_for_flags: i64,
    pub stats_target_duels_per_song: f64,
    pub defaults: SettingsDefaults,
    pub ranges: SettingsRanges,
}

#[derive(Debug, Default, Deserialize)]
pub struct UpdateSettingsRequest {
    pub daily_streak_goal: Option<i64>,
    pub rank_weighting: Option<bool>,
    pub rank_weight_strength: Option<f64>,
    pub rank_weight_scale: Option<f64>,
    pub rank_weight_min_avg: Option<f64>,
    pub stats_session_gap_minutes: Option<i64>,
    pub stats_fast_decision_seconds: Option<f64>,
    pub stats_upset_threshold_pct: Option<f64>,
    pub stats_side_bias_margin_pct: Option<f64>,
    pub stats_low_agreement_pct: Option<f64>,
    pub stats_min_votes_for_flags: Option<i64>,
    pub stats_target_duels_per_song: Option<f64>,
}

async fn load_values(
    connection: &mut PgConnection,
    keys: &[&str],
) -> Result<HashMap<String, String>, ApiError> {
    let rows = sqlx::query_as::<_, (String, String)>(
        "SELECT key, value FROM app_settings WHERE key = ANY($1)",
    )
    .bind(keys)
    .fetch_all(&mut *connection)
    .await?;

    Ok(rows.into_iter().collect())
}

/// Parse a numeric setting, falling back to the default and clamping into the
/// documented range. Malformed, non-finite and missing values are all safe.
pub fn parse_f64(value: Option<&str>, default: f64, min: f64, max: f64) -> f64 {
    value
        .and_then(|value| value.trim().parse::<f64>().ok())
        .filter(|value| value.is_finite())
        .unwrap_or(default)
        .clamp(min, max)
}

pub fn parse_i64(value: Option<&str>, default: i64, min: i64, max: i64) -> i64 {
    value
        .and_then(|value| value.trim().parse::<i64>().ok())
        .unwrap_or(default)
        .clamp(min, max)
}

/// Whether duel selection should give higher-rated tracks proportionally more
/// matchups. A missing setting means enabled; `off`, `false` or `0` (case-
/// insensitive, trimmed) disable it.
pub fn parse_rank_weighting(value: Option<&str>) -> bool {
    match value {
        Some(value) => !matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "off" | "false" | "0"
        ),
        None => true,
    }
}

pub async fn rank_weighting_config(
    connection: &mut PgConnection,
) -> Result<RankWeightingConfig, ApiError> {
    let values = load_values(connection, &LIVE_KEYS).await?;

    Ok(RankWeightingConfig {
        enabled: parse_rank_weighting(values.get(RANK_WEIGHTING_KEY).map(String::as_str)),
        strength: parse_f64(
            values.get(RANK_WEIGHT_STRENGTH_KEY).map(String::as_str),
            DEFAULT_RANK_WEIGHT_STRENGTH,
            MIN_RANK_WEIGHT_STRENGTH,
            MAX_RANK_WEIGHT_STRENGTH,
        ),
        scale: parse_f64(
            values.get(RANK_WEIGHT_SCALE_KEY).map(String::as_str),
            DEFAULT_RANK_WEIGHT_SCALE,
            MIN_RANK_WEIGHT_SCALE,
            MAX_RANK_WEIGHT_SCALE,
        ),
        min_avg: parse_f64(
            values.get(RANK_WEIGHT_MIN_AVG_KEY).map(String::as_str),
            DEFAULT_RANK_WEIGHT_MIN_AVG,
            MIN_RANK_WEIGHT_MIN_AVG,
            MAX_RANK_WEIGHT_MIN_AVG,
        ),
    })
}

pub async fn stats_config(connection: &mut PgConnection) -> Result<StatsConfig, ApiError> {
    let values = load_values(connection, &STATS_KEYS).await?;
    Ok(stats_config_from_values(&values))
}

fn stats_config_from_values(values: &HashMap<String, String>) -> StatsConfig {
    StatsConfig {
        session_gap_minutes: parse_i64(
            values.get(STATS_SESSION_GAP_KEY).map(String::as_str),
            DEFAULT_STATS_SESSION_GAP_MINUTES,
            MIN_STATS_SESSION_GAP_MINUTES,
            MAX_STATS_SESSION_GAP_MINUTES,
        ),
        fast_decision_seconds: parse_f64(
            values.get(STATS_FAST_DECISION_KEY).map(String::as_str),
            DEFAULT_STATS_FAST_DECISION_SECONDS,
            MIN_STATS_FAST_DECISION_SECONDS,
            MAX_STATS_FAST_DECISION_SECONDS,
        ),
        upset_threshold_pct: parse_f64(
            values.get(STATS_UPSET_THRESHOLD_KEY).map(String::as_str),
            DEFAULT_STATS_UPSET_THRESHOLD_PCT,
            MIN_STATS_UPSET_THRESHOLD_PCT,
            MAX_STATS_UPSET_THRESHOLD_PCT,
        ),
        side_bias_margin_pct: parse_f64(
            values.get(STATS_SIDE_BIAS_MARGIN_KEY).map(String::as_str),
            DEFAULT_STATS_SIDE_BIAS_MARGIN_PCT,
            MIN_STATS_SIDE_BIAS_MARGIN_PCT,
            MAX_STATS_SIDE_BIAS_MARGIN_PCT,
        ),
        low_agreement_pct: parse_f64(
            values.get(STATS_LOW_AGREEMENT_KEY).map(String::as_str),
            DEFAULT_STATS_LOW_AGREEMENT_PCT,
            MIN_STATS_LOW_AGREEMENT_PCT,
            MAX_STATS_LOW_AGREEMENT_PCT,
        ),
        min_votes_for_flags: parse_i64(
            values
                .get(STATS_MIN_VOTES_FOR_FLAGS_KEY)
                .map(String::as_str),
            DEFAULT_STATS_MIN_VOTES_FOR_FLAGS,
            MIN_STATS_MIN_VOTES_FOR_FLAGS,
            MAX_STATS_MIN_VOTES_FOR_FLAGS,
        ),
        target_duels_per_song: parse_f64(
            values.get(STATS_TARGET_DUELS_KEY).map(String::as_str),
            DEFAULT_STATS_TARGET_DUELS_PER_SONG,
            MIN_STATS_TARGET_DUELS_PER_SONG,
            MAX_STATS_TARGET_DUELS_PER_SONG,
        ),
    }
}

/// Read the app-wide daily streak goal. Unknown or malformed values fall back
/// to 3 and are clamped to the supported range.
pub async fn daily_streak_goal(connection: &mut PgConnection) -> Result<i64, ApiError> {
    let value = sqlx::query_scalar::<_, String>("SELECT value FROM app_settings WHERE key = $1")
        .bind(DAILY_STREAK_GOAL_KEY)
        .fetch_optional(&mut *connection)
        .await?;

    Ok(parse_i64(
        value.as_deref(),
        DEFAULT_DAILY_STREAK_GOAL,
        MIN_DAILY_STREAK_GOAL,
        MAX_DAILY_STREAK_GOAL,
    ))
}

async fn build_response(connection: &mut PgConnection) -> Result<AdminSettingsResponse, ApiError> {
    let goal = daily_streak_goal(connection).await?;
    let rank = rank_weighting_config(connection).await?;
    let stats = stats_config(connection).await?;

    Ok(AdminSettingsResponse {
        daily_streak_goal: goal,
        min: MIN_DAILY_STREAK_GOAL,
        max: MAX_DAILY_STREAK_GOAL,
        rank_weighting: rank.enabled,
        rank_weight_strength: rank.strength,
        rank_weight_scale: rank.scale,
        rank_weight_min_avg: rank.min_avg,
        stats_session_gap_minutes: stats.session_gap_minutes,
        stats_fast_decision_seconds: stats.fast_decision_seconds,
        stats_upset_threshold_pct: stats.upset_threshold_pct,
        stats_side_bias_margin_pct: stats.side_bias_margin_pct,
        stats_low_agreement_pct: stats.low_agreement_pct,
        stats_min_votes_for_flags: stats.min_votes_for_flags,
        stats_target_duels_per_song: stats.target_duels_per_song,
        defaults: SettingsDefaults::current(),
        ranges: SettingsRanges::current(),
    })
}

pub async fn get_settings(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> Result<Json<AdminSettingsResponse>, ApiError> {
    let mut connection = state.db.acquire().await?;
    Ok(Json(build_response(&mut connection).await?))
}

fn clamped_f64(value: Option<f64>, default: f64, min: f64, max: f64) -> Option<f64> {
    value.map(|value| {
        if value.is_finite() {
            value.clamp(min, max)
        } else {
            default
        }
    })
}

async fn upsert(connection: &mut PgConnection, key: &str, value: String) -> Result<(), ApiError> {
    sqlx::query(
        "INSERT INTO app_settings (key, value, updated_at) \
         VALUES ($1, $2, now()) \
         ON CONFLICT (key) DO UPDATE SET \
             value = EXCLUDED.value, updated_at = now()",
    )
    .bind(key)
    .bind(value)
    .execute(&mut *connection)
    .await?;
    Ok(())
}

pub async fn update_settings(
    State(state): State<AppState>,
    admin: AdminUser,
    Json(payload): Json<UpdateSettingsRequest>,
) -> Result<Json<AdminSettingsResponse>, ApiError> {
    let mut connection = state.db.acquire().await?;
    let defaults = SettingsDefaults::current();
    let ranges = SettingsRanges::current();

    let mut changes: Vec<(&str, String)> = Vec::new();

    if let Some(goal) = payload.daily_streak_goal {
        let goal = goal.clamp(ranges.daily_streak_goal[0], ranges.daily_streak_goal[1]);
        changes.push((DAILY_STREAK_GOAL_KEY, goal.to_string()));
    }
    if let Some(enabled) = payload.rank_weighting {
        changes.push((
            RANK_WEIGHTING_KEY,
            if enabled { "on" } else { "off" }.to_string(),
        ));
    }
    if let Some(value) = clamped_f64(
        payload.rank_weight_strength,
        defaults.rank_weight_strength,
        ranges.rank_weight_strength[0],
        ranges.rank_weight_strength[1],
    ) {
        changes.push((RANK_WEIGHT_STRENGTH_KEY, value.to_string()));
    }
    if let Some(value) = clamped_f64(
        payload.rank_weight_scale,
        defaults.rank_weight_scale,
        ranges.rank_weight_scale[0],
        ranges.rank_weight_scale[1],
    ) {
        changes.push((RANK_WEIGHT_SCALE_KEY, value.to_string()));
    }
    if let Some(value) = clamped_f64(
        payload.rank_weight_min_avg,
        defaults.rank_weight_min_avg,
        ranges.rank_weight_min_avg[0],
        ranges.rank_weight_min_avg[1],
    ) {
        changes.push((RANK_WEIGHT_MIN_AVG_KEY, value.to_string()));
    }
    if let Some(value) = payload.stats_session_gap_minutes {
        let value = value.clamp(
            ranges.stats_session_gap_minutes[0],
            ranges.stats_session_gap_minutes[1],
        );
        changes.push((STATS_SESSION_GAP_KEY, value.to_string()));
    }
    if let Some(value) = clamped_f64(
        payload.stats_fast_decision_seconds,
        defaults.stats_fast_decision_seconds,
        ranges.stats_fast_decision_seconds[0],
        ranges.stats_fast_decision_seconds[1],
    ) {
        changes.push((STATS_FAST_DECISION_KEY, value.to_string()));
    }
    if let Some(value) = clamped_f64(
        payload.stats_upset_threshold_pct,
        defaults.stats_upset_threshold_pct,
        ranges.stats_upset_threshold_pct[0],
        ranges.stats_upset_threshold_pct[1],
    ) {
        changes.push((STATS_UPSET_THRESHOLD_KEY, value.to_string()));
    }
    if let Some(value) = clamped_f64(
        payload.stats_side_bias_margin_pct,
        defaults.stats_side_bias_margin_pct,
        ranges.stats_side_bias_margin_pct[0],
        ranges.stats_side_bias_margin_pct[1],
    ) {
        changes.push((STATS_SIDE_BIAS_MARGIN_KEY, value.to_string()));
    }
    if let Some(value) = clamped_f64(
        payload.stats_low_agreement_pct,
        defaults.stats_low_agreement_pct,
        ranges.stats_low_agreement_pct[0],
        ranges.stats_low_agreement_pct[1],
    ) {
        changes.push((STATS_LOW_AGREEMENT_KEY, value.to_string()));
    }
    if let Some(value) = payload.stats_min_votes_for_flags {
        let value = value.clamp(
            ranges.stats_min_votes_for_flags[0],
            ranges.stats_min_votes_for_flags[1],
        );
        changes.push((STATS_MIN_VOTES_FOR_FLAGS_KEY, value.to_string()));
    }
    if let Some(value) = clamped_f64(
        payload.stats_target_duels_per_song,
        defaults.stats_target_duels_per_song,
        ranges.stats_target_duels_per_song[0],
        ranges.stats_target_duels_per_song[1],
    ) {
        changes.push((STATS_TARGET_DUELS_KEY, value.to_string()));
    }

    for (key, value) in &changes {
        upsert(&mut connection, key, value.clone()).await?;
        tracing::info!(
            admin_id = %admin.0.id,
            admin_username = %admin.0.username,
            key,
            value,
            "app setting updated"
        );
    }

    // Statistics are cached for a short while; a settings change must be
    // visible immediately, especially the flag thresholds.
    state.cache.clear();

    Ok(Json(build_response(&mut connection).await?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rank_weighting_is_enabled_when_the_setting_is_missing() {
        assert!(parse_rank_weighting(None));
    }

    #[test]
    fn rank_weighting_is_disabled_by_off_false_or_zero() {
        for value in ["off", "false", "0", "OFF", " OFF ", "False", "0\t"] {
            assert!(
                !parse_rank_weighting(Some(value)),
                "expected {value:?} to disable rank weighting"
            );
        }
    }

    #[test]
    fn rank_weighting_stays_enabled_for_any_other_value() {
        for value in ["on", "true", "1", "yes", "disabled", ""] {
            assert!(
                parse_rank_weighting(Some(value)),
                "expected {value:?} to keep rank weighting enabled"
            );
        }
    }

    #[test]
    fn numeric_settings_fall_back_and_clamp() {
        assert_eq!(parse_f64(None, 500.0, 100.0, 2000.0), 500.0);
        assert_eq!(parse_f64(Some("nope"), 500.0, 100.0, 2000.0), 500.0);
        assert_eq!(parse_f64(Some("1"), 500.0, 100.0, 2000.0), 100.0);
        assert_eq!(parse_f64(Some("99999"), 500.0, 100.0, 2000.0), 2000.0);
        assert_eq!(parse_f64(Some("NaN"), 500.0, 100.0, 2000.0), 500.0);
        assert_eq!(parse_f64(Some("inf"), 500.0, 100.0, 2000.0), 500.0);
        assert_eq!(parse_i64(Some("3.5"), 15, 5, 120), 15);
        assert_eq!(parse_i64(Some("-2"), 15, 5, 120), 5);
        assert_eq!(parse_i64(Some("900"), 15, 5, 120), 120);
    }

    #[test]
    fn stats_config_uses_defaults_for_missing_rows() {
        let config = stats_config_from_values(&HashMap::new());
        assert_eq!(
            config.session_gap_minutes,
            DEFAULT_STATS_SESSION_GAP_MINUTES
        );
        assert_eq!(
            config.fast_decision_seconds,
            DEFAULT_STATS_FAST_DECISION_SECONDS
        );
        assert_eq!(
            config.upset_threshold_pct,
            DEFAULT_STATS_UPSET_THRESHOLD_PCT
        );
        assert_eq!(
            config.side_bias_margin_pct,
            DEFAULT_STATS_SIDE_BIAS_MARGIN_PCT
        );
        assert_eq!(config.low_agreement_pct, DEFAULT_STATS_LOW_AGREEMENT_PCT);
        assert_eq!(
            config.min_votes_for_flags,
            DEFAULT_STATS_MIN_VOTES_FOR_FLAGS
        );
        assert_eq!(
            config.target_duels_per_song,
            DEFAULT_STATS_TARGET_DUELS_PER_SONG
        );
    }

    #[test]
    fn stats_config_clamps_stored_values() {
        let values = HashMap::from([
            (STATS_SESSION_GAP_KEY.to_string(), "999".to_string()),
            (STATS_FAST_DECISION_KEY.to_string(), "-1".to_string()),
            (STATS_UPSET_THRESHOLD_KEY.to_string(), "0".to_string()),
            (STATS_LOW_AGREEMENT_KEY.to_string(), "99".to_string()),
            (STATS_TARGET_DUELS_KEY.to_string(), "1000".to_string()),
        ]);
        let config = stats_config_from_values(&values);
        assert_eq!(config.session_gap_minutes, MAX_STATS_SESSION_GAP_MINUTES);
        assert_eq!(
            config.fast_decision_seconds,
            MIN_STATS_FAST_DECISION_SECONDS
        );
        assert_eq!(config.upset_threshold_pct, MIN_STATS_UPSET_THRESHOLD_PCT);
        assert_eq!(config.low_agreement_pct, MAX_STATS_LOW_AGREEMENT_PCT);
        assert_eq!(
            config.target_duels_per_song,
            MAX_STATS_TARGET_DUELS_PER_SONG
        );
    }

    #[test]
    fn defaults_and_ranges_agree() {
        let defaults = SettingsDefaults::current();
        let ranges = SettingsRanges::current();
        assert!(ranges.rank_weight_strength[0] <= defaults.rank_weight_strength);
        assert!(defaults.rank_weight_strength <= ranges.rank_weight_strength[1]);
        assert!(ranges.stats_target_duels_per_song[0] <= defaults.stats_target_duels_per_song);
        assert!(defaults.stats_target_duels_per_song <= ranges.stats_target_duels_per_song[1]);
    }
}
