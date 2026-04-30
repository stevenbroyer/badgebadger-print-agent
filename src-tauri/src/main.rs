// Prevents additional console window on Windows in release. Without this
// the agent spawns a black cmd.exe alongside the GUI which is jarring
// for non-technical users.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    badgebadger_print_agent_lib::run();
}
