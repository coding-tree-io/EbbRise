//! `%APPDATA%\EbbRise\statistics.json`: one record per day, oldest first, for the last 365 days.

use std::path::Path;

use crate::logic::DayStatistics;

const DAYS_KEPT: usize = 365;

/// The saved records; empty when the file is missing or unreadable.
pub(crate) fn load(path: &Path) -> Vec<DayStatistics> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    serde_json::from_str(&text).unwrap_or_else(|error| {
        tracing::warn!(%error, "statistics unreadable");
        Vec::new()
    })
}

/// Puts `records` into the file, replacing the saved record of the same day.
pub(crate) fn save(path: &Path, records: &[DayStatistics]) {
    if records.is_empty() {
        return;
    }
    let merged = merge(load(path), records);
    let result = serde_json::to_vec_pretty(&merged)
        .map_err(std::io::Error::from)
        .and_then(|bytes| settings::write_atomically(path, &bytes));
    if let Err(error) = result {
        tracing::warn!(%error, "statistics not saved");
    }
}

fn merge(mut saved: Vec<DayStatistics>, records: &[DayStatistics]) -> Vec<DayStatistics> {
    for record in records {
        saved.retain(|old| old.day != record.day);
        saved.push(record.clone());
    }
    saved.sort_by(|a, b| a.day.cmp(&b.day));
    let surplus = saved.len().saturating_sub(DAYS_KEPT);
    saved.drain(..surplus);
    saved
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(day: &str, shown: u32) -> DayStatistics {
        let mut record = DayStatistics {
            day: day.to_owned(),
            ..DayStatistics::default()
        };
        record.mini.shown = shown;
        record
    }

    #[test]
    fn merge_replaces_the_same_day_sorts_and_keeps_a_year() {
        let merged = merge(
            vec![day("2026-11-08", 1), day("2026-11-09", 2)],
            &[day("2026-11-09", 5), day("2026-11-07", 3)],
        );
        let shown: Vec<_> = merged
            .iter()
            .map(|r| (r.day.as_str(), r.mini.shown))
            .collect();
        assert_eq!(
            shown,
            [("2026-11-07", 3), ("2026-11-08", 1), ("2026-11-09", 5)]
        );

        let year: Vec<_> = (0..400).map(|n| day(&format!("d{n:03}"), 0)).collect();
        let kept = merge(year, &[]);
        assert_eq!(kept.len(), DAYS_KEPT);
        assert_eq!(kept[0].day, "d035");
    }

    #[test]
    fn save_writes_a_file_that_load_reads_back() {
        let path = std::env::temp_dir().join(format!("ebbrise-stats-{}.json", std::process::id()));
        save(&path, &[day("2026-11-09", 4)]);
        save(&path, &[day("2026-11-10", 1)]);
        let loaded = load(&path);
        std::fs::remove_file(&path).unwrap();
        assert_eq!(loaded, [day("2026-11-09", 4), day("2026-11-10", 1)]);
    }
}
