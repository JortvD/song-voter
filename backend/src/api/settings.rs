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
