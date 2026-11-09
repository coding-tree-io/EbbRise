//! Windows adapters: signal window and timer, clocks, Focus Assist, toasts and single instance.
//! Each turns an OS feature into plain values; no domain types here (ADR 0002).

mod clock;
mod focus_assist;
mod instance;
mod signals;
pub mod toast;

pub use clock::{awake_ms, idle_ms};
pub use instance::is_first_instance;
pub use signals::{Signal, SignalWindow, quit, run_message_loop};
