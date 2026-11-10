//! Windows adapters: signal window and timer, clocks, Focus Assist, running processes, toasts,
//! sounds, autostart, single instance, the input block during breaks and one HTTPS GET.
//! Each turns an OS feature into plain values; no domain types here (ADR 0002).

mod autostart;
mod clock;
mod focus_assist;
mod front;
mod http;
mod input_block;
mod instance;
mod processes;
mod screen;
mod signals;
pub mod sound;
mod title_bar;
pub mod toast;

pub use autostart::{autostart_command, set_autostart};
pub use clock::{awake_ms, idle_ms};
pub use front::bring_to_front;
pub use http::https_get;
pub use input_block::InputBlock;
pub use instance::{attach_parent_console, is_first_instance};
pub use processes::running_process_names;
pub use screen::{primary_screen_size, small_icon_size};
pub use signals::{Signal, SignalWindow, running_app_window, send_command};
pub use title_bar::title_bar;
