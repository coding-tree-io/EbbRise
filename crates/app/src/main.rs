//! Composition root: loads settings, connects Windows signals and the tray to the planner,
//! and carries out what the planner returns. Everything runs on the main thread.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod break_window;
mod logging;
mod logic;
mod tray_menu;

use std::cell::RefCell;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use breaks::until_morning;
use commands::Command;
use platform::{Signal, SignalWindow, awake_ms, idle_ms, sound, toast};
use settings::Settings;
use tracing::{error, info, warn};
use tray_icon::menu::MenuEvent;
use tray_icon::{Icon, TrayIcon, TrayIconBuilder, TrayIconEvent};

use crate::break_window::BreakView;
use crate::logic::{BreakButton, Core, Effect, Environment, MenuAction, MenuEntry};
use crate::tray_menu::build_menu;

const APP_ID: &str = "StretchCore";

/// The real OS behind [`Environment`].
struct Windows;

impl Environment for Windows {
    fn running_processes(&self) -> Option<Vec<String>> {
        platform::running_process_names()
            .inspect_err(|error| warn!(%error, "process list unavailable"))
            .ok()
    }

    fn until_morning(&self, settings: &Settings) -> Option<i64> {
        until_morning(
            &jiff::Zoned::now(),
            &settings.morning_hour,
            settings.pos_latitude,
            settings.pos_longitude,
        )
    }
}

struct App {
    core: Core,
    signals: SignalWindow,
    tray: TrayIcon,
    /// The menu the tray shows, so it is rebuilt only when it changes.
    menu: Vec<MenuEntry>,
    /// `%APPDATA%\stretch-core\audio`, where the user's sounds live.
    audio_folder: Option<PathBuf>,
    /// The break window while a break runs.
    break_view: Option<BreakView>,
}

type AppAction = Box<dyn FnOnce(&mut App)>;

thread_local! {
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
    /// Actions that arrived while another action ran: a Windows call inside an action can
    /// deliver a tray, menu or OS event to this thread, which then waits here.
    static WAITING: RefCell<VecDeque<AppAction>> = const { RefCell::new(VecDeque::new()) };
}

/// Runs `action` on the app, after any actions that are still waiting.
fn with_app(action: impl FnOnce(&mut App) + 'static) {
    WAITING.with(|waiting| waiting.borrow_mut().push_back(Box::new(action)));
    APP.with(|app| {
        // Busy: the action that is running takes this one when it is done.
        let Ok(mut app) = app.try_borrow_mut() else {
            return;
        };
        let Some(app) = app.as_mut() else {
            return;
        };
        while let Some(action) = WAITING.with(|waiting| waiting.borrow_mut().pop_front()) {
            action(app);
        }
    });
}

fn main() {
    if !platform::is_first_instance(r"Local\StretchCore") {
        let args: Vec<String> = std::env::args().skip(1).collect();
        return run_second_instance(&args);
    }
    let folder = app_folder();
    if let Some(folder) = &folder {
        logging::start(&log_path(folder));
    }
    info!(version = env!("CARGO_PKG_VERSION"), "starting");
    let settings = load_settings();
    sync_autostart(settings.open_at_login);
    let audio_folder = folder.map(|folder| folder.join("audio"));
    if let Some(audio) = &audio_folder
        && let Err(error) = std::fs::create_dir_all(audio)
    {
        warn!(%error, "audio folder not created");
    }
    if let Err(error) = toast::register(APP_ID, "Stretch") {
        error!(%error, "toast registration failed");
    }
    let signals =
        SignalWindow::new(|signal| with_app(|app| app.on_signal(signal))).expect("signal window");
    let core = Core::new(settings, awake_ms(), signals.focus_assist);

    let tray = TrayIconBuilder::new()
        .with_tooltip(core.words().text("main.toolTipHeader"))
        .with_icon(tray_icon(false))
        .build()
        .expect("tray icon");
    TrayIconEvent::set_event_handler(Some(|event| {
        if let TrayIconEvent::Enter { .. } = event {
            with_app(|app| app.refresh_tray());
        }
    }));
    MenuEvent::set_event_handler(Some(|event: MenuEvent| {
        if let Some(action) = MenuAction::from_id(event.id.0.as_str()) {
            with_app(move |app| app.on_menu(action));
        }
    }));

    APP.with(|app| {
        *app.borrow_mut() = Some(App {
            core,
            signals,
            tray,
            menu: Vec::new(),
            audio_folder,
            break_view: None,
        });
    });
    with_app(App::rearm_timer_and_refresh_tray);
    // Slint's loop also dispatches the signal window's and the tray's Win32 messages.
    if let Err(error) = slint::run_event_loop_until_quit() {
        error!(%error, "event loop failed");
    }
}

impl App {
    fn on_signal(&mut self, signal: Signal) {
        if signal != Signal::Timer {
            info!(?signal, "signal");
        }
        let effects = self.core.on_signal(signal, awake_ms(), idle_ms(), &Windows);
        self.carry_out(effects);
        self.rearm_timer_and_refresh_tray();
    }

    fn on_break_button(&mut self, button: BreakButton) {
        info!(?button, "break button");
        let effects = self.core.on_break_button(button, awake_ms());
        self.carry_out(effects);
        self.rearm_timer_and_refresh_tray();
    }

    fn carry_out(&mut self, effects: Vec<Effect>) {
        for effect in effects {
            match effect {
                Effect::Toast(toast) => {
                    info!(title = %toast.title, "toast");
                    toast::show(
                        APP_ID,
                        &toast.title,
                        &toast.text,
                        self.core.settings().silent_notifications,
                    );
                }
                Effect::Sound(name) => self.play(&name),
                Effect::ShowBreak(screen) => self.open_break(screen),
                Effect::CloseBreak => self.close_break(),
            }
        }
    }

    fn close_break(&mut self) {
        if let Some(view) = self.break_view.take() {
            view.close();
        }
    }

    fn open_break(&mut self, screen: logic::BreakScreen) {
        self.close_break();
        let on_button = |button| with_app(move |app| app.on_break_button(button));
        let on_tick = || with_app(App::refresh_break);
        match BreakView::open(screen, self.core.settings(), on_button, on_tick) {
            Ok(view) => self.break_view = Some(view),
            Err(error) => error!(%error, "break window failed"),
        }
        self.refresh_break();
    }

    fn refresh_break(&mut self) {
        if let (Some(view), Some(time)) = (&self.break_view, self.core.break_time(awake_ms())) {
            view.refresh(self.core.words(), &time);
        }
    }

    fn on_menu(&mut self, action: MenuAction) {
        info!(?action, "menu");
        if action == MenuAction::Quit {
            if self.core.may_quit() {
                return drop(slint::quit_event_loop());
            }
            return info!("quit refused: a strict-mode break runs");
        }
        self.core.on_menu(action, awake_ms(), &Windows);
        self.rearm_timer_and_refresh_tray();
    }

    fn rearm_timer_and_refresh_tray(&mut self) {
        match self.core.next_wake(awake_ms(), idle_ms()) {
            Some(wake) => self.signals.arm(wake.delay, wake.tolerance),
            None => self.signals.disarm(),
        }
        self.refresh_tray();
    }

    fn refresh_tray(&mut self) {
        let now = awake_ms();
        let (text, paused) = self.core.tray(now);
        let _ = self.tray.set_tooltip(Some(text));
        let _ = self.tray.set_icon(Some(tray_icon(paused)));
        let menu = self.core.menu(now);
        if menu != self.menu {
            self.tray.set_menu(Some(Box::new(build_menu(&menu))));
            self.menu = menu;
        }
    }

    /// Plays the user's `<name>.wav` from the audio folder at the settings' volume.
    fn play(&self, name: &str) {
        // shortcut: Stretchly's own WAVs are not bundled until their licence is known (ADR 0001 R4).
        let Some(path) = self
            .audio_folder
            .as_deref()
            .and_then(|folder| sound_file(folder, name))
        else {
            return info!(name, "no sound file");
        };
        match std::fs::read(&path) {
            Ok(wav) => match sound::with_volume(&wav, self.core.settings().volume) {
                Some(wav) => sound::play(wav),
                None => warn!(path = %path.display(), "not a WAV file"),
            },
            Err(error) => warn!(path = %path.display(), %error, "sound unreadable"),
        }
    }
}

/// Answers `help`, `version` and `logs` here and passes every other command to the running app.
fn run_second_instance(args: &[String]) {
    platform::attach_parent_console();
    let words: Vec<&str> = args.iter().map(String::as_str).collect();
    let Some(command) = Command::parse(&words) else {
        return eprintln!("command '{}' is not supported", words.join(" "));
    };
    match command.name.as_str() {
        "help" => println!("{}", commands::HELP),
        "version" => println!("Stretch version {}", env!("CARGO_PKG_VERSION")),
        "logs" => match app_folder() {
            Some(folder) => println!("{}", log_path(&folder).display()),
            None => eprintln!("APPDATA is not set, so there is no log file"),
        },
        _ => {
            let taken = platform::running_app_window()
                .is_some_and(|window| platform::send_command(window, args));
            if !taken {
                eprintln!("the running Stretch app did not take the command");
            }
        }
    }
}

/// `<name>.wav` in `folder`, when the name is a plain file name (upstream `audioResolver`)
/// and the file exists.
fn sound_file(folder: &Path, name: &str) -> Option<PathBuf> {
    let file = format!("{name}.wav");
    let plain = file
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'));
    let path = folder.join(file);
    (plain && path.is_file()).then_some(path)
}

/// Upstream syncs the login item with `openAtLogin` at every start.
fn sync_autostart(open_at_login: bool) {
    let command = std::env::current_exe()
        .ok()
        .map(|exe| format!("\"{}\"", exe.display()));
    let command = command.filter(|_| open_at_login);
    if let Err(error) = platform::set_autostart(APP_ID, command.as_deref()) {
        warn!(%error, "autostart not changed");
    }
}

/// shortcut: a plain coloured square until the redesigned icons exist (Stretchly's icon licence is unclear, R4).
fn tray_icon(paused: bool) -> Icon {
    let colour = if paused {
        [150u8, 150, 150, 255]
    } else {
        [40u8, 160, 120, 255]
    };
    Icon::from_rgba(colour.repeat(16 * 16), 16, 16).expect("tray icon image")
}

/// `%APPDATA%\stretch-core`, where the config and the logs live.
fn app_folder() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|app_data| PathBuf::from(app_data).join("stretch-core"))
}

fn log_path(app_folder: &Path) -> PathBuf {
    app_folder.join("logs").join("main.log")
}

fn load_settings() -> Settings {
    let Some(folder) = app_folder() else {
        return Settings::default();
    };
    let own = folder.join("config.json");
    let stretchly = folder.with_file_name("Stretchly").join("config.json");
    match Settings::load_or_import(&own, &stretchly) {
        Ok(settings) => {
            info!(path = %own.display(), "settings loaded");
            settings
        }
        Err(error) => {
            error!(path = %own.display(), %error, "settings unreadable, using defaults");
            Settings::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sound_files_are_plain_names_inside_the_audio_folder() {
        let folder = std::env::temp_dir().join(format!("stretch-audio-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join("my-chime_2.wav"), b"").unwrap();
        assert_eq!(
            sound_file(&folder, "my-chime_2"),
            Some(folder.join("my-chime_2.wav"))
        );
        assert_eq!(sound_file(&folder, "missing"), None);
        assert_eq!(sound_file(&folder, r"..\my-chime_2"), None);
        assert_eq!(sound_file(&folder, "sub/my-chime_2"), None);
        std::fs::remove_dir_all(&folder).unwrap();
    }
}
