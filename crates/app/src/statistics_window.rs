//! The Statistics window: one day (concept D4-A), or a week or month chart (design C2), from
//! `statistics.json` and today's live record, under the shoreline of the last 14 days (P2).

use std::rc::Rc;
use std::time::Duration;

use jiff::ToSpan;
use jiff::civil::Date;
use slint::{ComponentHandle, ModelRc, SharedString, Timer, VecModel};
use tracing::warn;

use crate::logic::{BreakCounts, DayStatistics, day_range};
use crate::settings_window::GROUND;
use crate::statistics_period::{
    Range, SHORE_DAYS, active_time, chart_bar, completed, daily_limit, day_label, period_label,
    shore_state, streak, taken_and_skipped, top_hours, weekday_letter,
};
use crate::ui::{ChartBar, Palette, StatisticsWindow};
use crate::{App, statistics_store, with_app};

const TITLE: &str = "EbbRise statistics";

pub(crate) struct StatisticsView {
    window: StatisticsWindow,
    /// Every recorded day, oldest first; the last one is today.
    days: Vec<DayStatistics>,
    range: Range,
    /// A day in the period shown; in the Day view, a recorded day.
    anchor: Date,
    /// The daily limit, hours; 0 when it is off.
    limit_hours: f32,
}

impl App {
    /// Opens the Statistics window on today, or brings the open one to the front with
    /// today's counts.
    pub(crate) fn open_statistics(&mut self) {
        let days = self.statistics_days();
        let limit_hours = self.daily_limit_hours();
        if let Some(view) = &mut self.statistics_view {
            view.days = days;
            view.anchor = view.today();
            view.limit_hours = limit_hours;
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
        window.on_back(|| with_app(|app| app.move_statistics(-1)));
        window.on_forward(|| with_app(|app| app.move_statistics(1)));
        window.on_range_changed(|index| with_app(move |app| app.change_statistics_range(index)));
        window.on_dark_mode_changed(colour_title_bar);
        window.window().on_close_requested(|| {
            with_app(App::close_statistics);
            slint::CloseRequestResponse::HideWindow
        });
        let mut view = StatisticsView {
            window,
            days,
            range: Range::Day,
            anchor: Date::default(),
            limit_hours,
        };
        view.anchor = view.today();
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

    fn daily_limit_hours(&self) -> f32 {
        let settings = self.core.settings();
        if settings.daily_limit {
            settings.daily_limit_time as f32 / 3_600_000.0
        } else {
            0.0
        }
    }

    /// Frees the window after this event, as the settings window is.
    fn close_statistics(&mut self) {
        if let Some(view) = self.statistics_view.take() {
            Timer::single_shot(Duration::ZERO, move || drop(view));
        }
    }

    fn move_statistics(&mut self, step: i64) {
        if let Some(view) = &mut self.statistics_view {
            view.anchor = match view.range {
                // The Day view moves between recorded days only.
                Range::Day => {
                    let shown = view.shown_index();
                    let next = shown.saturating_add_signed(step as isize);
                    view.date(next.min(view.days.len() - 1))
                }
                range => range.step(view.anchor, step).min(view.today()),
            };
            view.fill();
        }
    }

    fn change_statistics_range(&mut self, index: i32) {
        if let Some(view) = &mut self.statistics_view {
            view.range = Range::from_index(index);
            if view.range == Range::Day {
                view.anchor = view.date(view.shown_index());
            }
            view.fill();
        }
    }
}

impl StatisticsView {
    fn date(&self, index: usize) -> Date {
        self.days[index].day.parse().unwrap_or_default()
    }

    fn today(&self) -> Date {
        self.date(self.days.len() - 1)
    }

    /// The last recorded day on or before the anchor.
    fn shown_index(&self) -> usize {
        let anchor = self.anchor.to_string();
        self.days
            .iter()
            .rposition(|day| day.day <= anchor)
            .unwrap_or(0)
    }

    fn fill(&self) {
        let window = &self.window;
        let today = self.today();
        window.set_range(self.range.index());
        let shore = day_range(
            &self.days,
            today.saturating_sub((SHORE_DAYS - 1).days()),
            today,
        );
        window.set_shore(model(shore.iter().map(|day| shore_state(day) as i32)));
        window.set_shore_letters(model(shore.iter().map(|day| {
            SharedString::from(if day.day == today.to_string() {
                "Today".to_owned()
            } else {
                weekday_letter(&day.day)
            })
        })));
        window.set_streak(streak(&self.days) as i32);
        if self.range == Range::Day {
            return self.fill_day();
        }
        let (first, last) = self.range.period(self.anchor);
        let earliest = self.date(0);
        window.set_day(period_label(self.range, first, last).into());
        window.set_can_go_back(first > earliest);
        window.set_can_go_forward(last < today);
        let days = day_range(&self.days, first, last);
        let measured: Vec<i64> = days.iter().filter_map(|day| day.active_ms).collect();
        window.set_active_time(if measured.is_empty() {
            "Not measured".to_owned().into()
        } else {
            active_time(Some(measured.iter().sum())).into()
        });
        let (taken, skipped) = days
            .iter()
            .map(taken_and_skipped)
            .fold((0, 0), |a, b| (a.0 + b.0, a.1 + b.1));
        window.set_taken_share(
            match taken + skipped {
                0 => "No breaks".to_owned(),
                all => format!("{} %", (taken * 100 + all / 2) / all),
            }
            .into(),
        );
        window.set_skipped(skipped.to_string().into());
        let bars: Vec<ChartBar> = days
            .iter()
            .enumerate()
            .map(|(i, day)| chart_bar(self.range, day, i, days.len()))
            .collect();
        let most = bars
            .iter()
            .map(|bar| bar.hours)
            .fold(self.limit_hours, f32::max);
        window.set_top_hours(top_hours(most));
        window.set_limit_hours(self.limit_hours);
        window.set_bars(ModelRc::from(Rc::new(VecModel::from(bars))));
    }

    fn fill_day(&self) {
        let window = &self.window;
        let shown = self.shown_index();
        let day = &self.days[shown];
        let today = shown + 1 == self.days.len();
        window.set_day(day_label(&day.day, today).into());
        window.set_can_go_back(shown > 0);
        window.set_can_go_forward(!today);
        window.set_active_time(active_time(day.active_ms).into());
        window.set_completed(completed(day).into());
        window.set_mini(counts(&day.mini));
        window.set_long(counts(&day.long));
        window.set_daily_limit(daily_limit(day).into());
    }
}

fn model<T: Clone + 'static>(items: impl Iterator<Item = T>) -> ModelRc<T> {
    ModelRc::from(Rc::new(VecModel::from(items.collect::<Vec<_>>())))
}

fn counts(counts: &BreakCounts) -> ModelRc<i32> {
    let values = [
        counts.shown,
        counts.completed,
        counts.natural,
        counts.skipped,
        counts.postponed,
    ];
    model(values.iter().map(|&n| i32::try_from(n).unwrap_or(i32::MAX)))
}

fn colour_title_bar(dark: bool) {
    if !platform::title_bar(TITLE, dark, GROUND[usize::from(!dark)]) {
        warn!("statistics title bar not coloured");
    }
}
