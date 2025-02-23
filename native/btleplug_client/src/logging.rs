use std::env;
use log::{debug, error, info, warn, LevelFilter};
use pretty_env_logger::env_logger;
use env_logger::{Builder, Target};
use atty::Stream;
use std::io::Write;

pub fn init_log() {
    if atty::is(Stream::Stdout) {
        init_log_cli();
    } else {
        init_log_phoenix();
    }
}

/// **🎨 CLI Mode:** Full-color logs for interactive terminals.
fn init_log_cli() {
    let log_level = env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()); // Read env var
    let filter = log_level.parse().unwrap_or(LevelFilter::Info); // Fallback if parsing fails

    Builder::new()
        .format(|buf, record| {
            let level_style = buf.default_level_style(record.level()); // Apply colors
            writeln!(
                buf,
                "{} [{}] {}:{} - {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                level_style.value(record.level()), // Colored log level
                record.file().unwrap_or("unknown"),
                record.line().unwrap_or(0),
                record.args()
            )
        })
        .filter_level(filter) // ✅ Explicitly set the filter level
        .target(Target::Stdout)
        .init();
}

/// **🌐 Phoenix Mode:** Cleaner logs (no colors, better formatting for Elixir).
fn init_log_phoenix() {
    let log_level = env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    let filter = log_level.parse().unwrap_or(LevelFilter::Info);

    Builder::new()
        .format(|buf, record| {
            writeln!(
                buf,
                "[{}] {}:{} [{}] - {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.file().unwrap_or("unknown"),
                record.line().unwrap_or(0),
                record.level(),
                record.args()
            )
        })
        .filter_level(filter) // ✅ Explicitly set the filter level
        .target(Target::Stdout)
        .init();
}
