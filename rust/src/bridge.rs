//! Android JNI bridge exposing the embedded engine lifecycle to Kotlin.
//!
//! These functions match the native declarations in
//! `io.intiface.central.IntifaceBridge`:
//!   - `startEngine(...)` -> Int (0 on success)
//!   - `stopEngine()` -> Int (0 on success)

use crate::api::runtime::EngineOptionsExternal;
use crate::embedded::{logging, runtime};
use jni::errors::LogErrorAndDefault;
use jni::objects::{JObject, JString};
use jni::sys::jint;
use jni::{Env, EnvUnowned};
use log::{Level, error};

fn optional_string(env: &Env, value: &JString) -> jni::errors::Result<Option<String>> {
  if value.is_null() {
    return Ok(None);
  }
  value.try_to_string(env).map(Some)
}

fn optional_u16(value: jint) -> Option<u16> {
  if value < 0 {
    None
  } else {
    u16::try_from(value).ok()
  }
}

// The Kotlin side (io.intiface.central.IntifaceBridge) keeps the signature of the engine 4 options:
// the address the engine listens on is built here from its port and its all-interfaces flag, the
// HID manager no longer exists, and the XInput one became the SDL gamepad one.
#[unsafe(no_mangle)]
pub extern "system" fn Java_io_intiface_central_IntifaceBridge_startEngine<'caller>(
  mut unowned_env: EnvUnowned<'caller>,
  _this: JObject<'caller>,
  server_name: JString<'caller>,
  use_bluetooth_le: bool,
  use_serial_port: bool,
  _use_hid: bool,
  use_lovense_dongle_serial: bool,
  use_lovense_dongle_hid: bool,
  use_xinput: bool,
  use_lovense_connect: bool,
  use_device_websocket_server: bool,
  use_simulated_devices: bool,
  websocket_use_all_interfaces: bool,
  websocket_port: jint,
  frontend_websocket_port: jint,
  frontend_in_process_channel: bool,
  max_ping_time: jint,
  device_config_json: JString<'caller>,
  user_device_config_json: JString<'caller>,
  user_device_config_path: JString<'caller>,
  device_websocket_server_port: jint,
  crash_main_thread: bool,
  crash_task_thread: bool,
  broadcast_server_mdns: bool,
  mdns_suffix: JString<'caller>,
  repeater_mode: bool,
  repeater_local_port: jint,
  repeater_remote_address: JString<'caller>,
  rest_api_port: jint,
  emit_output_observations: bool,
  websocket_client_address: JString<'caller>,
) -> jint {
  logging::init();

  let websocket_listen_address = optional_u16(websocket_port).map(|port| {
    let host = if websocket_use_all_interfaces { "0.0.0.0" } else { "127.0.0.1" };
    format!("{host}:{port}")
  });

  let options = unowned_env
    .with_env(|env| -> jni::errors::Result<Option<EngineOptionsExternal>> {
      Ok(Some(EngineOptionsExternal {
        server_name: optional_string(env, &server_name)?.unwrap_or_default(),
        websocket_listen_address: websocket_listen_address.clone(),
        websocket_client_address: optional_string(env, &websocket_client_address)?,
        frontend_websocket_port: optional_u16(frontend_websocket_port),
        frontend_in_process_channel,
        max_ping_time: max_ping_time.max(0) as u32,
        use_bluetooth_le,
        use_serial_port,
        use_lovense_dongle_serial,
        use_lovense_dongle_hid,
        use_sdl_gamepad: use_xinput,
        use_lovense_connect,
        use_device_websocket_server,
        use_simulated_devices,
        device_config_json: optional_string(env, &device_config_json)?,
        user_device_config_json: optional_string(env, &user_device_config_json)?,
        user_device_config_path: optional_string(env, &user_device_config_path)?,
        device_websocket_server_port: optional_u16(device_websocket_server_port),
        crash_main_thread,
        crash_task_thread,
        broadcast_server_mdns,
        mdns_suffix: optional_string(env, &mdns_suffix)?,
        repeater_mode,
        repeater_local_port: optional_u16(repeater_local_port),
        repeater_remote_address: optional_string(env, &repeater_remote_address)?,
        rest_api_port: optional_u16(rest_api_port),
        emit_output_observations,
      }))
    })
    .resolve::<LogErrorAndDefault>();
  let Some(options) = options else {
    error!("Error reading the engine options from Java");
    return 1;
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
pub extern "system" fn Java_io_intiface_central_IntifaceBridge_stopEngine<'caller>(
  _env: EnvUnowned<'caller>,
  _this: JObject<'caller>,
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
pub extern "system" fn Java_io_intiface_central_IntifaceBridge_setLogLevelNative<'caller>(
  _env: EnvUnowned<'caller>,
  _this: JObject<'caller>,
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
