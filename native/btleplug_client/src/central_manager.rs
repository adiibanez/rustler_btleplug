use crate::atoms;
use crate::peripheral::PeripheralRef;
use crate::peripheral::PeripheralState;
use rustler::{Atom, Encoder, Env, Error as RustlerError, LocalPid, ResourceArc, Term};

use btleplug::api::{
    bleuuid::BleUuid, Central, CentralEvent, CharPropFlags, Characteristic, Manager as _,
    Peripheral, ScanFilter, Service, ValueNotification,
};
use btleplug::platform::{Adapter, Manager};
use futures::StreamExt;
use tokio::spawn;
use uuid::Uuid;

// Remove or comment out this function as it's now handled in lib.rs
// pub fn load(env: Env) -> bool {
//     rustler::resource!(CentralRef, env);
//     true
// }

use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;
use tokio::sync::mpsc;
use crate::RUNTIME;

pub struct CentralRef(pub(crate) Arc<Mutex<CentralManagerState>>); // ✅ Required by Rustler

pub struct CentralManagerState {
    pub pid: LocalPid,
    pub adapter: Adapter,
    pub manager: Manager,
    pub event_sender: mpsc::Sender<CentralEvent>,
    pub event_receiver: Arc<RwLock<mpsc::Receiver<CentralEvent>>>, // ✅ Async safe RwLock
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
    
    let manager = RUNTIME.block_on(Manager::new()).map_err(|e| {
        RustlerError::Term(Box::new(format!("Manager error: {}", e)))
    })?;

    let adapters = RUNTIME.block_on(manager.adapters()).map_err(|e| {
        RustlerError::Term(Box::new(format!("Adapter error: {}", e)))
    })?;

    if adapters.is_empty() {
        return Err(RustlerError::Term(Box::new("No available adapter")));
    }

    let adapter = adapters.into_iter().next().unwrap();
    let adapter_info = RUNTIME.block_on(adapter.adapter_info());
    println!("[Rust] Adapter initialized: {:?}", adapter_info);

    let (event_sender, event_receiver) = mpsc::channel::<CentralEvent>(100);
    let event_receiver = Arc::new(RwLock::new(event_receiver));
    let event_receiver_clone = event_receiver.clone();

    let state = CentralManagerState::new(env.pid(), manager, adapter, event_sender, event_receiver);
    let resource = ResourceArc::new(CentralRef(Arc::new(Mutex::new(state))));

    RUNTIME.spawn(async move {
        println!("[Rust] Inside tokio::spawn ...");
        
        let mut receiver = event_receiver_clone.write().await;
        
        while let Some(event) = receiver.recv().await {
            match event {
                CentralEvent::DeviceDiscovered(id) => {
                    println!("[Rust] Device Discovered: {:?}", id);
                }
                CentralEvent::DeviceUpdated(id) => {
                    println!("[Rust] Device Updated: {:?}", id);
                }
                CentralEvent::DeviceConnected(id) => {
                    println!("[Rust] Device Connected: {:?}", id);
                }
                _ => {}
            }
        }
        println!("[Rust] Event receiver closed.");
    });

    Ok(resource)
}

// Update start_scan to use RUNTIME as well
#[rustler::nif]
pub fn start_scan(resource: ResourceArc<CentralRef>) -> Result<ResourceArc<CentralRef>, RustlerError> {
    println!("[Rust] Starting BLE scan...");

    let resource_arc = resource.0.clone();

    RUNTIME.spawn(async move {
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
                            let name = properties.and_then(|p| p.as_ref()?.local_name.clone());

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
                            }
                        }
                        Err(e) => println!("PeripheralDiscovery Error {:?}", e),
                    }
                }
                _ => {}
            }
        }
    });

    Ok(resource)
}