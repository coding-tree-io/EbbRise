//! The settings window: filled from the settings, and every change applied and saved at once.

use std::path::Path;
use std::rc::Rc;
use std::time::Duration;

use i18n::LANGUAGES;
use settings::Settings;
use slint::{ComponentHandle, ModelRc, SharedString, Timer, VecModel};
use tracing::{info, warn};

use crate::logic::{BreakForm, SettingsForm, UsageForm};
use crate::ui::{Palette, SettingsWindow};
use crate::{App, app_folder, log_path, sync_autostart, with_app};

const TITLE: &str = "EbbRise settings";
/// The window's ground colour in dark and light mode, for its title bar.
pub(crate) const GROUND: [u32; 2] = [0x16130E, 0xFAF6EE];

impl App {
    /// Opens the settings window, or brings the open one to the front.
    pub(crate) fn open_settings(&mut self) {
        if let Some(window) = &self.settings_window {
            let _ = window.show();
            return;
        }
        let window = match SettingsWindow::new() {
            Ok(window) => window,
            Err(error) => return warn!(%error, "settings window failed"),
        };
        window.set_version(env!("CARGO_PKG_VERSION").into());
        crate::settings_words::translate(&window, self.core.words());
        let languages: Vec<SharedString> =
            LANGUAGES.iter().map(|(_, name)| (*name).into()).collect();
        window.set_languages(ModelRc::new(VecModel::from(languages)));
        // The short form fits the row; the full path is long and user-specific.
        window.set_audio_folder(r"%APPDATA%\EbbRise\audio".into());
        window.on_changed(|| with_app(App::on_settings_changed));
        window.on_play(|slot| with_app(move |app| app.preview_sound(slot)));
        window.on_open_audio_folder(|| with_app(|app| open(app.audio_folder.as_deref())));
        window.on_open_logs(|| {
            open(
                app_folder()
                    .map(|f| log_path(&f))
                    .as_deref()
                    .and_then(Path::parent),
            )
        });
        window.on_dark_mode_changed(colour_title_bar);
        window.on_restore_defaults(|| with_app(|app| app.change_settings(Settings::default())));
        window.window().on_close_requested(|| {
            with_app(App::close_settings);
            slint::CloseRequestResponse::HideWindow
        });
        self.settings_window = Some(window);
        self.fill_settings_window();
        if let Some(window) = &self.settings_window {
            if let Err(error) = window.show() {
                warn!(%error, "settings window not shown");
            }
            // Windows creates the native window during the next event loop turn.
            let dark = window.get_dark_mode();
            Timer::single_shot(Duration::from_millis(50), move || colour_title_bar(dark));
        }
    }

    fn reopen_settings(&mut self) {
        if let Some(window) = self.settings_window.take() {
            let _ = window.hide();
            Timer::single_shot(Duration::ZERO, move || drop(window));
        }
        self.open_settings();
    }

    /// Frees the window after this event, as the break window is.
    fn close_settings(&mut self) {
        if let Some(window) = self.settings_window.take() {
            Timer::single_shot(Duration::ZERO, move || drop(window));
        }
    }

    /// Puts the current settings and sound files into the window.
    fn fill_settings_window(&mut self) {
        let Some(window) = &self.settings_window else {
            return;
        };
        let files = crate::sounds::names(self.audio_folder.as_deref());
        window
            .global::<Palette>()
            .set_theme(crate::ui::theme(self.core.settings()));
        let form = SettingsForm::new(self.core.settings(), &files);
        set_form(window, &form, self.core.words());
        self.settings_sounds = form.sounds;
    }

    fn on_settings_changed(&mut self) {
        let Some(window) = &self.settings_window else {
            return;
        };
        let form = read_form(window, self.settings_sounds.clone(), self.core.settings());
        let changed = form.apply(self.core.settings());
        self.change_settings(changed);
    }

    /// Applies, saves and (for start at login) syncs `changed`; refills the window.
    fn change_settings(&mut self, changed: Settings) {
        if changed == *self.core.settings() {
            return;
        }
        let login = changed.open_at_login != self.core.settings().open_at_login;
        let language = changed.language != self.core.settings().language;
        if let Some(path) = app_folder().map(|folder| folder.join("config.json"))
            && let Err(error) = changed.save(&path)
        {
            warn!(path = %path.display(), %error, "settings not saved");
        }
        if login {
            sync_autostart(changed.open_at_login);
        }
        info!("settings changed");
        self.core.set_settings(changed, platform::awake_ms());
        if language {
            // The window's words are set when it opens, so it opens again in the new language.
            self.reopen_settings();
        } else {
            self.fill_settings_window();
        }
        self.rearm_timer_and_refresh_tray();
    }

    fn preview_sound(&self, slot: i32) {
        let Some(window) = &self.settings_window else {
            return;
        };
        let choice = [
            window.get_mini_start(),
            window.get_mini_end(),
            window.get_long_start(),
            window.get_long_end(),
        ];
        let index = usize::try_from(slot)
            .ok()
            .and_then(|slot| choice.get(slot).copied());
        let name = index
            .and_then(|i| usize::try_from(i).ok())
            .and_then(|i| self.settings_sounds.get(i));
        if let Some(name) = name.filter(|name| name.as_str() != crate::logic::SILENCE) {
            self.play(name);
        }
    }
}

fn set_form(window: &SettingsWindow, form: &SettingsForm, words: &i18n::Translator) {
    window.set_fullscreen(form.fullscreen);
    window.set_ideas(form.ideas);
    window.set_natural_breaks(form.natural_breaks);
    window.set_open_at_login(form.open_at_login);
    window.set_monitor_dnd(form.monitor_dnd);
    window.set_full_screen_hold(form.no_breaks_in_full_screen);
    window.set_pause_media(form.pause_media);
    let sizes = display_sizes();
    let (choices, choice) = display_choices(&sizes, &form.screen, form.all_screens);
    window.set_displays(ModelRc::new(VecModel::from(
        choices
            .into_iter()
            .map(SharedString::from)
            .collect::<Vec<_>>(),
    )));
    window.set_display(choice as i32);
    window.set_language(form.language as i32);
    window.set_theme(form.theme as i32);
    window.set_mini_on(form.mini.on);
    window.set_mini_every(form.mini.every as i32);
    window.set_mini_length(form.mini.length as i32);
    window.set_mini_notify(form.mini.notify);
    window.set_mini_strict(form.mini.strict);
    window.set_long_on(form.long.on);
    window.set_long_every(form.long.every as i32);
    window.set_long_length(form.long.length as i32);
    window.set_long_notify(form.long.notify);
    window.set_long_strict(form.long.strict);
    window.set_active_time_only(form.usage.active_time_only);
    window.set_block_input(form.usage.block_input);
    window.set_daily_limit(form.usage.daily_limit);
    window.set_daily_limit_hours(form.usage.daily_limit_hours as i32);
    let labels: Vec<SharedString> = form
        .sounds
        .iter()
        .zip(form.sound_labels())
        .map(|(name, label)| crate::settings_words::sound_name(words, name).unwrap_or(label))
        .map(Into::into)
        .collect();
    window.set_sounds(ModelRc::from(Rc::new(VecModel::from(labels))));
    let [mini_start, mini_end, long_start, long_end] = form.sound_choice.map(|i| i as i32);
    window.set_mini_start(mini_start);
    window.set_mini_end(mini_end);
    window.set_long_start(long_start);
    window.set_long_end(long_end);
    window.set_volume(form.volume as i32);
}

fn read_form(window: &SettingsWindow, sounds: Vec<String>, before: &Settings) -> SettingsForm {
    let index = |value: i32| usize::try_from(value).unwrap_or(0);
    let (screen, all_screens) = display_setting(index(window.get_display()), &before.screen);
    SettingsForm {
        no_breaks_in_full_screen: window.get_full_screen_hold(),
        pause_media: window.get_pause_media(),
        screen,
        all_screens,
        fullscreen: window.get_fullscreen(),
        ideas: window.get_ideas(),
        natural_breaks: window.get_natural_breaks(),
        open_at_login: window.get_open_at_login(),
        monitor_dnd: window.get_monitor_dnd(),
        language: index(window.get_language()).min(LANGUAGES.len() - 1),
        theme: index(window.get_theme()),
        mini: BreakForm {
            on: window.get_mini_on(),
            every: i64::from(window.get_mini_every()),
            length: i64::from(window.get_mini_length()),
            notify: window.get_mini_notify(),
            strict: window.get_mini_strict(),
        },
        long: BreakForm {
            on: window.get_long_on(),
            every: i64::from(window.get_long_every()),
            length: i64::from(window.get_long_length()),
            notify: window.get_long_notify(),
            strict: window.get_long_strict(),
        },
        usage: UsageForm {
            active_time_only: window.get_active_time_only(),
            block_input: window.get_block_input(),
            daily_limit: window.get_daily_limit(),
            daily_limit_hours: i64::from(window.get_daily_limit_hours()),
        },
        sound_choice: [
            window.get_mini_start(),
            window.get_mini_end(),
            window.get_long_start(),
            window.get_long_end(),
        ]
        .map(|i| index(i).min(sounds.len().saturating_sub(1))),
        sounds,
        volume: i64::from(window.get_volume()),
    }
}

/// The fixed entries of the "Show breaks on" list; displays follow them.
const DISPLAY_CHOICES: [&str; 3] = ["Main display", "Display with the mouse", "All displays"];

/// Width and height of each connected display; empty when Windows does not list them.
fn display_sizes() -> Vec<(i64, i64)> {
    platform::displays()
        .map(|found| {
            found
                .displays
                .iter()
                .map(|d| (d.bounds.width, d.bounds.height))
                .collect()
        })
        .unwrap_or_else(|error| {
            warn!(%error, "displays not listed");
            Vec::new()
        })
}

/// The list entries and the chosen one for Stretchly's `screen` and `allScreens`. A saved
/// display that is not connected now keeps an entry, so opening the window changes nothing.
fn display_choices(sizes: &[(i64, i64)], screen: &str, all_screens: bool) -> (Vec<String>, usize) {
    let saved = screen.parse::<usize>().ok();
    let count = sizes.len().max(saved.map_or(0, |i| i + 1));
    let mut choices: Vec<String> = DISPLAY_CHOICES.map(str::to_owned).into();
    choices.extend((0..count).map(|i| match sizes.get(i) {
        Some((width, height)) => format!("Display {} ({width} × {height})", i + 1),
        None => format!("Display {} (not connected)", i + 1),
    }));
    let choice = match (all_screens, screen, saved) {
        (true, ..) => 2,
        (_, "cursor", _) => 1,
        (_, _, Some(i)) => DISPLAY_CHOICES.len() + i,
        _ => 0,
    };
    (choices, choice)
}

/// Stretchly's `screen` and `allScreens` for a list entry; "All displays" keeps `screen`.
fn display_setting(choice: usize, screen: &str) -> (String, bool) {
    match choice {
        0 => ("primary".to_owned(), false),
        1 => ("cursor".to_owned(), false),
        2 => (screen.to_owned(), true),
        n => ((n - DISPLAY_CHOICES.len()).to_string(), false),
    }
}

/// Colours the settings title bar for the palette mode.
fn colour_title_bar(dark: bool) {
    if !platform::title_bar(TITLE, dark, GROUND[usize::from(!dark)]) {
        warn!("settings title bar not coloured");
    }
}

/// Opens a folder in Explorer.
fn open(target: Option<&Path>) {
    let Some(target) = target else {
        return;
    };
    if let Err(error) = std::process::Command::new("explorer").arg(target).spawn() {
        warn!(%error, "could not open {}", target.display());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_display_choice_reads_back_as_itself() {
        let sizes = [(2560, 1440), (1920, 1080)];
        let (choices, _) = display_choices(&sizes, "primary", false);
        assert_eq!(choices[4], "Display 2 (1920 × 1080)");
        for choice in 0..choices.len() {
            let (screen, all) = display_setting(choice, "1");
            assert_eq!(display_choices(&sizes, &screen, all).1, choice);
        }
        let (choices, choice) = display_choices(&sizes[..1], "2", false);
        assert_eq!(
            (choices[5].as_str(), choice),
            ("Display 3 (not connected)", 5)
        );
    }
}
