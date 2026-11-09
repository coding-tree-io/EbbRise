//! Composition root: loads settings, connects Windows signals and the tray to the planner,
//! and carries out what the planner returns. Everything runs on the main thread.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod logic;

use std::cell::RefCell;
use std::path::PathBuf;

use breaks::until_morning;
use commands::Command;
use platform::{Signal, SignalWindow, awake_ms, idle_ms, toast};
use settings::Settings;
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder, TrayIconEvent};

use crate::logic::{Core, Environment, MenuAction};

const APP_ID: &str = "StretchCore";

/// The real OS behind [`Environment`].
struct Windows;

impl Environment for Windows {
    fn running_processes(&self) -> Vec<String> {
        platform::running_process_names()
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
}

thread_local! {
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
}

fn with_app(action: impl FnOnce(&mut App)) {
    APP.with(|app| {
        if let Some(app) = app.borrow_mut().as_mut() {
            action(app);
        }
    });
}

fn main() {
    if !platform::is_first_instance(r"Local\StretchCore") {
        let args: Vec<String> = std::env::args().skip(1).collect();
        return run_second_instance(&args);
    }
    let settings = load_settings();
    if let Err(error) = toast::register(APP_ID, "Stretch") {
        eprintln!("toast registration failed: {error}");
    }
    let signals =
        SignalWindow::new(|signal| with_app(|app| app.on_signal(signal))).expect("signal window");
    let core = Core::new(settings, awake_ms(), signals.focus_assist);

    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(tray_menu()))
        .with_tooltip("Stretch")
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
            with_app(|app| app.on_menu(action));
        }
    }));

    APP.with(|app| {
        *app.borrow_mut() = Some(App {
            core,
            signals,
            tray,
        });
    });
    with_app(App::rearm_timer_and_refresh_tray);
    platform::run_message_loop();
}

impl App {
    fn on_signal(&mut self, signal: Signal) {
        for toast in self.core.on_signal(signal, awake_ms(), idle_ms(), &Windows) {
            toast::show(
                APP_ID,
                &toast.title,
                &toast.text,
                self.core.settings().silent_notifications,
            );
        }
        self.rearm_timer_and_refresh_tray();
    }

    fn on_menu(&mut self, action: MenuAction) {
        if action == MenuAction::Quit {
            return platform::quit();
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

    fn refresh_tray(&self) {
        let (text, paused) = self.core.tray(awake_ms());
        let _ = self.tray.set_tooltip(Some(text));
        let _ = self.tray.set_icon(Some(tray_icon(paused)));
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
        // shortcut: no log file until `tracing` lands; print its path then.
        "logs" => println!("Stretch keeps no log file yet."),
        _ => {
            let taken = platform::running_app_window()
                .is_some_and(|window| platform::send_command(window, args));
            if !taken {
                eprintln!("the running Stretch app did not take the command");
            }
        }
    }
}

fn tray_menu() -> Menu {
    let item = |action: MenuAction, text: &str| MenuItem::with_id(action.id(), text, true, None);
    Menu::with_items(&[
        &item(MenuAction::Resume, "Resume"),
        &item(MenuAction::PauseHour, "Pause for 1 hour"),
        &item(MenuAction::PauseUntilMorning, "Pause until morning"),
        &item(MenuAction::PauseIndefinitely, "Pause indefinitely"),
        &PredefinedMenuItem::separator(),
        &item(MenuAction::SkipToMini, "Skip to mini break"),
        &item(MenuAction::SkipToLong, "Skip to long break"),
        &item(MenuAction::Reset, "Reset breaks"),
        &PredefinedMenuItem::separator(),
        &item(MenuAction::Quit, "Quit"),
    ])
    .expect("tray menu")
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

fn load_settings() -> Settings {
    let Some(app_data) = std::env::var_os("APPDATA").map(PathBuf::from) else {
        return Settings::default();
    };
    let own = app_data.join("stretch-core").join("config.json");
    let stretchly = app_data.join("Stretchly").join("config.json");
    Settings::load_or_import(&own, &stretchly).unwrap_or_else(|error| {
        eprintln!("using defaults, {}: {error}", own.display());
        Settings::default()
    })
}
