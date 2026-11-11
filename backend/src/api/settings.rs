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
const DAILY_STREAK_GOAL_KEY: &str = "daily_streak_goal";
const RANK_WEIGHTING_KEY: &str = "rank_weighting";

#[derive(Debug, Serialize)]
pub struct AdminSettingsResponse {
    daily_streak_goal: i64,
    min: i64,
    max: i64,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSettingsRequest {
    daily_streak_goal: i64,
}

/// Read the app-wide daily streak goal. Unknown or malformed values fall back
/// to 3 and are clamped to the supported range.
pub async fn daily_streak_goal(connection: &mut PgConnection) -> Result<i64, ApiError> {
    let value = sqlx::query_scalar::<_, String>("SELECT value FROM app_settings WHERE key = $1")
        .bind(DAILY_STREAK_GOAL_KEY)
        .fetch_optional(&mut *connection)
        .await?;

    let goal = value
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(DEFAULT_DAILY_STREAK_GOAL);

    Ok(goal.clamp(MIN_DAILY_STREAK_GOAL, MAX_DAILY_STREAK_GOAL))
}

/// Whether duel selection should give higher-rated tracks proportionally more
/// matchups. A missing setting means enabled; `off`, `false` or `0` (case-
/// insensitive, trimmed) disable it.
pub async fn rank_weighting_enabled(connection: &mut PgConnection) -> Result<bool, ApiError> {
    let value = sqlx::query_scalar::<_, String>("SELECT value FROM app_settings WHERE key = $1")
        .bind(RANK_WEIGHTING_KEY)
        .fetch_optional(&mut *connection)
        .await?;

    Ok(parse_rank_weighting(value.as_deref()))
}

fn parse_rank_weighting(value: Option<&str>) -> bool {
    match value {
        Some(value) => !matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "off" | "false" | "0"
        ),
        None => true,
    }
}

pub async fn get_settings(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> Result<Json<AdminSettingsResponse>, ApiError> {
    let mut connection = state.db.acquire().await?;
    let goal = daily_streak_goal(&mut connection).await?;

    Ok(Json(AdminSettingsResponse {
        daily_streak_goal: goal,
        min: MIN_DAILY_STREAK_GOAL,
        max: MAX_DAILY_STREAK_GOAL,
    }))
}

pub async fn update_settings(
    State(state): State<AppState>,
    admin: AdminUser,
    Json(payload): Json<UpdateSettingsRequest>,
) -> Result<Json<AdminSettingsResponse>, ApiError> {
    if !(MIN_DAILY_STREAK_GOAL..=MAX_DAILY_STREAK_GOAL).contains(&payload.daily_streak_goal) {
        return Err(ApiError::bad_request(format!(
            "daily streak goal must be between {MIN_DAILY_STREAK_GOAL} and {MAX_DAILY_STREAK_GOAL}"
        )));
    }

    let mut connection = state.db.acquire().await?;

    sqlx::query(
        "INSERT INTO app_settings (key, value, updated_at) \
         VALUES ($1, $2, now()) \
         ON CONFLICT (key) DO UPDATE SET \
             value = EXCLUDED.value, updated_at = now()",
    )
    .bind(DAILY_STREAK_GOAL_KEY)
    .bind(payload.daily_streak_goal.to_string())
    .execute(&mut *connection)
    .await?;

    tracing::info!(
        admin_id = %admin.0.id,
        daily_streak_goal = payload.daily_streak_goal,
        "daily streak goal updated"
    );

    Ok(Json(AdminSettingsResponse {
        daily_streak_goal: payload.daily_streak_goal,
        min: MIN_DAILY_STREAK_GOAL,
        max: MAX_DAILY_STREAK_GOAL,
    }))
}

#[cfg(test)]
mod tests {
    use super::parse_rank_weighting;

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
}
