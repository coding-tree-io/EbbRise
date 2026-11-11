//! The app side of the features from Workrave (`docs/workrave-features.md`): the daily-limit
//! reminder window, the input block during breaks, and saving the statistics.

use std::path::PathBuf;

use tracing::{info, warn};

use crate::break_window::BreakView;
use crate::logic::{BreakButton, DailyLimitScreen, DayStatistics, Environment};
use crate::storage::app_folder;
use crate::{App, statistics_store, with_app};

/// Today's record from an earlier run, to continue it.
pub(crate) fn saved_today(os: &impl Environment) -> Option<DayStatistics> {
    let path = app_folder()?.join("statistics.json");
    let today = os.day();
    statistics_store::load(&path)
        .into_iter()
        .find(|record| record.day == today)
}

impl App {
    pub(crate) fn statistics_path(&self) -> Option<PathBuf> {
        app_folder().map(|folder| folder.join("statistics.json"))
    }

    /// Saves the records that changed since the last save; keeps them for the next save when
    /// the write fails.
    pub(crate) fn save_statistics(&mut self) {
        let now = platform::awake_ms();
        let records = self.core.statistics_to_save(now);
        if records.is_empty() {
            return;
        }
        let Some(path) = self.statistics_path() else {
            self.core.keep_unsaved_statistics(records);
            return;
        };
        if let Err(error) = statistics_store::save(&path, &records) {
            warn!(%error, "statistics not saved");
            self.core.keep_unsaved_statistics(records);
        } else {
            self.core.statistics_saved(now);
        }
    }

    /// Saves today's record as it is, with the active time (at quit).
    pub(crate) fn save_today_statistics(&mut self) {
        self.core.prepare_final_statistics(
            platform::awake_ms(),
            platform::idle_ms(),
            &crate::environment::Windows,
        );
        self.save_statistics();
        let Some(path) = self.statistics_path() else {
            return;
        };
        let now = platform::awake_ms();
        match statistics_store::save(&path, &[self.core.today_statistics(now)]) {
            Ok(()) => self.core.statistics_saved(now),
            Err(error) => warn!(%error, "statistics not saved"),
        }
    }

    pub(crate) fn open_daily_limit(&mut self, limit: DailyLimitScreen) {
        self.close_daily_limit();
        let on_button = |button| {
            with_app(move |app| {
                let effects = app
                    .core
                    .on_daily_limit(button == BreakButton::Postpone, platform::awake_ms());
                app.carry_out(effects);
                app.save_statistics();
                app.rearm_timer_and_refresh_tray();
            });
        };
        info!("daily limit reminder");
        match BreakView::open(limit.screen, self.core.settings(), on_button, || {}) {
            Ok(view) => {
                view.show_still(&limit.number, &limit.caption);
                self.limit_view = Some(view);
            }
            Err(error) => warn!(%error, "daily limit window failed"),
        }
    }

    pub(crate) fn close_daily_limit(&mut self) {
        if let Some(view) = self.limit_view.take() {
            view.close();
        }
    }

    /// Starts the input block for the break that just opened, when the setting is on.
    pub(crate) fn block_input(&mut self) {
        if !self.core.settings().block_input_during_breaks || self.break_view.is_none() {
            return;
        }
        match platform::InputBlock::start() {
            Ok(block) => self.input_block = Some(block),
            Err(error) => warn!(%error, "input not blocked"),
        }
    }
}
