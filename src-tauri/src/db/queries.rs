use rusqlite::{params, Connection, Result};
use serde::Serialize;

// ---------------------------------------------------------------------------
// Session CRUD (DATA-03)
// ---------------------------------------------------------------------------

/// Inserts a new session row when a round begins.
/// Returns the row ID so it can be passed to `complete_session` later.
pub fn insert_session(
    conn: &Connection,
    round_type: &str,
    duration_secs: u32,
    plant_id: Option<&str>,
) -> Result<i64> {
    let started_at = unix_now();
    conn.execute(
        "INSERT INTO sessions (started_at, round_type, duration_secs, completed, plant_id)
         VALUES (?1, ?2, ?3, 0, ?4)",
        params![started_at, round_type, duration_secs, plant_id],
    )?;
    let id = conn.last_insert_rowid();
    log::debug!("[db] session started: id={id} type={round_type} duration={duration_secs}s");
    Ok(id)
}

/// Updates a session when the round ends (by completion or skip).
pub fn complete_session(conn: &Connection, session_id: i64, completed: bool) -> Result<()> {
    let plant: Option<(String, u32)> = conn
        .query_row(
            "SELECT plant_id, duration_secs FROM sessions WHERE id = ?1 AND plant_id IS NOT NULL",
            [session_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .ok();
    let definition = plant
        .as_ref()
        .and_then(|(id, _)| crate::plants::find(conn, id).ok().flatten());
    let growth_stage = if completed {
        plant.as_ref().and_then(|(_, duration)| {
            definition
                .as_ref()
                .and_then(|plant| crate::plants::growth_stage(plant, *duration))
        })
    } else {
        None
    };
    let plant_name = growth_stage.and_then(|_| definition.as_ref().map(|plant| plant.name.clone()));
    let plant_accent =
        growth_stage.and_then(|_| definition.as_ref().map(|plant| plant.accent.clone()));
    let icon_path = growth_stage.and_then(|stage| {
        definition
            .as_ref()
            .and_then(|plant| crate::plants::stage_icon_path(plant, stage))
    });
    conn.execute(
        "UPDATE sessions
         SET ended_at = ?1, completed = ?2, growth_stage = ?3,
             plant_name = ?4, plant_accent = ?5, plant_icon_path = ?6
         WHERE id = ?7",
        params![
            unix_now(),
            completed as i64,
            growth_stage,
            plant_name,
            plant_accent,
            icon_path,
            session_id
        ],
    )?;
    log::debug!("[db] session ended: id={session_id} completed={completed}");
    Ok(())
}

pub fn update_session_duration(
    conn: &Connection,
    session_id: i64,
    duration_secs: u32,
    plant_id: Option<&str>,
) -> Result<()> {
    conn.execute(
        "UPDATE sessions
         SET duration_secs = ?1, plant_id = ?2
         WHERE id = ?3 AND completed = 0",
        params![duration_secs, plant_id, session_id],
    )?;
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct ForestEntry {
    pub session_id: i64,
    pub plant_id: String,
    pub growth_stage: String,
    pub duration_secs: u32,
    pub planted_at: i64,
    pub local_date: String,
    pub plant_name: Option<String>,
    pub plant_accent: Option<String>,
    pub icon_path: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ForestData {
    pub period: String,
    pub start_date: String,
    pub end_date: String,
    pub total_focus_secs: u32,
    pub entries: Vec<ForestEntry>,
}

/// Returns completed focus plants in a local-calendar day, week, or month.
pub fn get_forest(conn: &Connection, period: &str, anchor: Option<&str>) -> Result<ForestData> {
    let anchor = anchor.unwrap_or("now");
    let (start_modifier, end_modifier) = match period {
        "day" => ("start of day", "+1 day"),
        "week" => ("weekday 0", "+7 days"),
        "month" => ("start of month", "+1 month"),
        _ => return Err(rusqlite::Error::InvalidParameterName(period.to_string())),
    };

    let start_date: String = if period == "week" {
        // Move to the coming Sunday, then step back to Monday of this week.
        conn.query_row("SELECT date(?1, 'weekday 0', '-6 days')", [anchor], |row| {
            row.get(0)
        })?
    } else {
        conn.query_row(
            "SELECT date(?1, ?2)",
            params![anchor, start_modifier],
            |row| row.get(0),
        )?
    };
    let end_date: String = conn.query_row(
        "SELECT date(?1, ?2)",
        params![&start_date, end_modifier],
        |row| row.get(0),
    )?;

    let mut stmt = conn.prepare(
        "SELECT id, plant_id, growth_stage, duration_secs, ended_at,
                date(ended_at, 'unixepoch', 'localtime'),
                plant_name, plant_accent, plant_icon_path
         FROM sessions
         WHERE round_type = 'work' AND completed = 1
           AND plant_id IS NOT NULL AND growth_stage IS NOT NULL
           AND date(ended_at, 'unixepoch', 'localtime') >= ?1
           AND date(ended_at, 'unixepoch', 'localtime') < ?2
         ORDER BY ended_at",
    )?;
    let entries: Vec<ForestEntry> = stmt
        .query_map(params![&start_date, &end_date], |row| {
            Ok(ForestEntry {
                session_id: row.get(0)?,
                plant_id: row.get(1)?,
                growth_stage: row.get(2)?,
                duration_secs: row.get(3)?,
                planted_at: row.get(4)?,
                local_date: row.get(5)?,
                plant_name: row.get(6)?,
                plant_accent: row.get(7)?,
                icon_path: row.get(8)?,
            })
        })?
        .collect::<Result<Vec<_>>>()?;
    let total_focus_secs = conn.query_row(
        "SELECT COALESCE(SUM(duration_secs), 0)
         FROM sessions
         WHERE round_type = 'work' AND completed = 1
           AND date(ended_at, 'unixepoch', 'localtime') >= ?1
           AND date(ended_at, 'unixepoch', 'localtime') < ?2",
        params![&start_date, &end_date],
        |row| row.get(0),
    )?;

    Ok(ForestData {
        period: period.to_string(),
        start_date,
        end_date,
        total_focus_secs,
        entries,
    })
}

// ---------------------------------------------------------------------------
// Stats queries
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct SessionStats {
    pub total_work_sessions: i64,
    pub completed_work_sessions: i64,
    /// Sum of duration_secs for all *completed* work sessions.
    pub total_work_secs: i64,
}

pub fn get_all_time_stats(conn: &Connection) -> Result<SessionStats> {
    let total_work_sessions: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sessions WHERE round_type = 'work'",
        [],
        |r| r.get(0),
    )?;

    let completed_work_sessions: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sessions WHERE round_type = 'work' AND completed = 1",
        [],
        |r| r.get(0),
    )?;

    let total_work_secs: i64 = conn.query_row(
        "SELECT COALESCE(SUM(duration_secs), 0)
         FROM sessions WHERE round_type = 'work' AND completed = 1",
        [],
        |r| r.get(0),
    )?;

    Ok(SessionStats {
        total_work_sessions,
        completed_work_sessions,
        total_work_secs,
    })
}

// ---------------------------------------------------------------------------
// Detailed stats queries (DATA-04)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct DailyStats {
    pub rounds: u32,
    pub focus_mins: u32,
    /// None when no work sessions were started today (avoids 0/0).
    pub completion_rate: Option<f32>,
    /// Completed work rounds per hour of the day (index 0 = midnight).
    pub by_hour: Vec<u32>,
}

#[derive(Debug, Serialize)]
pub struct DayStat {
    /// Local calendar date in "YYYY-MM-DD" format.
    pub date: String,
    pub rounds: u32,
}

#[derive(Debug, Serialize)]
pub struct HeatmapEntry {
    /// Local calendar date in "YYYY-MM-DD" format.
    pub date: String,
    pub count: u32,
}

#[derive(Debug, Serialize)]
pub struct StreakInfo {
    pub current: u32,
    pub longest: u32,
}

/// Completed work rounds and focus time for today (local calendar date).
pub fn get_daily_stats(conn: &Connection) -> Result<DailyStats> {
    let today: String = conn.query_row("SELECT date('now', 'localtime')", [], |r| r.get(0))?;

    let total: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sessions
         WHERE round_type = 'work'
         AND date(started_at, 'unixepoch', 'localtime') = ?1",
        [&today],
        |r| r.get(0),
    )?;

    let completed: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sessions
         WHERE round_type = 'work' AND completed = 1
         AND date(started_at, 'unixepoch', 'localtime') = ?1",
        [&today],
        |r| r.get(0),
    )?;

    let focus_secs: i64 = conn.query_row(
        "SELECT COALESCE(SUM(duration_secs), 0) FROM sessions
         WHERE round_type = 'work' AND completed = 1
         AND date(started_at, 'unixepoch', 'localtime') = ?1",
        [&today],
        |r| r.get(0),
    )?;

    let mut by_hour = vec![0u32; 24];
    let mut stmt = conn.prepare(
        "SELECT CAST(strftime('%H', datetime(started_at, 'unixepoch', 'localtime')) AS INTEGER) as h,
                COUNT(*) as cnt
         FROM sessions
         WHERE round_type = 'work' AND completed = 1
         AND date(started_at, 'unixepoch', 'localtime') = ?1
         GROUP BY h",
    )?;
    let rows = stmt.query_map([&today], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, u32>(1)?)))?;
    for row in rows.flatten() {
        let (h, cnt) = row;
        if (0..24).contains(&h) {
            by_hour[h as usize] = cnt;
        }
    }

    Ok(DailyStats {
        rounds: completed as u32,
        focus_mins: ((focus_secs + 30) / 60) as u32,
        completion_rate: if total > 0 {
            Some(completed as f32 / total as f32)
        } else {
            None
        },
        by_hour,
    })
}

/// Completed work rounds per local calendar day for the last 7 days.
pub fn get_weekly_stats(conn: &Connection) -> Result<Vec<DayStat>> {
    let mut stmt = conn.prepare(
        "SELECT date(started_at, 'unixepoch', 'localtime') as day,
                COUNT(*) as rounds
         FROM sessions
         WHERE round_type = 'work' AND completed = 1
         AND date(started_at, 'unixepoch', 'localtime') >= date('now', 'localtime', '-6 days')
         GROUP BY day
         ORDER BY day",
    )?;
    let rows = stmt
        .query_map([], |r| {
            Ok(DayStat {
                date: r.get(0)?,
                rounds: r.get(1)?,
            })
        })?
        .collect();
    rows
}

/// Completed work rounds per local calendar day, all time (no date limit).
/// The frontend slices this into per-year views for navigation.
pub fn get_heatmap_data(conn: &Connection) -> Result<Vec<HeatmapEntry>> {
    let mut stmt = conn.prepare(
        "SELECT date(started_at, 'unixepoch', 'localtime') as day,
                COUNT(*) as cnt
         FROM sessions
         WHERE round_type = 'work' AND completed = 1
         GROUP BY day
         ORDER BY day",
    )?;
    let rows = stmt
        .query_map([], |r| {
            Ok(HeatmapEntry {
                date: r.get(0)?,
                count: r.get(1)?,
            })
        })?
        .collect();
    rows
}

/// Current and longest work-session streaks (consecutive local calendar days).
/// A streak stays active until midnight: if yesterday had sessions but today does not,
/// the streak is still counted as current.
pub fn get_streak(conn: &Connection) -> Result<StreakInfo> {
    let today: String = conn.query_row("SELECT date('now', 'localtime')", [], |r| r.get(0))?;

    let mut stmt = conn.prepare(
        "SELECT date(started_at, 'unixepoch', 'localtime') as day
         FROM sessions
         WHERE round_type = 'work' AND completed = 1
         GROUP BY day
         ORDER BY day",
    )?;
    let days: Vec<String> = stmt.query_map([], |r| r.get(0))?.flatten().collect();

    Ok(compute_streak(&days, &today))
}

// ---------------------------------------------------------------------------
// Streak helpers
// ---------------------------------------------------------------------------

/// Convert a "YYYY-MM-DD" string to a day number for arithmetic comparison.
/// Uses the proleptic Gregorian calendar; absolute value is arbitrary — only
/// differences between dates matter.
fn date_to_day_num(s: &str) -> Option<i32> {
    let mut parts = s.splitn(3, '-');
    let y: i32 = parts.next()?.parse().ok()?;
    let m: i32 = parts.next()?.parse().ok()?;
    let d: i32 = parts.next()?.parse().ok()?;
    let y = if m <= 2 { y - 1 } else { y };
    let m = if m <= 2 { m + 12 } else { m };
    Some(y * 365 + y / 4 - y / 100 + y / 400 + (153 * m - 457) / 5 + d)
}

pub fn compute_streak(days: &[String], today: &str) -> StreakInfo {
    let nums: Vec<i32> = days.iter().filter_map(|s| date_to_day_num(s)).collect();
    if nums.is_empty() {
        return StreakInfo {
            current: 0,
            longest: 0,
        };
    }

    let today_n = match date_to_day_num(today) {
        Some(n) => n,
        None => {
            return StreakInfo {
                current: 0,
                longest: 0,
            }
        }
    };

    // Current streak — alive if most recent session day is today or yesterday.
    let last = *nums.last().unwrap();
    let current = if last == today_n || last == today_n - 1 {
        let mut count = 0u32;
        let mut expected = last;
        for &n in nums.iter().rev() {
            if n == expected {
                count += 1;
                expected -= 1;
            } else {
                break;
            }
        }
        count
    } else {
        0
    };

    // Longest streak.
    let mut longest = 1u32;
    let mut run = 1u32;
    for i in 1..nums.len() {
        if nums[i] == nums[i - 1] + 1 {
            run += 1;
            if run > longest {
                longest = run;
            }
        } else {
            run = 1;
        }
    }

    StreakInfo { current, longest }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrations;

    fn setup() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        migrations::run(&conn).unwrap();
        conn
    }

    #[test]
    fn insert_and_complete_session() {
        let conn = setup();
        let id = insert_session(&conn, "work", 1500, None).unwrap();
        assert!(id > 0);

        complete_session(&conn, id, true).unwrap();

        let completed: i64 = conn
            .query_row("SELECT completed FROM sessions WHERE id = ?1", [id], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(completed, 1);
    }

    #[test]
    fn stats_empty_db() {
        let conn = setup();
        let stats = get_all_time_stats(&conn).unwrap();
        assert_eq!(stats.total_work_sessions, 0);
        assert_eq!(stats.completed_work_sessions, 0);
        assert_eq!(stats.total_work_secs, 0);
    }

    #[test]
    fn compute_streak_empty() {
        let info = compute_streak(&[], "2024-03-15");
        assert_eq!(info.current, 0);
        assert_eq!(info.longest, 0);
    }

    #[test]
    fn compute_streak_active_today() {
        let days = vec![
            "2024-03-13".to_string(),
            "2024-03-14".to_string(),
            "2024-03-15".to_string(),
        ];
        let info = compute_streak(&days, "2024-03-15");
        assert_eq!(info.current, 3);
        assert_eq!(info.longest, 3);
    }

    #[test]
    fn compute_streak_active_until_midnight() {
        // Yesterday had sessions, today does not — streak still live.
        let days = vec!["2024-03-13".to_string(), "2024-03-14".to_string()];
        let info = compute_streak(&days, "2024-03-15");
        assert_eq!(info.current, 2);
    }

    #[test]
    fn compute_streak_broken() {
        // Last session was 2 days ago — streak is broken.
        let days = vec!["2024-03-12".to_string(), "2024-03-13".to_string()];
        let info = compute_streak(&days, "2024-03-15");
        assert_eq!(info.current, 0);
    }

    #[test]
    fn compute_streak_longest_across_break() {
        let days = vec![
            "2024-03-01".to_string(),
            "2024-03-02".to_string(),
            "2024-03-03".to_string(),
            "2024-03-10".to_string(),
            "2024-03-11".to_string(),
        ];
        let info = compute_streak(&days, "2024-03-11");
        assert_eq!(info.current, 2);
        assert_eq!(info.longest, 3);
    }

    #[test]
    fn get_daily_stats_empty() {
        let conn = setup();
        let stats = get_daily_stats(&conn).unwrap();
        assert_eq!(stats.rounds, 0);
        assert_eq!(stats.focus_mins, 0);
        assert!(stats.completion_rate.is_none());
        assert_eq!(stats.by_hour.len(), 24);
    }

    #[test]
    fn get_weekly_stats_empty() {
        let conn = setup();
        let stats = get_weekly_stats(&conn).unwrap();
        assert!(stats.is_empty());
    }

    #[test]
    fn get_heatmap_data_empty() {
        let conn = setup();
        let entries = get_heatmap_data(&conn).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn focus_mins_rounds_to_nearest_minute() {
        let conn = setup();

        // 339 s = 5:39 → rounds up to 6 min (remainder 39 ≥ 30).
        let id1 = insert_session(&conn, "work", 339, None).unwrap();
        complete_session(&conn, id1, true).unwrap();
        let stats = get_daily_stats(&conn).unwrap();
        assert_eq!(stats.focus_mins, 6, "339 s should round to 6 min");

        // Reset and test round-down: 324 s = 5:24 → rounds down to 5 min (remainder 24 < 30).
        let conn2 = setup();
        let id2 = insert_session(&conn2, "work", 324, None).unwrap();
        complete_session(&conn2, id2, true).unwrap();
        let stats2 = get_daily_stats(&conn2).unwrap();
        assert_eq!(stats2.focus_mins, 5, "324 s should round to 5 min");

        // Exact minute boundary: 1500 s = 25:00 → stays 25 min.
        let conn3 = setup();
        let id3 = insert_session(&conn3, "work", 1500, None).unwrap();
        complete_session(&conn3, id3, true).unwrap();
        let stats3 = get_daily_stats(&conn3).unwrap();
        assert_eq!(stats3.focus_mins, 25, "1500 s should be exactly 25 min");
    }

    #[test]
    fn stats_counts_correctly() {
        let conn = setup();

        let id1 = insert_session(&conn, "work", 1500, None).unwrap();
        complete_session(&conn, id1, true).unwrap();

        let id2 = insert_session(&conn, "work", 1500, None).unwrap();
        complete_session(&conn, id2, false).unwrap(); // skipped

        let _id3 = insert_session(&conn, "short-break", 300, None).unwrap();

        let stats = get_all_time_stats(&conn).unwrap();
        assert_eq!(stats.total_work_sessions, 2);
        assert_eq!(stats.completed_work_sessions, 1);
        assert_eq!(stats.total_work_secs, 1500);
    }

    #[test]
    fn forest_only_contains_completed_eligible_plants() {
        let conn = setup();

        let planted = insert_session(&conn, "work", 1500, Some("cherry")).unwrap();
        complete_session(&conn, planted, true).unwrap();

        let skipped = insert_session(&conn, "work", 3600, Some("oak")).unwrap();
        complete_session(&conn, skipped, false).unwrap();

        let too_short = insert_session(&conn, "work", 300, Some("oak")).unwrap();
        complete_session(&conn, too_short, true).unwrap();

        let forest = get_forest(&conn, "day", None).unwrap();
        assert_eq!(forest.entries.len(), 1);
        assert_eq!(forest.entries[0].plant_id, "cherry");
        assert_eq!(forest.entries[0].growth_stage, "medium");
        assert_eq!(
            forest.entries[0].plant_name.as_deref(),
            Some("Cherry Blossom")
        );
        assert_eq!(forest.entries[0].plant_accent.as_deref(), Some("#e99aaa"));
        assert_eq!(forest.total_focus_secs, 1500);
    }
}
