use crate::atoms;
use crate::peripheral::PeripheralRef;
use crate::peripheral::PeripheralState;
use rustler::{Atom, Encoder, Env, Error as RustlerError, LocalPid, OwnedEnv, ResourceArc, Term};

use btleplug::api::{
    bleuuid::BleUuid, Central, CentralEvent, CharPropFlags, Characteristic, Manager as _,
    Peripheral, ScanFilter, Service, ValueNotification,
};
use btleplug::platform::{Adapter, Manager};
use futures::StreamExt;
use tokio::spawn;
use uuid::Uuid;

use crate::RUNTIME;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};

pub struct CentralRef(pub(crate) Arc<Mutex<CentralManagerState>>);

pub struct CentralManagerState {
    pub pid: LocalPid,
    pub adapter: Adapter,
    pub manager: Manager,
    pub event_sender: mpsc::Sender<CentralEvent>,
    pub event_receiver: Arc<RwLock<mpsc::Receiver<CentralEvent>>>,
}

impl CentralManagerState {
    pub fn new(
        pid: LocalPid,
        manager: Manager,
        adapter: Adapter,
        event_sender: mpsc::Sender<CentralEvent>,
        event_receiver: Arc<RwLock<mpsc::Receiver<CentralEvent>>>,
    ) -> Self {
        CentralManagerState {
            pid,
            manager,
            adapter,
            event_sender,
            event_receiver,
        }
    }
}

#[rustler::nif]
pub fn create_central(env: Env) -> Result<ResourceArc<CentralRef>, RustlerError> {
    println!("[Rust] Creating CentralManager...");

    let manager = RUNTIME
        .block_on(Manager::new())
        .map_err(|e| RustlerError::Term(Box::new(format!("Manager error: {}", e))))?;

    let adapters = RUNTIME
        .block_on(manager.adapters())
        .map_err(|e| RustlerError::Term(Box::new(format!("Adapter error: {}", e))))?;

    if adapters.is_empty() {
        return Err(RustlerError::Term(Box::new("No available adapter")));
    }

    let adapter = adapters.into_iter().next().unwrap();
    let adapter_info = RUNTIME.block_on(adapter.adapter_info());
    println!("[Rust] Adapter initialized: {:?}", adapter_info);

    let (event_sender, event_receiver) = mpsc::channel::<CentralEvent>(100);
    let event_receiver = Arc::new(RwLock::new(event_receiver));
    let event_receiver_clone = event_receiver.clone();
    let event_sender_clone = event_sender.clone();

    let state = CentralManagerState::new(
        env.pid(),
        manager,
        adapter.clone(),
        event_sender,
        event_receiver,
    );
    let resource = ResourceArc::new(CentralRef(Arc::new(Mutex::new(state))));
    let pid = env.pid();

    // Spawn a task to handle adapter events
    RUNTIME.spawn(async move {
        println!("[Rust] Starting adapter event handler...");
        let mut events = match adapter.events().await {
            Ok(events) => events,
            Err(e) => {
                println!("[Rust] Failed to get adapter events: {:?}", e);
                return;
            }
        };

        while let Some(event) = events.next().await {
            println!("[Rust] Received adapter event: {:?}", event);
            if let Err(e) = event_sender_clone.send(event).await {
                println!("[Rust] Failed to forward event: {:?}", e);
                break;
            }
        }
        println!("[Rust] Adapter event handler closed");
    });

    RUNTIME.spawn(async move {
        println!("[Rust] Starting event receiver handler...");
        let mut receiver = event_receiver_clone.write().await;

        while let Some(event) = receiver.recv().await {
            let mut msg_env = OwnedEnv::new();
            match event {
                CentralEvent::DeviceDiscovered(id) => {
                    let uuid = id.to_string();
                    println!("[Rust] Device discovered - UUID: {}", uuid);
                    match msg_env.send_and_clear(&pid, |env| {
                        (atoms::btleplug_device_discovered(), uuid).encode(env)
                    }) {
                        Ok(_) => println!("[Rust] Successfully sent device discovery message to Elixir process"),
                        Err(e) => println!(
                            "[Rust] Failed to send device discovery message to Elixir process (Error: {:?}. \
                            This might happen if the Elixir process has terminated.",
                            e
                        ),
                    }
                }
                CentralEvent::ManufacturerDataAdvertisement {
                    id,
                    manufacturer_data,
                } => {
                    println!("DEBUG: Manufacturer data from device ID: {:?}", id);
                    if let Err(e) = msg_env.send_and_clear(&pid, |env| {
                        (
                            atoms::btleplug_manufacturer_data_advertisement(),
                            format!("Manufacturer data from: {:?}", id),
                            manufacturer_data,
                        )
                            .encode(env)
                    }) {
                        println!("[Rust] Failed to send manufacturer data message: {:?}", e);
                    }
                }
                _ => {
                    println!("[Rust] Other event: {:?}", event);
                }
            }
        }
        println!("[Rust] Event receiver closed.");
    });

    Ok(resource)
}

#[rustler::nif]
pub fn start_scan(
    env: Env,
    resource: ResourceArc<CentralRef>,
    duration_ms: u64,
) -> Result<ResourceArc<CentralRef>, RustlerError> {
    println!("[Rust] Starting BLE scan for {} ms...", duration_ms);

    let resource_arc = resource.0.clone();
    let resource_arc_stop = resource_arc.clone();

    RUNTIME.spawn(async move {
        let mut msg_env = OwnedEnv::new();

        let adapter = {
            let central_state = resource_arc.lock().unwrap();
            central_state.adapter.clone()
        };

        let pid = {
            let central_state = resource_arc.lock().unwrap();
            central_state.pid.clone()
        };

        if let Err(e) = adapter.start_scan(ScanFilter::default()).await {
            println!("[Rust] Failed to start scan: {:?}", e);
            return;
        }
        msg_env.send_and_clear(&pid, |env| {
            (
                atoms::btleplug_scan_started(),
                format!("Scan started: {:?} ms", duration_ms),
            )
                .encode(env)
        });

        println!("[Rust] Scan started successfully");

        // Wait for the specified duration
        sleep(Duration::from_millis(duration_ms)).await;

        // Stop the scan after timeout
        let adapter = {
            let central_state = resource_arc_stop.lock().unwrap();
            central_state.adapter.clone()
        };

        if let Err(e) = adapter.stop_scan().await {
            println!("[Rust] Failed to stop scan after timeout: {:?}", e);
            return;
        }

        msg_env.send_and_clear(&pid, |env| {
            (
                atoms::btleplug_scan_stopped(),
                format!("Scan stopped after timeout: {:?} ms", duration_ms),
            )
                .encode(env)
        });

        println!("[Rust] Scan stopped automatically after {} ms", duration_ms);
    });

    Ok(resource)
}

#[rustler::nif]
pub fn stop_scan(
    resource: ResourceArc<CentralRef>,
) -> Result<ResourceArc<CentralRef>, RustlerError> {
    println!("[Rust] Stopping BLE scan...");

    let resource_arc = resource.0.clone();

    RUNTIME.spawn(async move {
        let adapter = {
            let central_state = resource_arc.lock().unwrap();
            central_state.adapter.clone()
        };

        if let Err(e) = adapter.stop_scan().await {
            println!("[Rust] Failed to stop scan: {:?}", e);
            return;
        }
        println!("[Rust] Scan stopped successfully");
    });
    Ok(resource)
}

#[rustler::nif]
pub fn find_peripheral(
    env: Env,
    resource: ResourceArc<CentralRef>,
    uuid: String,
) -> Result<ResourceArc<PeripheralRef>, RustlerError> {
    println!("[Rust] Finding peripheral with UUID: {}", uuid);

    let resource_arc = resource.0.clone();

    let adapter = {
        let central_state = resource_arc.lock().unwrap();
        central_state.adapter.clone()
    };

    let peripherals = RUNTIME.block_on(async {
        adapter
            .peripherals()
            .await
            .map_err(|e| RustlerError::Term(Box::new(format!("Failed to get peripherals: {}", e))))
    })?;

    // Find the peripheral with matching UUID
    for peripheral in peripherals {
        println!(
            "[Rust] Iterating peripheral peripheral.id(): {:?}, uuid: {:?}",
            peripheral.id(),
            uuid
        );
        if peripheral.id().to_string() == uuid {
            println!("[Rust] Found peripheral: {:?}", peripheral.id());
            let peripheral_state = PeripheralState::new(env.pid(), peripheral);
            return Ok(ResourceArc::new(PeripheralRef(Arc::new(Mutex::new(
                peripheral_state,
            )))));
        }
    }

    Err(RustlerError::Term(Box::new("Peripheral not found")))
}
