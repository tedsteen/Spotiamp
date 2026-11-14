// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    env_logger::init();
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("A crypto provider");
    if std::env::args().nth(1).as_deref() == Some(spotiamp_lib::account::ACCOUNT_CHECK_ARG) {
        spotiamp_lib::account::run_account_check();
    }
    spotiamp_lib::run();
}
