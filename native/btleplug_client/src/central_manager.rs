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
    let event_sender_clone = event_sender.clone();

    let state = CentralManagerState::new(env.pid(), manager, adapter.clone(), event_sender, event_receiver);
    let resource = ResourceArc::new(CentralRef(Arc::new(Mutex::new(state))));

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

    // Spawn a task to handle our channel events
    RUNTIME.spawn(async move {
        println!("[Rust] Starting event receiver handler...");
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
pub fn start_scan(resource: ResourceArc<CentralRef>) -> Result<ResourceArc<CentralRef>, RustlerError> {
    println!("[Rust] Starting BLE scan...");

    let resource_arc = resource.0.clone();

    RUNTIME.spawn(async move {
        let adapter = {
            let central_state = resource_arc.lock().unwrap();
            central_state.adapter.clone()
        };

        if let Err(e) = adapter.start_scan(ScanFilter::default()).await {
            println!("[Rust] Failed to start scan: {:?}", e);
            return;
        }
        println!("[Rust] Scan started successfully");
    });

    Ok(resource)
}


#[rustler::nif]
pub fn stop_scan(resource: ResourceArc<CentralRef>) -> Result<ResourceArc<CentralRef>, RustlerError> {
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
    uuid: String
) -> Result<ResourceArc<PeripheralRef>, RustlerError> {
    println!("[Rust] Finding peripheral with UUID: {}", uuid);
    
    let resource_arc = resource.0.clone();
    
    // Get the adapter from the central state
    let adapter = {
        let central_state = resource_arc.lock().unwrap();
        central_state.adapter.clone()
    };

    // Use the runtime to get peripherals
    let peripherals = RUNTIME.block_on(async {
        adapter.peripherals().await.map_err(|e| {
            RustlerError::Term(Box::new(format!("Failed to get peripherals: {}", e)))
        })
    })?;

    // Find the peripheral with matching UUID
    for peripheral in peripherals {
        println!("[Rust] Iterating peripheral peripheral.id(): {:?}, uuid: {:?}", peripheral.id(), uuid);
        if peripheral.id().to_string() == uuid {
            println!("[Rust] Found peripheral: {:?}", peripheral.id());
            let peripheral_state = PeripheralState::new(env.pid(), peripheral);
            return Ok(ResourceArc::new(PeripheralRef(Arc::new(Mutex::new(peripheral_state)))));
        }
    }

    Err(RustlerError::Term(Box::new("Peripheral not found")))
}




