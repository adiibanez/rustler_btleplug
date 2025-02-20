use crate::atoms;
use crate::peripheral::PeripheralRef;
use crate::peripheral::PeripheralState;
use rustler::{Atom, Encoder, Env, Error as RustlerError, LocalPid, ResourceArc, Term};
use std::sync::{Arc, Mutex};

use btleplug::api::{
    bleuuid::BleUuid, Central, CentralEvent, CharPropFlags, Characteristic, Manager as _,
    Peripheral, ScanFilter, Service, ValueNotification,
};
//use btleplug::api::{Central, CentralEvent, Manager as _, Peripheral, ScanFilter, Service};
use btleplug::platform::{Adapter, Manager};
use futures::StreamExt;
use tokio::runtime::Runtime;
use tokio::spawn;
use tokio::sync::mpsc;

use uuid::Uuid;

pub fn load(env: Env) -> bool {
    rustler::resource!(CentralRef, env);
    rustler::resource!(CentralManagerState, env);
    true
}


pub struct CentralRef(pub(crate) Arc<Mutex<CentralManagerState>>);

pub struct CentralManagerState {
    pub pid: LocalPid,
    pub adapter: Adapter,
    pub manager: Manager,
    pub event_sender: mpsc::Sender<CentralEvent>, // Channel to send events
}

impl CentralManagerState {
    pub fn new(
        pid: LocalPid,
        manager: Manager,
        adapter: Adapter,
        event_sender: mpsc::Sender<CentralEvent>,
    ) -> Self {
        CentralManagerState {
            pid,
            manager,
            adapter,
            event_sender,
        }
    }
}

#[rustler::nif]
pub fn create_central(env: Env) -> Result<ResourceArc<CentralRef>, RustlerError> {
    println!("[Rust] Creating CentralManager...");

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            println!("[Rust] Runtime creation failed: {:?}", e);
            return Err(RustlerError::Term(Box::new(format!("Runtime error: {}", e))));
        }
    };

    let manager = match runtime.block_on(Manager::new()) {
        Ok(m) => m,
        Err(e) => {
            println!("[Rust] Manager creation failed: {:?}", e);
            return Err(RustlerError::Term(Box::new(format!("Manager error: {}", e))));
        }
    };

    let adapters = match runtime.block_on(manager.adapters()) {
        Ok(a) => a,
        Err(e) => {
            println!("[Rust] Failed to get adapters: {:?}", e);
            return Err(RustlerError::Term(Box::new(format!("Adapter error: {}", e))));
        }
    };

    let adapter = match adapters.into_iter().next() {
        Some(a) => a,
        None => {
            println!("[Rust] No available BLE adapter found.");
            return Err(RustlerError::Term(Box::new("No available adapter")));
        }
    };

    // Create the channel here.
    let (event_sender, mut event_receiver) = mpsc::channel::<CentralEvent>(100);

    let state = CentralManagerState::new(env.pid(), manager, adapter, event_sender);
    let state_arc = Arc::new(Mutex::new(state)); // Wrap in Arc *before* moving

    println!("[Rust] Before creating ResourceArc ...");

    let resource = ResourceArc::new(CentralRef(state_arc.clone())); // Clone the Arc

    println!("[Rust] After creating ResourceArc ...");
    let resource_clone = resource.clone(); // ✅ Clone before spawning

    // Process the central events by reading from the new channel
    tokio::spawn(async move {
        println!("[Rust] Inside tokio::spawn ...");


        while let Some(event) = event_receiver.recv().await {
            // *IMPORTANT*: Minimize the time you hold the lock.
            // Extract the data you need *before* the await.
            let peripheral_id = match event {
                CentralEvent::DeviceDiscovered(id) => {
                    println!("[Rust] Device Discovered: {:?}", id);
                    Some(id)
                }
                CentralEvent::DeviceUpdated(id) => {
                    println!("[Rust] Device Updated: {:?}", id);
                    Some(id)
                }
                CentralEvent::DeviceConnected(id) => {
                    println!("[Rust] Device Connected: {:?}", id);
                    Some(id)
                }
                _ => None,
            };

            // Drop the lock *immediately* after extracting the data.
            // Now you can use `peripheral_id` safely across the `.await` point.
        }
        println!("[Rust] Event receiver closed.");
    });

    Ok(resource)
}

#[rustler::nif]
pub fn find_peripheral(
    env: Env,
    resource: ResourceArc<CentralRef>,
    uuid: String,
) -> Result<ResourceArc<PeripheralRef>, RustlerError> {

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            println!("[Rust] Runtime creation failed: {:?}", e);
            return Err(RustlerError::Term(Box::new(format!("Runtime error: {}", e))));
        }
    };


    println!("[Rust] Finding Peripheral: {}", uuid);

    // Lock the central manager state
    let mut state = resource.0.lock().map_err(|_| {
        RustlerError::Term(Box::new("Failed to lock CentralManagerState".to_string()))
    })?;

    // Ensure `adapter` exists in `state`
    let peripherals = runtime
        .block_on(state.adapter.peripherals())
        .map_err(|e| RustlerError::Term(Box::new(format!("Manager error: {}", e))))?;

    println!("[Rust] Peripherals retrieved successfully.");

    for peripheral in peripherals {
        if peripheral.id().to_string() == uuid {
            // Create a new PeripheralState
            let peripheral_state = PeripheralState::new(env.pid(), peripheral);
            let peripheral_resource =
                ResourceArc::new(PeripheralRef(Arc::new(Mutex::new(peripheral_state))));

            return Ok(peripheral_resource);
        }
    }

    Err(RustlerError::Term(Box::new(
        "Peripheral not found".to_string(),
    )))
}

#[rustler::nif]
pub fn start_scan(
    env: Env,
    resource: ResourceArc<CentralRef>,
) -> Result<ResourceArc<CentralRef>, RustlerError> {
    println!("[Rust] Starting BLE scan...");

    let resource_arc = resource.0.clone();

    tokio::spawn(async move {
        // Lock only when needed, then release it
        let adapter = {
            let central_state = resource_arc.lock().unwrap();
            central_state.adapter.clone()
        };

        let mut events = match adapter.events().await {
            Ok(e) => e,
            Err(_) => {
                println!("[Rust] Failed to lock on event stream");
                return;
            }
        };

        if let Err(e) = adapter.start_scan(ScanFilter::default()).await {
            println!("[Rust] Failed to start scan: {:?}", e);
            return;
        }

        while let Some(event) = events.as_mut().next().await {
            match event {
                CentralEvent::DeviceDiscovered(id) => {
                    let peripheral_result = adapter.peripheral(&id).await;

                    match peripheral_result {
                        Ok(peripheral) => {
                            let peripheral_is_connected =
                                peripheral.is_connected().await.unwrap_or(false);
                            let properties = peripheral.properties().await.ok();
                            let name = properties.and_then(|p| p.unwrap().local_name);

                            let predefined_prefixes =
                                vec!["PressureSensor", "Arduino", "HumiditySensor"];
                            let should_connect = name.as_ref().map_or(false, |name_str| {
                                predefined_prefixes
                                    .iter()
                                    .any(|prefix| name_str.contains(prefix))
                            });

                            if should_connect && !peripheral_is_connected {
                                if let Err(err) = peripheral.connect().await {
                                    eprintln!("Error connecting to peripheral, skipping: {}", err);
                                    continue;
                                }
                                println!("Now connected to peripheral {:?}.", name);
                                if let Err(e) = peripheral.discover_services().await {
                                    println!("Error discovering services: {:?}", e);
                                }

                                for service in peripheral.services() {
                                    println!("Service: {:?}", service.uuid);
                                }
                            }
                        }
                        Err(e) => {
                            println!("PeripheralDiscovery Error {:?}", e);
                        }
                    }
                }
                CentralEvent::StateUpdate(state) => {
                    println!("AdapterStatusUpdate {:?}", state);
                }
                CentralEvent::DeviceConnected(id) => {
                    println!("DeviceConnected: {:?}", id);
                }
                CentralEvent::DeviceDisconnected(id) => {
                    println!("DeviceDisconnected: {:?}", id);
                }
                CentralEvent::ManufacturerDataAdvertisement {
                    id,
                    manufacturer_data,
                } => {
                    println!(
                        "ManufacturerDataAdvertisement: {:?}, {:?}",
                        id, manufacturer_data
                    );
                }
                CentralEvent::ServiceDataAdvertisement { id, service_data } => {
                    println!("ServiceDataAdvertisement: {:?}, {:?}", id, service_data);
                }
                CentralEvent::ServicesAdvertisement { id, services } => {
                    println!("ServicesAdvertisement: {:?}, {:?}", id, services);
                }
                _ => {}
            }
        }
    });

    Ok(resource)
}
