//! Composition root: loads settings, connects Windows signals and the tray to the planner,
//! and carries out what the planner returns. Everything runs on the main thread.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod break_window;
mod logging;
mod logic;
mod settings_window;
mod settings_words;
mod sounds;
mod storage;
mod tray_menu;
mod ui;
mod update;

use std::cell::RefCell;
use std::collections::VecDeque;
use std::path::PathBuf;

use breaks::until_morning;
use commands::Command;
use platform::{Signal, SignalWindow, awake_ms, idle_ms, sound, toast};
use settings::Settings;
use tracing::{error, info, warn};
use tray_icon::menu::MenuEvent;
use tray_icon::{TrayIcon, TrayIconBuilder, TrayIconEvent};

use crate::break_window::BreakView;
use crate::logic::{BreakButton, Core, Effect, Environment, MenuAction, MenuEntry};
use crate::storage::{app_folder, load_settings, log_path, prepare_app_folder};
use crate::tray_menu::{build_menu, tray_icon};

const APP_ID: &str = "EbbRise";

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
    /// `%APPDATA%\EbbRise\audio`, where the user's sounds live.
    audio_folder: Option<PathBuf>,
    /// The break window while a break runs.
    break_view: Option<BreakView>,
    settings_window: Option<ui::SettingsWindow>,
    /// The sound names behind the settings window's sound lists.
    settings_sounds: Vec<String>,
    /// A newer release tag, once the update check found one.
    new_version: Option<String>,
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
    if !platform::is_first_instance(r"Local\EbbRise") {
        let args: Vec<String> = std::env::args().skip(1).collect();
        return run_second_instance(&args);
    }
    let audio_folder = prepare_app_folder();
    info!(version = env!("CARGO_PKG_VERSION"), "starting");
    let settings = load_settings();
    sync_autostart(settings.open_at_login);
    if let Err(error) = toast::register(APP_ID, "EbbRise") {
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
            settings_window: None,
            settings_sounds: Vec::new(),
            new_version: None,
        });
    });
    with_app(App::rearm_timer_and_refresh_tray);
    update::start();
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
                Effect::OpenSettings => self.open_settings(),
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
        if action == MenuAction::DownloadUpdate {
            return open_in_shell(update::RELEASES);
        }
        if action == MenuAction::Preferences {
            return self.open_settings();
        }
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
        let mut menu = self.update_entries();
        menu.extend(self.core.menu(now));
        if menu != self.menu {
            self.tray.set_menu(Some(Box::new(build_menu(&menu))));
            self.menu = menu;
        }
    }

    /// Plays sound `name` (the user's file or a built-in one) at the settings' volume.
    fn play(&self, name: &str) {
        let Some(wav) = sounds::wav(self.audio_folder.as_deref(), name) else {
            return info!(name, "no sound file");
        };
        match sound::with_volume(&wav, self.core.settings().volume) {
            Some(wav) => sound::play(wav),
            None => warn!(name, "not a WAV file"),
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
        "version" => println!("EbbRise version {}", env!("CARGO_PKG_VERSION")),
        "logs" => match app_folder() {
            Some(folder) => println!("{}", log_path(&folder).display()),
            None => eprintln!("APPDATA is not set, so there is no log file"),
        },
        _ => {
            let taken = platform::running_app_window()
                .is_some_and(|window| platform::send_command(window, args));
            if !taken {
                eprintln!("the running EbbRise app did not take the command");
            }
        }
    }
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

/// Opens a folder in Explorer, or a web address in the default browser.
fn open_in_shell(target: &str) {
    if let Err(error) = std::process::Command::new("explorer").arg(target).spawn() {
        warn!(%error, "could not open {target}");
    }
}
