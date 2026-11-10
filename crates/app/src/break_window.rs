//! The Slint break windows: opens one on each display that shows the break, refreshes the
//! countdown once a second, and hands button presses back to the app.

use std::time::Duration;

use breaks::{BreakTime, Display, Rect, target_display, window_position};
use i18n::Translator;
use settings::Settings;
use slint::{ComponentHandle, PhysicalPosition, PhysicalSize, Timer, TimerMode};

use crate::logic::{BreakButton, BreakScreen, countdown, idea_shown, ring_commands};
use crate::ui::{BreakWindow, Palette};

pub struct BreakView {
    /// One window per display that shows the break; the first one takes the keyboard.
    windows: Vec<BreakWindow>,
    _tick: Timer,
}

impl BreakView {
    /// Shows the windows; `on_tick` runs once a second until the view closes.
    pub fn open(
        screen: BreakScreen,
        settings: &Settings,
        on_button: impl Fn(BreakButton) + Clone + 'static,
        on_tick: impl FnMut() + 'static,
    ) -> Result<Self, slint::PlatformError> {
        let title = screen.kind.clone();
        let windows = target_rects(settings)
            .into_iter()
            .map(|rect| open_window(&screen, settings, on_button.clone(), rect))
            .collect::<Result<Vec<_>, _>>()?;
        // The windows exist only once shown. Always-on-top alone leaves the keyboard with the
        // app behind; the break takes it, and loses its minimize routes.
        Timer::single_shot(Duration::from_millis(50), move || {
            if !(platform::lock_window(&title) && platform::bring_to_front(&title)) {
                tracing::warn!(title, "break window not locked in front");
            }
        });
        let tick = Timer::default();
        tick.start(TimerMode::Repeated, Duration::from_secs(1), on_tick);
        Ok(Self {
            windows,
            _tick: tick,
        })
    }

    pub fn refresh(&self, words: &Translator, time: &BreakTime) {
        let (number, caption) = countdown(words, time);
        let ring = ring_commands(time);
        let passed = 1.0 - time.remaining as f32 / time.length.max(1) as f32;
        for window in &self.windows {
            window.set_number(number.as_str().into());
            window.set_caption(caption.as_str().into());
            window.set_ring(ring.as_str().into());
            window.set_passed(passed.clamp(0.0, 1.0));
            window.set_idea_shown(idea_shown(time));
            window.set_show_postpone(time.may_postpone);
            window.set_show_skip(time.may_skip);
            window.set_show_finish(time.awaiting_finish);
        }
    }

    /// Shows a still ring with `number` and `caption`, no water, the words at once, and the
    /// Postpone and Skip buttons (the daily-limit reminder).
    pub fn show_still(&self, number: &str, caption: &str) {
        let full = BreakTime {
            long: false,
            remaining: 1,
            length: 1,
            may_postpone: true,
            may_skip: true,
            awaiting_finish: false,
        };
        for window in &self.windows {
            window.set_number(number.into());
            window.set_caption(caption.into());
            window.set_ring(ring_commands(&full).into());
            window.set_passed(0.0);
            window.set_idea_shown(true);
            window.set_show_postpone(true);
            window.set_show_skip(true);
            window.set_show_finish(false);
        }
    }

    /// Hides the windows now and frees them after the current event, which may be a button.
    pub fn close(self) {
        for window in &self.windows {
            if let Err(error) = window.hide() {
                tracing::warn!(%error, "break window not hidden");
            }
        }
        Timer::single_shot(Duration::ZERO, move || drop(self));
    }
}

fn open_window(
    screen: &BreakScreen,
    settings: &Settings,
    on_button: impl Fn(BreakButton) + Clone + 'static,
    rect: Rect,
) -> Result<BreakWindow, slint::PlatformError> {
    let window = BreakWindow::new()?;
    window
        .global::<Palette>()
        .set_theme(crate::ui::theme(settings));
    window.set_kind(screen.kind.as_str().into());
    window.set_headline(screen.headline.as_str().into());
    window.set_detail(screen.detail.as_str().into());
    window.set_source(screen.source.as_str().into());
    window.set_postpone_label(screen.postpone.as_str().into());
    window.set_skip_label(screen.skip.as_str().into());
    window.set_finish_label(screen.finish.as_str().into());
    let press = |button| {
        let on_button = on_button.clone();
        move || on_button(button)
    };
    window.on_postpone(press(BreakButton::Postpone));
    window.on_skip(press(BreakButton::Skip));
    window.on_finish(press(BreakButton::Finish));
    window
        .window()
        .set_position(PhysicalPosition::new(rect.x as i32, rect.y as i32));
    window
        .window()
        .set_size(PhysicalSize::new(rect.width as u32, rect.height as u32));
    // Only the buttons end a break early: Alt+F4 does not close it.
    window
        .window()
        .on_close_requested(|| slint::CloseRequestResponse::KeepWindowShown);
    window.show()?;
    Ok(window)
}

/// Where the break windows go: the display that Stretchly's `screen` chooses, or every display
/// with `allScreens`; all of it with `fullscreen`, else centred at `breakWindowWidth` ×
/// `breakWindowHeight` of it.
fn target_rects(settings: &Settings) -> Vec<Rect> {
    let found = platform::displays().unwrap_or_else(|error| {
        tracing::warn!(%error, "displays not listed; using the main display");
        let (width, height) = platform::primary_screen_size();
        let bounds = platform::Rect {
            x: 0,
            y: 0,
            width,
            height,
        };
        platform::Displays {
            displays: vec![platform::Display {
                id: 0,
                bounds,
                work_area: bounds,
            }],
            primary: 0,
            cursor: 0,
        }
    });
    let displays: Vec<Display> = found
        .displays
        .iter()
        .map(|d| Display {
            id: d.id,
            bounds: Rect {
                x: d.bounds.x,
                y: d.bounds.y,
                width: d.bounds.width,
                height: d.bounds.height,
            },
        })
        .collect();
    let count = if settings.all_screens {
        displays.len()
    } else {
        1
    };
    let share = |size: i64, part: f64| (size as f64 * part.clamp(0.1, 1.0)) as i64;
    (0..count)
        .filter_map(|index| {
            target_display(
                &displays,
                found.primary,
                found.cursor,
                settings,
                index as i64,
            )
        })
        .map(|display| {
            let bounds = display.bounds;
            window_position(
                bounds,
                share(bounds.width, settings.break_window_width),
                share(bounds.height, settings.break_window_height),
                settings.fullscreen,
            )
        })
        .collect()
}
