use std::collections::{BTreeMap, HashSet};
use std::sync::OnceLock;

use chrono::{Duration, NaiveDate};
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, Postgres, QueryBuilder};
use uuid::Uuid;

use crate::api::settings::daily_streak_goal;
use crate::error::ApiError;

/// The last seven days are returned for the streak strip.
const WEEK_DAYS: i64 = 7;

/// IANA names are static for the life of the process. Loading them once avoids
/// a 35–79 ms `pg_timezone_names` scan on every matchup/summary request.
static TIMEZONE_NAMES: OnceLock<HashSet<String>> = OnceLock::new();

#[derive(Debug, Default, Deserialize)]
pub struct StreakQuery {
    pub tz: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StreakSummary {
    pub current: i64,
    pub longest: i64,
    pub today_picks: i64,
    pub goal: i64,
    pub today_met: bool,
    pub at_risk: bool,
    pub days: Vec<StreakDay>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StreakDay {
    pub date: String,
    pub picks: i64,
    pub met: bool,
    pub is_today: bool,
}

#[derive(Debug, sqlx::FromRow)]
struct DayRow {
    day: NaiveDate,
    picks: i64,
}

/// Compute the current daily streak from the voter's picks.
///
/// The streak is app-wide, not playlist-specific, and uses the caller's local
/// calendar (an IANA timezone from the browser). If no valid timezone is given,
/// UTC is used.
pub async fn load_streak(
    connection: &mut PgConnection,
    user_id: Uuid,
    requested_tz: Option<&str>,
) -> Result<StreakSummary, ApiError> {
    let timezone = resolve_timezone(connection, requested_tz).await?;

    let today = sqlx::query_scalar::<_, NaiveDate>("SELECT (now() AT TIME ZONE $1)::date")
        .bind(&timezone)
        .fetch_one(&mut *connection)
        .await?;

    let mut builder: QueryBuilder<Postgres> =
        QueryBuilder::new("SELECT (v.created_at AT TIME ZONE ");
    builder.push_bind(&timezone);
    builder.push(
        ")::date AS day, COUNT(*) AS picks \
         FROM votes v \
         WHERE v.user_id = ",
    );
    builder.push_bind(user_id);
    builder.push(" GROUP BY day ORDER BY day DESC");

    let rows = builder
        .build_query_as::<DayRow>()
        .fetch_all(&mut *connection)
        .await?;

    let counts: BTreeMap<NaiveDate, i64> =
        rows.into_iter().map(|row| (row.day, row.picks)).collect();

    let goal = daily_streak_goal(connection).await?;
    let today_picks = counts.get(&today).copied().unwrap_or(0);
    let today_met = today_picks >= goal;

    let yesterday = today.checked_sub_signed(Duration::days(1));
    let yesterday_met = yesterday
        .and_then(|day| counts.get(&day).copied())
        .unwrap_or(0)
        >= goal;

    let mut current = 0;
    let mut cursor = if today_met {
        Some(today)
    } else if yesterday_met {
        yesterday
    } else {
        None
    };

    while let Some(day) = cursor {
        if counts.get(&day).copied().unwrap_or(0) < goal {
            break;
        }
        current += 1;
        cursor = day.checked_sub_signed(Duration::days(1));
    }

    let mut met_days: Vec<NaiveDate> = counts
        .iter()
        .filter(|(_, picks)| **picks >= goal)
        .map(|(day, _)| *day)
        .collect();
    met_days.sort_unstable();

    let mut longest = 0;
    let mut run = 0;
    let mut previous: Option<NaiveDate> = None;

    for day in met_days {
        if let Some(prev) = previous {
            if prev.checked_add_signed(Duration::days(1)) == Some(day) {
                run += 1;
            } else {
                run = 1;
            }
        } else {
            run = 1;
        }
        longest = longest.max(run);
        previous = Some(day);
    }

    let mut days = Vec::with_capacity(WEEK_DAYS as usize);
    for offset in (0..WEEK_DAYS).rev() {
        let Some(day) = today.checked_sub_signed(Duration::days(offset)) else {
            continue;
        };
        let picks = counts.get(&day).copied().unwrap_or(0);
        days.push(StreakDay {
            date: day.to_string(),
            picks,
            met: picks >= goal,
            is_today: day == today,
        });
    }

    Ok(StreakSummary {
        current,
        longest,
        today_picks,
        goal,
        today_met,
        at_risk: !today_met && current > 0,
        days,
    })
}

pub(crate) async fn resolve_timezone(
    connection: &mut PgConnection,
    requested: Option<&str>,
) -> Result<String, ApiError> {
    let Some(requested) = requested.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok("UTC".to_string());
    };

    // A control character (notably NUL) would reach PostgreSQL as an invalid
    // timezone name and turn a client input into a 500. Fall back to UTC.
    if requested.chars().any(char::is_control) {
        return Ok("UTC".to_string());
    }

    let names = if let Some(names) = TIMEZONE_NAMES.get() {
        names
    } else {
        let rows = sqlx::query_scalar::<_, String>("SELECT name FROM pg_timezone_names")
            .fetch_all(&mut *connection)
            .await?;
        let loaded = rows.into_iter().collect::<HashSet<_>>();
        let _ = TIMEZONE_NAMES.set(loaded);
        TIMEZONE_NAMES.get().expect("timezone names were just set")
    };

    Ok(if names.contains(requested) {
        requested.to_string()
    } else {
        "UTC".to_string()
    })
}
