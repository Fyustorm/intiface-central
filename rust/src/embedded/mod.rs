//! Flutter-free engine lifecycle used by the Android JNI bridge.
//!
//! This module intentionally does not depend on `flutter_rust_bridge` so that
//! the engine can be started and stopped from Kotlin without a Dart runtime.

pub mod frontend;
pub mod logging;
pub mod runtime;
