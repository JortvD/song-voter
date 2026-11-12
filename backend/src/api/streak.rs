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

    Ok(streak_summary(&counts, today, goal))
}

/// Pure streak calculation, separated from the database so the calendar rules
/// can be tested exhaustively.
fn streak_summary(counts: &BTreeMap<NaiveDate, i64>, today: NaiveDate, goal: i64) -> StreakSummary {
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

    StreakSummary {
        current,
        longest,
        today_picks,
        goal,
        today_met,
        at_risk: !today_met && current > 0,
        days,
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn day(offset: i64) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 1, 15)
            .unwrap()
            .checked_add_signed(Duration::days(offset))
            .unwrap()
    }

    fn counts(entries: &[(i64, i64)]) -> BTreeMap<NaiveDate, i64> {
        entries
            .iter()
            .map(|(offset, picks)| (day(*offset), *picks))
            .collect()
    }

    #[test]
    fn empty_history_has_no_streak() {
        let summary = streak_summary(&BTreeMap::new(), day(0), 3);
        assert_eq!(summary.current, 0);
        assert_eq!(summary.longest, 0);
        assert_eq!(summary.today_picks, 0);
        assert!(!summary.today_met);
        assert!(!summary.at_risk);
    }

    #[test]
    fn meeting_today_starts_a_one_day_streak() {
        let summary = streak_summary(&counts(&[(0, 3)]), day(0), 3);
        assert_eq!(summary.current, 1);
        assert_eq!(summary.longest, 1);
        assert!(summary.today_met);
        assert!(!summary.at_risk);
    }

    #[test]
    fn exceeding_the_goal_still_counts_as_met() {
        let summary = streak_summary(&counts(&[(0, 30)]), day(0), 3);
        assert!(summary.today_met);
        assert_eq!(summary.today_picks, 30);
        assert_eq!(summary.current, 1);
    }

    #[test]
    fn one_pick_short_does_not_meet_the_goal() {
        let summary = streak_summary(&counts(&[(0, 2)]), day(0), 3);
        assert!(!summary.today_met);
        assert_eq!(summary.current, 0);
        assert_eq!(summary.longest, 0);
    }

    #[test]
    fn yesterday_only_is_at_risk_but_preserves_the_streak() {
        let summary = streak_summary(&counts(&[(-1, 3)]), day(0), 3);
        assert_eq!(summary.current, 1);
        assert!(summary.at_risk);
        assert!(!summary.today_met);
    }

    #[test]
    fn missing_today_and_yesterday_resets_the_streak() {
        let summary = streak_summary(&counts(&[(-2, 5), (-3, 4)]), day(0), 3);
        assert_eq!(summary.current, 0);
        assert!(!summary.at_risk);
        assert_eq!(summary.longest, 2);
    }

    #[test]
    fn consecutive_days_build_the_current_streak() {
        let summary = streak_summary(
            &counts(&[(-4, 3), (-3, 3), (-2, 3), (-1, 3), (0, 3)]),
            day(0),
            3,
        );
        assert_eq!(summary.current, 5);
        assert_eq!(summary.longest, 5);
    }

    #[test]
    fn a_gap_in_the_middle_breaks_the_current_streak() {
        let summary = streak_summary(
            &counts(&[(-5, 3), (-4, 3), (-3, 0), (-2, 3), (-1, 3), (0, 3)]),
            day(0),
            3,
        );
        assert_eq!(summary.current, 3);
        assert_eq!(summary.longest, 3);
    }

    #[test]
    fn longest_streak_can_be_longer_than_the_current_one() {
        let summary = streak_summary(
            &counts(&[(-10, 3), (-9, 3), (-8, 3), (-7, 3), (0, 3)]),
            day(0),
            3,
        );
        assert_eq!(summary.current, 1);
        assert_eq!(summary.longest, 4);
    }

    #[test]
    fn today_met_connects_to_yesterday() {
        let summary = streak_summary(&counts(&[(-1, 3), (0, 3)]), day(0), 3);
        assert_eq!(summary.current, 2);
        assert_eq!(summary.longest, 2);
        assert!(!summary.at_risk);
    }

    #[test]
    fn day_strip_is_always_seven_days_oldest_first() {
        let summary = streak_summary(&BTreeMap::new(), day(0), 3);
        assert_eq!(summary.days.len(), 7);
        assert_eq!(summary.days[0].date, day(-6).to_string());
        assert_eq!(summary.days[6].date, day(0).to_string());
        assert!(summary.days[6].is_today);
        assert!(summary.days[..6].iter().all(|entry| !entry.is_today));
    }

    #[test]
    fn day_strip_reports_picks_and_goal_state() {
        let summary = streak_summary(&counts(&[(-1, 4), (0, 1)]), day(0), 3);
        let yesterday = &summary.days[5];
        let today = &summary.days[6];
        assert_eq!(yesterday.picks, 4);
        assert!(yesterday.met);
        assert_eq!(today.picks, 1);
        assert!(!today.met);
    }

    #[test]
    fn day_strip_uses_the_configured_goal() {
        let summary = streak_summary(&counts(&[(0, 5)]), day(0), 10);
        assert!(!summary.today_met);
        assert!(!summary.days[6].met);
        assert_eq!(summary.goal, 10);
    }

    #[test]
    fn future_days_do_not_affect_the_current_streak() {
        let summary = streak_summary(&counts(&[(0, 3), (1, 3), (2, 3)]), day(0), 3);
        assert_eq!(summary.current, 1);
    }

    #[test]
    fn longest_streak_includes_future_days_if_present() {
        // The SQL query only returns past votes in practice, but the pure
        // helper should remain deterministic for any calendar map.
        let summary = streak_summary(&counts(&[(0, 3), (1, 3), (2, 3)]), day(0), 3);
        assert_eq!(summary.longest, 3);
    }

    #[test]
    fn goal_of_one_accepts_a_single_pick() {
        let summary = streak_summary(&counts(&[(0, 1)]), day(0), 1);
        assert!(summary.today_met);
        assert_eq!(summary.current, 1);
    }

    #[test]
    fn zero_pick_rows_still_appear_in_the_strip() {
        let summary = streak_summary(&counts(&[(-2, 0)]), day(0), 3);
        assert_eq!(summary.days[4].picks, 0);
        assert!(!summary.days[4].met);
    }

    #[test]
    fn streak_summary_preserves_the_goal() {
        for goal in [1, 3, 7, 100] {
            assert_eq!(streak_summary(&BTreeMap::new(), day(0), goal).goal, goal);
        }
    }
}
