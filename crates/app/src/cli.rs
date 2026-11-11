//! Command-line actions and forwarding to the running instance.

use crate::storage::{app_folder, log_path};
use commands::Command;

/// Answers commands that do not need an app instance.
pub(crate) fn handle_local_command(args: &[String]) -> bool {
    let words: Vec<&str> = args.iter().map(String::as_str).collect();
    let Some(command) = Command::parse(&words) else {
        platform::attach_parent_console();
        eprintln!("command '{}' is not supported", words.join(" "));
        return true;
    };
    if !matches!(command.name.as_str(), "help" | "version" | "logs") {
        return false;
    }
    platform::attach_parent_console();
    match command.name.as_str() {
        "help" => println!("{}", commands::HELP),
        "version" => println!("EbbRise version {}", env!("CARGO_PKG_VERSION")),
        "logs" => match app_folder() {
            Some(folder) => println!("{}", log_path(&folder).display()),
            None => eprintln!("APPDATA is not set, so there is no log file"),
        },
        _ => unreachable!(),
    }
    true
}

/// Passes a command to the running app.
pub(crate) fn run_second_instance(args: &[String]) {
    let taken =
        platform::running_app_window().is_some_and(|window| platform::send_command(window, args));
    if !taken {
        platform::attach_parent_console();
        eprintln!("the running EbbRise app did not take the command");
    }
}
