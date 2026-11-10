//! The Slint break window: opens it with the break's words, refreshes the countdown once a
//! second, and hands button presses back to the app.

use std::time::Duration;

use breaks::{BreakTime, Rect, window_position};
use i18n::Translator;
use settings::Settings;
use slint::{ComponentHandle, PhysicalPosition, PhysicalSize, Timer, TimerMode};

use crate::logic::{BreakButton, BreakScreen, countdown, ring_commands};
use crate::ui::{BreakWindow, Palette};

pub struct BreakView {
    window: BreakWindow,
    _tick: Timer,
}

impl BreakView {
    /// Shows the window; `on_tick` runs once a second until the view closes.
    pub fn open(
        screen: BreakScreen,
        settings: &Settings,
        on_button: impl Fn(BreakButton) + Clone + 'static,
        on_tick: impl FnMut() + 'static,
    ) -> Result<Self, slint::PlatformError> {
        let window = BreakWindow::new()?;
        window
            .global::<Palette>()
            .set_theme(crate::ui::theme(settings));
        let title = screen.kind.clone();
        window.set_kind(screen.kind.into());
        window.set_headline(screen.headline.into());
        window.set_detail(screen.detail.into());
        window.set_source(screen.source.into());
        window.set_postpone_label(screen.postpone.into());
        window.set_skip_label(screen.skip.into());
        window.set_finish_label(screen.finish.into());
        let press = |button| {
            let on_button = on_button.clone();
            move || on_button(button)
        };
        window.on_postpone(press(BreakButton::Postpone));
        window.on_skip(press(BreakButton::Skip));
        window.on_finish(press(BreakButton::Finish));
        place(&window, settings);
        // Only the buttons end a break early: Alt+F4 does not close it.
        window
            .window()
            .on_close_requested(|| slint::CloseRequestResponse::KeepWindowShown);
        window.show()?;
        // The window exists only once shown. Always-on-top alone leaves the keyboard with the
        // app behind; the break takes it, and loses its minimize routes.
        Timer::single_shot(Duration::from_millis(50), move || {
            if !(platform::lock_window(&title) && platform::bring_to_front(&title)) {
                tracing::warn!(title, "break window not locked in front");
            }
        });
        let tick = Timer::default();
        tick.start(TimerMode::Repeated, Duration::from_secs(1), on_tick);
        Ok(Self {
            window,
            _tick: tick,
        })
    }

    pub fn refresh(&self, words: &Translator, time: &BreakTime) {
        let (number, caption) = countdown(words, time);
        self.window.set_number(number.into());
        self.window.set_caption(caption.into());
        self.window.set_ring(ring_commands(time).into());
        self.window.set_show_postpone(time.may_postpone);
        self.window.set_show_skip(time.may_skip);
        self.window.set_show_finish(time.awaiting_finish);
    }

    /// Shows a still ring with `number` and `caption`, and the Postpone and Skip buttons
    /// (the daily-limit reminder).
    pub fn show_still(&self, number: &str, caption: &str) {
        let full = BreakTime {
            long: false,
            remaining: 1,
            length: 1,
            may_postpone: true,
            may_skip: true,
            awaiting_finish: false,
        };
        self.window.set_number(number.into());
        self.window.set_caption(caption.into());
        self.window.set_ring(ring_commands(&full).into());
        self.window.set_show_postpone(true);
        self.window.set_show_skip(true);
        self.window.set_show_finish(false);
    }

    /// Hides the window now and frees it after the current event, which may be its own button.
    pub fn close(self) {
        if let Err(error) = self.window.hide() {
            tracing::warn!(%error, "break window not hidden");
        }
        Timer::single_shot(Duration::ZERO, move || drop(self));
    }
}

/// The whole display when `fullscreen`, else centred at `breakWindowWidth` × `breakWindowHeight`
/// of the primary display.
fn place(window: &BreakWindow, settings: &Settings) {
    if settings.fullscreen {
        return window.window().set_fullscreen(true);
    }
    let (width, height) = platform::primary_screen_size();
    let display = Rect {
        x: 0,
        y: 0,
        width,
        height,
    };
    let share = |size: i64, part: f64| (size as f64 * part.clamp(0.1, 1.0)) as i64;
    let rect = window_position(
        display,
        share(width, settings.break_window_width),
        share(height, settings.break_window_height),
        false,
    );
    window
        .window()
        .set_position(PhysicalPosition::new(rect.x as i32, rect.y as i32));
    window
        .window()
        .set_size(PhysicalSize::new(rect.width as u32, rect.height as u32));
}
