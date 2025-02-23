use env_logger::{Builder, Target};
use log::{info, LevelFilter};
use pretty_env_logger::env_logger;
use std::env;
use std::io::{Write};

pub fn init_log() {
    match get_log_mode() {
        LogMode::Cli => {
            info!("Initializing in CLI mode (interactive terminal)");
            init_log_cli();
        }
        LogMode::Phoenix => {
            info!("Initializing in Phoenix mode (non-interactive terminal)");
            init_log_phoenix();
        }
    }
}

/// Determines the logging mode based on the `RUST_LOG_MODE` environment variable.
/// Defaults to CLI if not set or set to "cli".
fn get_log_mode() -> LogMode {
    match env::var("RUST_LOG_MODE")
        .unwrap_or_else(|_| "cli".to_string())
        .as_str()
    {
        "phoenix" => LogMode::Phoenix,
        _ => LogMode::Cli, // Default to CLI if not set or if set to something other than "phoenix"
    }
}

#[derive(Debug)]
enum LogMode {
    Cli,
    Phoenix,
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

fn init_log_phoenix() {
    let log_level = env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    let filter = log_level.parse().unwrap_or(LevelFilter::Info);

    let max_width = env::var("RUST_LOG_MAXWIDTH")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(100); // Default line width

    Builder::new()
        .format(move |buf, record| {
            let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
            let file = record.file().unwrap_or("unknown");
            let line = record.line().unwrap_or(0);
            let level = record.level();
            let message = record.args().to_string();

            // **Create log entry**
            let log_entry = format!(
                "[{}] {}:{} [{}] - {}",
                timestamp, file, line, level, message
            );

            // **Create a longer lived value to avoid temporary issue**
            let binding = log_entry.chars().collect::<Vec<_>>();
            let chunks = binding.chunks(max_width);

            // **Write each chunk with a newline** (no over-splitting!)
            for chunk in chunks {
                let formatted_chunk = chunk.iter().collect::<String>();
                writeln!(buf, "{}", formatted_chunk)?; // Proper line break
            }

            buf.flush()?; // **✅ Flush log output immediately** (flush only once)
            Ok(())
        })
        .filter_level(filter)
        .target(Target::Stdout)
        .init();
}
