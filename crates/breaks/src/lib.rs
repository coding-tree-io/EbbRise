//! Break timing (the planner), pause-until-morning, and the pure rules the UI and tray use.
//! Platform-free: the caller passes the time in and carries out the returned events.

mod morning;
mod planner;
mod rules;

pub use morning::until_morning;
pub use planner::{BreaksPlanner, Event};
pub use rules::{
    Display, Rect, allowed_audio_filename, allowed_image_filename, can_postpone, can_skip,
    exclusion_blocks, format_time_remaining, target_display, tray_icon_for_minutes,
    window_position,
};
pub use settings::{AppExclusion, MorningHour, Settings};
