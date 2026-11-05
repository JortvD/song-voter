use sqlx::PgPool;
use uuid::Uuid;

/// One row in the `auth_events` audit trail.
pub struct AuthEvent<'a> {
    pub user_id: Option<Uuid>,
    pub username: Option<&'a str>,
    pub event: &'a str,
    pub detail: Option<&'a str>,
    pub ip: Option<&'a str>,
    pub user_agent: Option<&'a str>,
}

/// Persists an audit event. Audit failures are logged but never fail the
/// request that triggered them.
pub async fn record(pool: &PgPool, event: AuthEvent<'_>) {
    let result = sqlx::query(
        "INSERT INTO auth_events (user_id, username, event, detail, ip, user_agent) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(event.user_id)
    .bind(event.username)
    .bind(event.event)
    .bind(event.detail)
    .bind(event.ip)
    .bind(event.user_agent)
    .execute(pool)
    .await;

    if let Err(error) = result {
        tracing::error!(
            %error,
            event = event.event,
            user_id = ?event.user_id,
            "failed to write auth audit event"
        );
    }
}
