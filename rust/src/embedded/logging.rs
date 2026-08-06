use log::{Level, LevelFilter, Log, Metadata, Record};
use std::sync::atomic::{AtomicU8, Ordering};
use tracing::field::{Field, Visit};
use tracing_subscriber::layer::{Context, SubscriberExt};
use tracing_subscriber::util::SubscriberInitExt;

/// Targets that log per-message noise at `Info`. They are capped to `Warn`
/// regardless of the configured global level.
const NOISY_TARGETS: &[&str] = &["buttplug_server::message::serializer"];

/// Default maximum level. The `log` crate's `Level` discriminant order is
/// Error=0, Warn=1, Info=2, Debug=3, Trace=4.
static MAX_LEVEL: AtomicU8 = AtomicU8::new(Level::Info as u8);

fn is_noisy_target(target: &str) -> bool {
  NOISY_TARGETS.iter().any(|prefix| target.starts_with(prefix))
}

fn is_enabled(level: Level, target: &str) -> bool {
  if (level as u8) > MAX_LEVEL.load(Ordering::Relaxed) {
    return false;
  }
  if is_noisy_target(target) && (level as u8) > (Level::Warn as u8) {
    return false;
  }
  true
}

fn write_line(level: Level, target: &str, message: &str) {
  #[cfg(target_os = "android")]
  android_log::write(level, &format!("[{level}] {target}: {message}"));
  #[cfg(not(target_os = "android"))]
  eprintln!("[{level}] {target}: {message}");
}

fn emit(level: Level, target: &str, message: &str) {
  if !is_enabled(level, target) {
    return;
  }
  write_line(level, target, message);
}

struct EmbeddedLogger;

impl Log for EmbeddedLogger {
  fn enabled(&self, metadata: &Metadata) -> bool {
    is_enabled(metadata.level(), metadata.target())
  }

  fn log(&self, record: &Record) {
    emit(
      record.level(),
      record.target(),
      &format!("{}", record.args()),
    );
  }

  fn flush(&self) {}
}

/// Forwards `tracing` events through the same level/noise filtering as
/// `log!` records. Span lifecycle is intentionally ignored: with the `log`
/// feature enabled, `tracing` emits one `log` record per span *creation* at
/// the span's level when no subscriber is set (that is the per-message logcat
/// flood from `info_span!("Buttplug Server Message", id = id)`), and installing
/// a subscriber stops that fallback.
struct EmbeddedTracingLayer;

impl<S> tracing_subscriber::Layer<S> for EmbeddedTracingLayer
where
  S: tracing::Subscriber,
{
  fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
    let metadata = event.metadata();
    let Some(level) = to_log_level(metadata.level()) else {
      return;
    };
    let mut visitor = FieldVisitor::default();
    event.record(&mut visitor);
    let message = visitor.message.unwrap_or_default();
    let line = if visitor.fields.is_empty() {
      message
    } else {
      let fields = visitor
        .fields
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join(", ");
      format!("{message}; {fields}")
    };
    emit(level, metadata.target(), &line);
  }
}

#[derive(Default)]
struct FieldVisitor {
  message: Option<String>,
  fields: Vec<(String, String)>,
}

impl Visit for FieldVisitor {
  fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
    self
      .fields
      .push((field.name().to_string(), format!("{value:?}")));
  }

  fn record_str(&mut self, field: &Field, value: &str) {
    if field.name() == "message" {
      self.message = Some(value.to_string());
    } else {
      self.fields.push((field.name().to_string(), value.to_string()));
    }
  }
}

fn to_log_level(level: &tracing::Level) -> Option<Level> {
  if level == &tracing::Level::ERROR {
    Some(Level::Error)
  } else if level == &tracing::Level::WARN {
    Some(Level::Warn)
  } else if level == &tracing::Level::INFO {
    Some(Level::Info)
  } else if level == &tracing::Level::DEBUG {
    Some(Level::Debug)
  } else if level == &tracing::Level::TRACE {
    Some(Level::Trace)
  } else {
    None
  }
}

/// Sets the maximum log level sent to logcat/stderr.
pub fn set_level(level: Level) {
  MAX_LEVEL.store(level as u8, Ordering::Relaxed);
}

/// Initializes the embedded engine logger and tracing subscriber. Safe to call
/// multiple times.
pub fn init() {
  static INIT: std::sync::Once = std::sync::Once::new();
  INIT.call_once(|| {
    let _ = log::set_boxed_logger(Box::new(EmbeddedLogger));
    // The `log` macros gate on `log::max_level()`, which never consults a
    // custom logger's `enabled()`. Set it to the maximum so every record
    // reaches `EmbeddedLogger::log`, where the real filtering happens.
    log::set_max_level(LevelFilter::Trace);
    // Install a real `tracing` subscriber. Without one, the `tracing` "log"
    // feature falls back to the `log` crate and emits a record per span
    // creation at the span's level, flooding logcat for per-message spans.
    // Span lifecycle is consumed silently; events are forwarded through the
    // same filtering as `log!` records.
    let _ = tracing_subscriber::registry()
      .with(EmbeddedTracingLayer)
      .try_init();
  });
}

#[cfg(target_os = "android")]
mod android_log {
  use log::Level;
  use std::ffi::CString;

  const ANDROID_LOG_DEBUG: i32 = 3;
  const ANDROID_LOG_INFO: i32 = 4;
  const ANDROID_LOG_WARN: i32 = 5;
  const ANDROID_LOG_ERROR: i32 = 6;
  const TAG: &[u8] = b"IntifaceEngine\0";

  unsafe extern "C" {
    fn __android_log_write(priority: i32, tag: *const u8, message: *const u8) -> i32;
  }

  pub fn write(level: Level, message: &str) {
    let priority = match level {
      Level::Error => ANDROID_LOG_ERROR,
      Level::Warn => ANDROID_LOG_WARN,
      Level::Info => ANDROID_LOG_INFO,
      Level::Debug | Level::Trace => ANDROID_LOG_DEBUG,
    };
    let message = CString::new(message).unwrap_or_default();
    unsafe {
      __android_log_write(priority, TAG.as_ptr(), message.as_ptr());
    }
  }
}
