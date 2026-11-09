//! Composition root: loads settings, connects Windows signals and the tray to the planner,
//! and carries out what the planner returns. Everything runs on the main thread.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod logging;
mod logic;

use std::cell::RefCell;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use breaks::until_morning;
use commands::Command;
use i18n::Translator;
use platform::{Signal, SignalWindow, awake_ms, idle_ms, toast};
use settings::Settings;
use tracing::{error, info, warn};
use tray_icon::menu::{IsMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder, TrayIconEvent};

use crate::logic::{Core, Environment, MenuAction};

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
    if let Some(folder) = app_folder() {
        logging::start(&log_path(&folder));
    }
    info!(version = env!("CARGO_PKG_VERSION"), "starting");
    let settings = load_settings();
    if let Err(error) = toast::register(APP_ID, "Stretch") {
        error!(%error, "toast registration failed");
    }
    let signals =
        SignalWindow::new(|signal| with_app(|app| app.on_signal(signal))).expect("signal window");
    let core = Core::new(settings, awake_ms(), signals.focus_assist);

    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(tray_menu(core.words())))
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
        });
    });
    with_app(App::rearm_timer_and_refresh_tray);
    platform::run_message_loop();
}

impl App {
    fn on_signal(&mut self, signal: Signal) {
        if signal != Signal::Timer {
            info!(?signal, "signal");
        }
        for toast in self.core.on_signal(signal, awake_ms(), idle_ms(), &Windows) {
            info!(title = %toast.title, "toast");
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
        info!(?action, "menu");
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

/// Upstream's tray menu, without the items that need a window (Preferences).
fn tray_menu(words: &Translator) -> Menu {
    let item =
        |action: MenuAction, key: &str| MenuItem::with_id(action.id(), words.text(key), true, None);
    let submenu = |key: &str, items: &[&dyn IsMenuItem]| {
        Submenu::with_items(words.text(key), true, items).expect("tray submenu")
    };
    let pause_hour = MenuItem::with_id(
        MenuAction::PauseHour.id(),
        words.count("utils.hours", 1),
        true,
        None,
    );
    Menu::with_items(&[
        &submenu(
            "main.skipToTheNext",
            &[
                &item(MenuAction::SkipToMini, "main.toMiniBreak"),
                &item(MenuAction::SkipToLong, "main.toLongBreak"),
            ],
        ),
        &item(MenuAction::Resume, "main.resume"),
        &submenu(
            "main.pause",
            &[
                &pause_hour,
                &item(MenuAction::PauseUntilMorning, "main.untilMorning"),
                &PredefinedMenuItem::separator(),
                &item(MenuAction::PauseIndefinitely, "main.indefinitely"),
            ],
        ),
        &item(MenuAction::Reset, "main.resetBreaks"),
        &PredefinedMenuItem::separator(),
        &item(MenuAction::Quit, "main.quitStretchly"),
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
