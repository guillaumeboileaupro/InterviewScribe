// Without this, a release build on Windows opens a console window alongside
// the app window (the default subsystem for a Rust binary is "console") -
// kept for debug builds so standard output remains visible during development.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    interviewscribe_lib::run();
}
