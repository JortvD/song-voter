use axum::extract::State;
use axum::http::HeaderMap;
use axum::Json;
use uuid::Uuid;

use crate::audit::{self, AuthEvent};
use crate::auth::{client_info, AdminUser};
use crate::error::{is_unique_violation, ApiError};
use crate::models::Invite;
use crate::state::AppState;

pub async fn create_invite(
    State(state): State<AppState>,
    admin: AdminUser,
    headers: HeaderMap,
) -> Result<Json<Invite>, ApiError> {
    let (ip, user_agent) = client_info(&headers);

    for attempt in 1..=5 {
        let code = generate_invite_code();

        let inserted = sqlx::query_as::<_, Invite>(
            "INSERT INTO invites (code, created_by) VALUES ($1, $2) \
             RETURNING id, code, created_at, used_at, expires_at, NULL::text AS used_by_username",
        )
        .bind(&code)
        .bind(admin.0.id)
        .fetch_one(&state.db)
        .await;

        match inserted {
            Ok(invite) => {
                audit::record(
                    &state.db,
                    AuthEvent {
                        user_id: Some(admin.0.id),
                        username: Some(&admin.0.username),
                        event: "invite_created",
                        detail: Some(&invite.code),
                        ip: ip.as_deref(),
                        user_agent: user_agent.as_deref(),
                    },
                )
                .await;

                tracing::info!(
                    admin_id = %admin.0.id,
                    invite_id = %invite.id,
                    code = %invite.code,
                    "invite created"
                );

                return Ok(Json(invite));
            }
            Err(error) if is_unique_violation(&error) => {
                tracing::warn!(code = %code, attempt, "invite code collision, retrying");
            }
            Err(error) => return Err(error.into()),
        }
    }

    Err(ApiError::internal(anyhow::anyhow!(
        "failed to generate a unique invite code after 5 attempts"
    )))
}

pub async fn list_invites(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> Result<Json<Vec<Invite>>, ApiError> {
    let invites = sqlx::query_as::<_, Invite>(
        "SELECT i.id, i.code, i.created_at, i.used_at, i.expires_at, \
                u.username AS used_by_username \
         FROM invites i \
         LEFT JOIN users u ON u.id = i.used_by \
         ORDER BY i.created_at DESC \
         LIMIT 200",
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(invites))
}

/// 10 uppercase hex characters from a random UUID (~40 bits of entropy).
fn generate_invite_code() -> String {
    Uuid::new_v4().simple().to_string()[..10].to_ascii_uppercase()
}
