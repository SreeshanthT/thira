//! Local logs and crash reports. Nothing here, or anywhere in Thira, sends data
//! off the machine (PRD: no telemetry).
//!
//! - Logs go to `thira.<date>.log` in the app's log directory, rotated daily
//!   and kept for a week. `THIRA_LOG` overrides the filter (`RUST_LOG` syntax).
//! - A panic writes `crash-<unix time>-<pid>.log` with the message, location
//!   and backtrace to the same directory, synchronously, before the default
//!   hook runs, so the report exists even if the process then aborts.

use std::backtrace::Backtrace;
use std::fmt::Write as _;
use std::fs;
use std::io;
use std::panic::PanicHookInfo;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::util::SubscriberInitExt;

/// Default filter: our own logs at info, GPU stack chatter only when it matters.
const DEFAULT_FILTER: &str = "info,wgpu_core=warn,wgpu_hal=warn,naga=warn";
const KEEP_LOG_FILES: usize = 7;

/// Where crash reports go; set once the app knows its log directory.
static CRASH_DIR: OnceLock<PathBuf> = OnceLock::new();

/// Keeps the background log writer alive; dropping it flushes the log.
#[derive(Debug)]
pub struct LogGuard(#[allow(dead_code)] WorkerGuard);

/// Builds the file-logging subscriber for `dir` without installing it.
fn subscriber(
    dir: &Path,
    filter: EnvFilter,
) -> io::Result<(
    impl tracing::Subscriber + Send + Sync + 'static,
    WorkerGuard,
)> {
    let appender = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix("thira")
        .filename_suffix("log")
        .max_log_files(KEEP_LOG_FILES)
        .build(dir)
        .map_err(io::Error::other)?;
    let (writer, guard) = tracing_appender::non_blocking(appender);
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(writer)
        .with_ansi(false)
        .finish();
    Ok((subscriber, guard))
}

/// Starts logging to `dir` and sends crash reports there too. Also captures
/// `log` records from dependencies (Tauri, wgpu).
pub fn init(dir: &Path) -> io::Result<LogGuard> {
    fs::create_dir_all(dir)?;
    let _ = CRASH_DIR.set(dir.to_path_buf());
    let filter =
        EnvFilter::try_from_env("THIRA_LOG").unwrap_or_else(|_| EnvFilter::new(DEFAULT_FILTER));
    let (subscriber, guard) = subscriber(dir, filter)?;
    subscriber.try_init().map_err(io::Error::other)?;
    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        os = std::env::consts::OS,
        arch = std::env::consts::ARCH,
        log_dir = %dir.display(),
        "Thira started"
    );
    Ok(LogGuard(guard))
}

/// Installs the crash-report panic hook, keeping the default hook after it.
/// Call first thing; until [`init`] runs, reports go to the temp directory.
pub fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let dir = CRASH_DIR
            .get()
            .cloned()
            .unwrap_or_else(|| std::env::temp_dir().join("thira"));
        match write_crash_report(&dir, info) {
            Ok(path) => tracing::error!(report = %path.display(), "panic: {}", message(info)),
            Err(err) => eprintln!("could not write a crash report to {}: {err}", dir.display()),
        }
        default_hook(info);
    }));
}

fn message(info: &PanicHookInfo<'_>) -> String {
    let payload = info.payload();
    payload
        .downcast_ref::<&str>()
        .map(|s| (*s).to_owned())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "non-string panic payload".to_owned())
}

fn write_crash_report(dir: &Path, info: &PanicHookInfo<'_>) -> io::Result<PathBuf> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default();
    let thread = std::thread::current();
    let mut report = String::new();
    // Writing to a String cannot fail.
    let _ = writeln!(report, "Thira {} crashed", env!("CARGO_PKG_VERSION"));
    let _ = writeln!(report, "time: {now} (seconds since 1970-01-01 UTC)");
    let _ = writeln!(
        report,
        "os: {} {}",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    let _ = writeln!(report, "thread: {}", thread.name().unwrap_or("<unnamed>"));
    let _ = writeln!(report, "message: {}", message(info));
    if let Some(location) = info.location() {
        let _ = writeln!(report, "location: {location}");
    }
    let _ = writeln!(report, "\nbacktrace:\n{}", Backtrace::force_capture());

    fs::create_dir_all(dir)?;
    let path = dir.join(format!("crash-{now}-{}.log", std::process::id()));
    fs::write(&path, report)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    use super::*;

    const CHILD_DIR: &str = "THIRA_TEST_CRASH_DIR";

    /// Runs in a child process (see the test below); a no-op otherwise.
    #[test]
    fn crash_child() {
        let Some(dir) = std::env::var_os(CHILD_DIR) else {
            return;
        };
        let _ = CRASH_DIR.set(PathBuf::from(dir));
        install_panic_hook();
        panic!("deliberate test crash");
    }

    #[test]
    fn a_panic_writes_a_crash_report() {
        let dir = tempfile::tempdir().expect("temp dir");
        let child = Command::new(std::env::current_exe().expect("test binary"))
            .args(["--exact", "diagnostics::tests::crash_child", "--nocapture"])
            .env(CHILD_DIR, dir.path())
            .output() // captured, so its panic output stays out of the test log
            .expect("run child");
        assert!(!child.status.success(), "the child should have panicked");

        let reports: Vec<_> = fs::read_dir(dir.path())
            .expect("read dir")
            .map(|e| e.expect("entry").path())
            .collect();
        assert_eq!(reports.len(), 1, "{reports:?}");
        let name = reports[0]
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        assert!(
            name.starts_with("crash-") && name.ends_with(".log"),
            "{name}"
        );
        let report = fs::read_to_string(&reports[0]).expect("read report");
        assert!(
            report.contains("message: deliberate test crash"),
            "{report}"
        );
        assert!(report.contains("location: app"), "{report}");
        assert!(report.contains("backtrace:"), "{report}");
    }

    #[test]
    fn logs_go_to_a_daily_file_in_the_log_dir() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (subscriber, guard) =
            subscriber(dir.path(), EnvFilter::new("info")).expect("subscriber");
        tracing::subscriber::with_default(subscriber, || {
            tracing::info!("hello from the test");
            tracing::debug!("filtered out");
        });
        drop(guard); // flushes

        let logs: Vec<_> = fs::read_dir(dir.path())
            .expect("read dir")
            .map(|e| e.expect("entry").path())
            .collect();
        assert_eq!(logs.len(), 1, "{logs:?}");
        let name = logs[0]
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        assert!(
            name.starts_with("thira.") && name.ends_with(".log"),
            "{name}"
        );
        let text = fs::read_to_string(&logs[0]).expect("read log");
        assert!(text.contains("hello from the test"), "{text}");
        assert!(!text.contains("filtered out"), "{text}");
    }
}
