//! Compiles the Slint UI.

fn main() {
    slint_build::compile("ui/break_window.slint").expect("break window UI");
}
