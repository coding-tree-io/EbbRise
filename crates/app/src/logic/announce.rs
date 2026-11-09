//! The sound and the toast that go with a planner event.

use breaks::Event;
use i18n::Translator;
use settings::Settings;

use super::{SILENCE, Toast};

/// The start sound when a break starts, and the end sound when it ends on time.
pub(super) fn sound(s: &Settings, event: &Event, on_time: bool) -> Option<String> {
    if s.silent_notifications {
        return None;
    }
    let name = match event {
        Event::StartMiniBreak => &s.mini_break_start_sound,
        Event::StartLongBreak => &s.long_break_start_sound,
        Event::FinishMiniBreak if on_time => &s.mini_break_audio,
        Event::FinishLongBreak if on_time => &s.long_break_audio,
        _ => return None,
    };
    (name != SILENCE).then(|| name.clone())
}

/// Upstream shows only the notifications and `resumingBreaks`; breaks get the break window.
pub(super) fn toast(words: &Translator, s: &Settings, event: &Event) -> Option<Toast> {
    let seconds = |interval: i64| (interval / 1000).to_string();
    let (title, text) = match event {
        Event::MiniBreakNotification => (
            words.with(
                "main.miniBreakIn",
                &[("seconds", &seconds(s.mini_break_notification_interval))],
            ),
            String::new(),
        ),
        Event::LongBreakNotification => (
            words.with(
                "main.longBreakIn",
                &[("seconds", &seconds(s.long_break_notification_interval))],
            ),
            String::new(),
        ),
        Event::ResumeBreaks => (words.text("main.resumingBreaks"), String::new()),
        Event::StartMiniBreak
        | Event::StartLongBreak
        | Event::FinishMiniBreak
        | Event::FinishLongBreak
        | Event::AwaitMiniBreakFinish
        | Event::AwaitLongBreakFinish
        | Event::MiniBreakTakenNaturally
        | Event::LongBreakTakenNaturally => return None,
    };
    Some(Toast { title, text })
}
