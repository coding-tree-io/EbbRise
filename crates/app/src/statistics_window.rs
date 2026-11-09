//! The Statistics window (concept D4-A): one day at a time, from `statistics.json` and today's
//! live record.

use std::rc::Rc;
use std::time::Duration;

use slint::{ComponentHandle, ModelRc, Timer, VecModel};
use tracing::warn;

use crate::logic::{BreakCounts, DayStatistics};
use crate::settings_window::GROUND;
use crate::ui::{Palette, StatisticsWindow};
use crate::{App, statistics_store, with_app};

const TITLE: &str = "EbbRise statistics";

pub(crate) struct StatisticsView {
    window: StatisticsWindow,
    /// Every recorded day, oldest first; the last one is today.
    days: Vec<DayStatistics>,
    shown: usize,
}

impl App {
    /// Opens the Statistics window on today, or brings the open one to the front with
    /// today's counts.
    pub(crate) fn open_statistics(&mut self) {
        let days = self.statistics_days();
        if let Some(view) = &mut self.statistics_view {
            view.shown = days.len() - 1;
            view.days = days;
            view.fill();
            let _ = view.window.show();
            return;
        }
        let window = match StatisticsWindow::new() {
            Ok(window) => window,
            Err(error) => return warn!(%error, "statistics window failed"),
        };
        window
            .global::<Palette>()
            .set_theme(crate::ui::theme(self.core.settings()));
        window.on_back(|| with_app(|app| app.move_statistics_day(-1)));
        window.on_forward(|| with_app(|app| app.move_statistics_day(1)));
        window.on_dark_mode_changed(colour_title_bar);
        window.window().on_close_requested(|| {
            with_app(App::close_statistics);
            slint::CloseRequestResponse::HideWindow
        });
        let view = StatisticsView {
            shown: days.len() - 1,
            window,
            days,
        };
        view.fill();
        if let Err(error) = view.window.show() {
            warn!(%error, "statistics window not shown");
        }
        let dark = view.window.get_dark_mode();
        Timer::single_shot(Duration::from_millis(50), move || colour_title_bar(dark));
        self.statistics_view = Some(view);
    }

    /// The saved days before today, then today's live record.
    fn statistics_days(&self) -> Vec<DayStatistics> {
        let today = self.core.today_statistics(platform::awake_ms());
        let mut days = self
            .statistics_path()
            .map(|path| statistics_store::load(&path))
            .unwrap_or_default();
        days.retain(|day| day.day < today.day);
        days.push(today);
        days
    }

    /// Frees the window after this event, as the settings window is.
    fn close_statistics(&mut self) {
        if let Some(view) = self.statistics_view.take() {
            Timer::single_shot(Duration::ZERO, move || drop(view));
        }
    }

    fn move_statistics_day(&mut self, step: isize) {
        if let Some(view) = &mut self.statistics_view {
            view.shown = view
                .shown
                .saturating_add_signed(step)
                .min(view.days.len() - 1);
            view.fill();
        }
    }
}

impl StatisticsView {
    fn fill(&self) {
        let window = &self.window;
        let day = &self.days[self.shown];
        let today = self.shown + 1 == self.days.len();
        window.set_day(day_label(&day.day, today).into());
        window.set_can_go_back(self.shown > 0);
        window.set_can_go_forward(!today);
        window.set_active_time(active_time(day.active_ms).into());
        window.set_completed(completed(day).into());
        window.set_mini(counts(&day.mini));
        window.set_long(counts(&day.long));
        window.set_daily_limit(daily_limit(day).into());
    }
}

fn counts(counts: &BreakCounts) -> ModelRc<i32> {
    let values = [
        counts.shown,
        counts.completed,
        counts.natural,
        counts.skipped,
        counts.postponed,
    ];
    let values: Vec<i32> = values
        .iter()
        .map(|&n| i32::try_from(n).unwrap_or(i32::MAX))
        .collect();
    ModelRc::from(Rc::new(VecModel::from(values)))
}

/// "Today", or the date such as "Sun 8 Nov 2026".
fn day_label(day: &str, today: bool) -> String {
    if today {
        return "Today".to_owned();
    }
    day.parse::<jiff::civil::Date>()
        .map(|date| date.strftime("%a %-d %b %Y").to_string())
        .unwrap_or_else(|_| day.to_owned())
}

/// "5 h 12 min", or "Not measured" when neither active-time feature was on.
fn active_time(active_ms: Option<i64>) -> String {
    let Some(ms) = active_ms else {
        return "Not measured".to_owned();
    };
    let minutes = ms / 60_000;
    match (minutes / 60, minutes % 60) {
        (0, minutes) => format!("{minutes} min"),
        (hours, minutes) => format!("{hours} h {minutes} min"),
    }
}

/// "14 of 18": completed breaks of the breaks shown.
fn completed(day: &DayStatistics) -> String {
    let shown = day.mini.shown + day.long.shown;
    let done = day.mini.completed + day.long.completed;
    format!("{done} of {shown}")
}

/// The daily-limit line; empty when no reminder came.
fn daily_limit(day: &DayStatistics) -> String {
    match (day.daily_limit_shown, day.daily_limit_stopped) {
        (0, _) => String::new(),
        (1, false) => "Daily limit: 1 reminder.".to_owned(),
        (1, true) => "Daily limit: 1 reminder; you stopped for the day.".to_owned(),
        (n, false) => format!("Daily limit: {n} reminders."),
        (n, true) => format!("Daily limit: {n} reminders; you stopped for the day."),
    }
}

fn colour_title_bar(dark: bool) {
    if !platform::title_bar(TITLE, dark, GROUND[usize::from(!dark)]) {
        warn!("statistics title bar not coloured");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_read_naturally() {
        assert_eq!(day_label("2026-11-08", false), "Sun 8 Nov 2026");
        assert_eq!(day_label("2026-11-09", true), "Today");
        assert_eq!(active_time(None), "Not measured");
        assert_eq!(active_time(Some(18_720_000)), "5 h 12 min");
        assert_eq!(active_time(Some(59_000)), "0 min");
        let mut day = DayStatistics::default();
        day.mini.shown = 15;
        day.mini.completed = 12;
        day.long.shown = 3;
        day.long.completed = 2;
        assert_eq!(completed(&day), "14 of 18");
        assert_eq!(daily_limit(&day), "");
        day.daily_limit_shown = 2;
        day.daily_limit_stopped = true;
        assert_eq!(
            daily_limit(&day),
            "Daily limit: 2 reminders; you stopped for the day."
        );
    }
}
