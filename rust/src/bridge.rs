//! Android JNI bridge exposing the embedded engine lifecycle to Kotlin.
//!
//! These functions match the native declarations in
//! `io.intiface.central.IntifaceBridge`:
//!   - `startEngine(...)` -> Int (0 on success)
//!   - `stopEngine()` -> Int (0 on success)

use crate::api::runtime::EngineOptionsExternal;
use crate::embedded::{logging, runtime};
use jni::objects::JString;
use jni::sys::{jboolean, jint};
use jni::{JNIEnv, objects::JObject};
use log::{Level, error};

fn optional_string(env: &JNIEnv, value: JString) -> Option<String> {
  if value.is_null() {
    return None;
  }
  env.get_string(value).ok().map(Into::into)
}

fn optional_u16(value: jint) -> Option<u16> {
  if value < 0 {
    None
  } else {
    u16::try_from(value).ok()
  }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_io_intiface_central_IntifaceBridge_startEngine(
  env: JNIEnv,
  _this: JObject,
  server_name: JString,
  use_bluetooth_le: jboolean,
  use_serial_port: jboolean,
  use_hid: jboolean,
  use_lovense_dongle_serial: jboolean,
  use_lovense_dongle_hid: jboolean,
  use_xinput: jboolean,
  use_lovense_connect: jboolean,
  use_device_websocket_server: jboolean,
  use_simulated_devices: jboolean,
  websocket_use_all_interfaces: jboolean,
  websocket_port: jint,
  frontend_websocket_port: jint,
  frontend_in_process_channel: jboolean,
  max_ping_time: jint,
  device_config_json: JString,
  user_device_config_json: JString,
  user_device_config_path: JString,
  device_websocket_server_port: jint,
  crash_main_thread: jboolean,
  crash_task_thread: jboolean,
  broadcast_server_mdns: jboolean,
  mdns_suffix: JString,
  repeater_mode: jboolean,
  repeater_local_port: jint,
  repeater_remote_address: JString,
  rest_api_port: jint,
  emit_output_observations: jboolean,
  websocket_client_address: JString,
) -> jint {
  logging::init();

  let options = EngineOptionsExternal {
    server_name: env.get_string(server_name).map(Into::into).unwrap_or_default(),
    use_bluetooth_le: use_bluetooth_le != 0,
    use_serial_port: use_serial_port != 0,
    use_hid: use_hid != 0,
    use_lovense_dongle_serial: use_lovense_dongle_serial != 0,
    use_lovense_dongle_hid: use_lovense_dongle_hid != 0,
    use_xinput: use_xinput != 0,
    use_lovense_connect: use_lovense_connect != 0,
    use_device_websocket_server: use_device_websocket_server != 0,
    use_simulated_devices: use_simulated_devices != 0,
    websocket_use_all_interfaces: websocket_use_all_interfaces != 0,
    websocket_port: optional_u16(websocket_port),
    frontend_websocket_port: optional_u16(frontend_websocket_port),
    frontend_in_process_channel: frontend_in_process_channel != 0,
    max_ping_time: max_ping_time as u32,
    device_config_json: optional_string(&env, device_config_json),
    user_device_config_json: optional_string(&env, user_device_config_json),
    user_device_config_path: optional_string(&env, user_device_config_path),
    device_websocket_server_port: optional_u16(device_websocket_server_port),
    crash_main_thread: crash_main_thread != 0,
    crash_task_thread: crash_task_thread != 0,
    broadcast_server_mdns: broadcast_server_mdns != 0,
    mdns_suffix: optional_string(&env, mdns_suffix),
    repeater_mode: repeater_mode != 0,
    repeater_local_port: optional_u16(repeater_local_port),
    repeater_remote_address: optional_string(&env, repeater_remote_address),
    rest_api_port: optional_u16(rest_api_port),
    emit_output_observations: emit_output_observations != 0,
    websocket_client_address: optional_string(&env, websocket_client_address),
  };

  match runtime::run_engine(options) {
    Ok(()) => 0,
    Err(err) => {
      error!("Error starting engine: {err:?}");
      1
    }
  }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_io_intiface_central_IntifaceBridge_stopEngine(
  _env: JNIEnv,
  _this: JObject,
) -> jint {
  match runtime::stop_engine() {
    Ok(()) => 0,
    Err(err) => {
      error!("Error stopping engine: {err:?}");
      1
    }
  }
}

/// Sets the maximum log level forwarded to logcat. Mapping:
/// 0=Error, 1=Warn, 2=Info, 3=Debug, 4=Trace. Any other value resets to Info.
#[unsafe(no_mangle)]
pub extern "system" fn Java_io_intiface_central_IntifaceBridge_setLogLevel(
  _env: JNIEnv,
  _this: JObject,
  level: jint,
) -> jint {
  let level = match level {
    0 => Level::Error,
    1 => Level::Warn,
    2 => Level::Info,
    3 => Level::Debug,
    4 => Level::Trace,
    _ => Level::Info,
  };
  logging::set_level(level);
  0
}
