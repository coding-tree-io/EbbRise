use crate::MorningHour;
use jiff::{ToSpan, Zoned};
use sunrise::{Coordinates, SolarDay, SolarEvent};

/// Milliseconds from `now` until the next morning (upstream `untilMorning.js`).
///
/// The morning is today if its time has not passed yet, else tomorrow. Latitude and
/// longitude, in degrees, are used only for sunrise. Returns `None` when the time cannot be
/// computed, such as an hour outside 0–23 or no sunrise at that place.
pub fn until_morning(
    now: &Zoned,
    morning: &MorningHour,
    latitude: f64,
    longitude: f64,
) -> Option<i64> {
    let mut target = match morning {
        MorningHour::Hour(hour) => now
            .with()
            .hour(i8::try_from(*hour).ok()?)
            .minute(0)
            .second(0)
            .build()
            .ok()?,
        MorningHour::Sunrise(value) if value == "sunrise" => {
            let date = chrono::NaiveDate::from_ymd_opt(
                now.year().into(),
                now.month().try_into().ok()?,
                now.day().try_into().ok()?,
            )?;
            let sunrise = SolarDay::new(Coordinates::new(latitude, longitude)?, date)
                .event_time(SolarEvent::Sunrise)?;
            jiff::Timestamp::from_millisecond(sunrise.timestamp_millis())
                .ok()?
                .to_zoned(now.time_zone().clone())
        }
        _ => return None,
    };
    if now >= target {
        // shortcut: tomorrow uses today's sunrise time, recalculate when minute accuracy matters.
        target = target.checked_add(1.day()).ok()?;
    }
    Some(target.timestamp().as_millisecond() - now.timestamp().as_millisecond())
}
