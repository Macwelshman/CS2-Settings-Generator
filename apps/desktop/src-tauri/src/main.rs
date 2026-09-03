#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if cs2_settings_desktop_lib::updates::run_helper() {
        return;
    }
    cs2_settings_desktop_lib::run();
}
