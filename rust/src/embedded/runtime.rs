use crate::{
  api::device_config_manager::DEVICE_CONFIG_MANAGER,
  embedded::{frontend::EmbeddedIntifaceEngineFrontend, logging},
  mobile_init,
};
use anyhow::{Result, anyhow};
use intiface_engine::{EngineOptionsExternal, IntifaceEngine};
use log::*;
use parking_lot::Mutex;
use std::{
  net::TcpStream,
  sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
  },
  thread::{self, JoinHandle},
  time::{Duration, Instant},
};
use tokio::sync::Notify;

const STARTUP_TIMEOUT: Duration = Duration::from_secs(15);
const ENGINE_CLEANUP_TIMEOUT: Duration = Duration::from_secs(10);

struct EngineState {
  running: Option<RunningEngine>,
}

struct RunningEngine {
  stop_requested: Arc<AtomicBool>,
  stop_notify: Arc<Notify>,
  thread: JoinHandle<()>,
}

static ENGINE: Mutex<EngineState> = Mutex::new(EngineState { running: None });

/// Starts the engine and blocks until its websocket server is ready (or fails).
pub fn run_engine(args: EngineOptionsExternal) -> Result<()> {
  logging::init();

  let engine = Arc::new(IntifaceEngine::default());
  let frontend = Arc::new(EmbeddedIntifaceEngineFrontend::new(Arc::new(
    tokio::sync::broadcast::channel(255).0,
  )));
  let options = args.clone().into();
  let dcm = (*DEVICE_CONFIG_MANAGER.read()).clone();
  let stop_requested = Arc::new(AtomicBool::new(false));
  let stop_notify = Arc::new(Notify::new());

  let runtime = mobile_init::create_runtime(None)?;

  let mut guard = ENGINE.lock();
  if guard.running.is_some() {
    return Err(anyhow!("Server already running!"));
  }

  let run_engine = engine.clone();
  let stop_engine = engine.clone();
  let thread_stop_requested = stop_requested.clone();
  let thread_stop_notify = stop_notify.clone();
  let thread = thread::Builder::new()
    .name("intiface-embedded-engine".to_owned())
    .spawn(move || {
      runtime.block_on(async move {
        tokio::join!(
          async {
            if let Err(err) = run_engine
              .run(&options, Some(frontend), &Some(dcm))
              .await
            {
              error!("Error running engine: {err:?}");
            }
          },
          async {
            let notified = thread_stop_notify.notified();
            tokio::pin!(notified);
            if !thread_stop_requested.load(Ordering::Acquire) {
              notified.await;
            }
            stop_engine.stop();
          },
        );
      });
    })
    .map_err(|err| anyhow!("Failed to spawn engine thread: {err}"))?;

  let result = wait_for_server_ready(args.websocket_port, &stop_requested);
  match result {
    Ok(()) => {
      guard.running = Some(RunningEngine {
        stop_requested,
        stop_notify,
        thread,
      });
      info!(
        "Embedded Intiface Engine started on port {}",
        args.websocket_port
          .map(|port| port.to_string())
          .unwrap_or_else(|| "n/a".to_owned())
      );
      Ok(())
    }
    Err(err) => {
      stop_requested.store(true, Ordering::Release);
      stop_notify.notify_waiters();
      let _ = wait_for_thread(thread);
      Err(err)
    }
  }
}

/// Stops a running engine and blocks until it has cleaned up.
pub fn stop_engine() -> Result<()> {
  let running = {
    let mut guard = ENGINE.lock();
    guard.running.take()
  };
  let Some(running) = running else {
    info!("Embedded Intiface Engine is not running");
    return Ok(());
  };
  info!("Stopping embedded Intiface Engine");
  running.stop_requested.store(true, Ordering::Release);
  running.stop_notify.notify_waiters();
  wait_for_thread(running.thread)
}

fn wait_for_server_ready(
  port: Option<u16>,
  stop_requested: &AtomicBool,
) -> Result<()> {
  let Some(port) = port else {
    return Ok(());
  };
  let deadline = Instant::now() + STARTUP_TIMEOUT;
  while Instant::now() < deadline {
    if stop_requested.load(Ordering::Acquire) {
      return Err(anyhow!("Engine stopped before the server became ready"));
    }
    if TcpStream::connect(("127.0.0.1", port)).is_ok() {
      return Ok(());
    }
    thread::sleep(Duration::from_millis(100));
  }
  Err(anyhow!(
    "Engine server did not become ready within {STARTUP_TIMEOUT:?}"
  ))
}

fn wait_for_thread(thread: JoinHandle<()>) -> Result<()> {
  let deadline = Instant::now() + ENGINE_CLEANUP_TIMEOUT;
  while Instant::now() < deadline {
    if thread.is_finished() {
      thread.join().expect("finished thread must be joinable");
      return Ok(());
    }
    thread::sleep(Duration::from_millis(100));
  }
  Err(anyhow!("Engine did not stop within {ENGINE_CLEANUP_TIMEOUT:?}"))
}
