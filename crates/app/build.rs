//! Compiles the Slint UI.

fn main() {
    slint_build::compile("ui/main.slint").expect("Slint UI");
}
