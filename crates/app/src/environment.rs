//! Windows values used by the app logic.

use crate::logic::Environment;
use breaks::until_morning;
use settings::Settings;
use tracing::warn;

/// The real OS behind [`Environment`].
pub(crate) struct Windows;

impl Environment for Windows {
    fn full_screen_busy(&self) -> Option<bool> {
        platform::full_screen_busy()
            .inspect_err(|error| warn!(%error, "full-screen state unavailable"))
            .ok()
    }
    fn running_processes(&self) -> Option<Vec<String>> {
        platform::running_process_names()
            .inspect_err(|error| warn!(%error, "process list unavailable"))
            .ok()
    }

    fn until_morning(&self, settings: &Settings) -> Option<i64> {
        until_morning(
            &jiff::Zoned::now(),
            &settings.morning_hour,
            settings.pos_latitude,
            settings.pos_longitude,
        )
    }

    fn day(&self) -> String {
        use jiff::ToSpan;
        // Civil time, so the day still starts at 04:00 on a daylight-saving change.
        jiff::Zoned::now()
            .datetime()
            .checked_sub(4.hours())
            .map(|time| time.date().to_string())
            .unwrap_or_default()
    }

    fn wall_ms(&self) -> i64 {
        jiff::Timestamp::now().as_millisecond()
    }

    fn until_day_boundary(&self) -> i64 {
        use jiff::ToSpan;
        let now = jiff::Zoned::now();
        let boundary = now
            .with()
            .hour(4)
            .minute(0)
            .second(0)
            .millisecond(0)
            .build();
        boundary
            .ok()
            .and_then(|boundary| {
                if boundary <= now {
                    boundary.checked_add(1.day()).ok()
                } else {
                    Some(boundary)
                }
            })
            .map_or(1_000, |boundary| {
                boundary.timestamp().as_millisecond() - now.timestamp().as_millisecond()
            })
    }
}
