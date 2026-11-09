//! The settings window: filled from the settings, and every change applied and saved at once.

use std::path::Path;
use std::rc::Rc;
use std::time::Duration;

use i18n::LANGUAGES;
use settings::Settings;
use slint::{ComponentHandle, ModelRc, SharedString, Timer, VecModel};
use tracing::{info, warn};

use crate::logic::{BreakForm, SettingsForm};
use crate::ui::{Palette, SettingsWindow};
use crate::{App, app_folder, log_path, sync_autostart, with_app};

const TITLE: &str = "Ebb settings";
/// The window's ground colour in dark and light mode, for its title bar.
const GROUND: [u32; 2] = [0x16130E, 0xFAF6EE];
const WEBSITE: &str = "https://hovancik.net/stretchly";

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
        let languages: Vec<SharedString> =
            LANGUAGES.iter().map(|(_, name)| (*name).into()).collect();
        window.set_languages(ModelRc::new(VecModel::from(languages)));
        // The short form fits the row; the full path is long and user-specific.
        window.set_audio_folder(r"%APPDATA%\stretch-core\audio".into());
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
        window.on_open_website(|| open(Some(Path::new(WEBSITE))));
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
        let files = self
            .audio_folder
            .as_deref()
            .map(sound_files)
            .unwrap_or_default();
        window
            .global::<Palette>()
            .set_theme(crate::ui::theme(self.core.settings()));
        let form = SettingsForm::new(self.core.settings(), &files);
        set_form(window, &form);
        self.settings_sounds = form.sounds;
    }

    fn on_settings_changed(&mut self) {
        let Some(window) = &self.settings_window else {
            return;
        };
        let form = read_form(window, self.settings_sounds.clone());
        let changed = form.apply(self.core.settings());
        self.change_settings(changed);
    }

    /// Applies, saves and (for start at login) syncs `changed`; refills the window.
    fn change_settings(&mut self, changed: Settings) {
        if changed == *self.core.settings() {
            return;
        }
        let login = changed.open_at_login != self.core.settings().open_at_login;
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
        self.fill_settings_window();
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

fn set_form(window: &SettingsWindow, form: &SettingsForm) {
    window.set_fullscreen(form.fullscreen);
    window.set_ideas(form.ideas);
    window.set_natural_breaks(form.natural_breaks);
    window.set_open_at_login(form.open_at_login);
    window.set_monitor_dnd(form.monitor_dnd);
    window.set_language(form.language as i32);
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
    let labels: Vec<SharedString> = form.sound_labels().into_iter().map(Into::into).collect();
    window.set_sounds(ModelRc::from(Rc::new(VecModel::from(labels))));
    let [mini_start, mini_end, long_start, long_end] = form.sound_choice.map(|i| i as i32);
    window.set_mini_start(mini_start);
    window.set_mini_end(mini_end);
    window.set_long_start(long_start);
    window.set_long_end(long_end);
    window.set_volume(form.volume as i32);
}

fn read_form(window: &SettingsWindow, sounds: Vec<String>) -> SettingsForm {
    let index = |value: i32| usize::try_from(value).unwrap_or(0);
    SettingsForm {
        fullscreen: window.get_fullscreen(),
        ideas: window.get_ideas(),
        natural_breaks: window.get_natural_breaks(),
        open_at_login: window.get_open_at_login(),
        monitor_dnd: window.get_monitor_dnd(),
        language: index(window.get_language()).min(LANGUAGES.len() - 1),
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

/// Colours the settings title bar for the palette mode.
fn colour_title_bar(dark: bool) {
    if !platform::title_bar(TITLE, dark, GROUND[usize::from(!dark)]) {
        warn!("settings title bar not coloured");
    }
}

/// The names of the WAV files in `folder` that the player accepts, sorted.
fn sound_files(folder: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
        .filter_map(|file| file.strip_suffix(".wav").map(str::to_owned))
        .filter(|name| crate::sound_file(folder, name).is_some())
        .collect();
    names.sort();
    names
}

/// Opens a folder in Explorer, or a web address in the default browser.
fn open(target: Option<&Path>) {
    let Some(target) = target else {
        return;
    };
    if let Err(error) = std::process::Command::new("explorer").arg(target).spawn() {
        warn!(%error, "could not open {}", target.display());
    }
}
