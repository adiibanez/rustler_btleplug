use crate::atoms;
use crate::peripheral::PeripheralRef;
use crate::peripheral::PeripheralState;
// use crate::elixir_bridge::ElixirBridge;

// use rustler::types::pid::Pid;
use rustler::{Atom, Encoder, Env, Error as RustlerError, LocalPid, ResourceArc, Term};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use btleplug::api::{Central, CentralEvent, Manager as _, Peripheral, ScanFilter};
use btleplug::platform::{Adapter, Manager};
use futures::StreamExt;
use tokio::runtime::Runtime;
use tokio::spawn;
use tokio::sync::mpsc::Sender;

pub struct CentralRef(pub(crate) Arc<Mutex<CentralManagerState>>);

pub struct CentralManagerState {
    pub pid: LocalPid,
    pub adapter: Adapter,
    pub manager: Manager,
}

impl CentralManagerState {
    pub fn new(pid: LocalPid, manager: Manager, adapter: Adapter) -> Self {
        CentralManagerState {
            // config,
            pid,
            manager,
            adapter,
        }
    }
}

pub fn load(env: Env) -> bool {
    rustler::resource!(CentralRef, env);
    true
}

#[rustler::nif]
pub fn create_central(env: Env) -> Result<ResourceArc<CentralRef>, RustlerError> {
    println!("[Rust] Creating CentralManager...");

    // Create a Tokio runtime to run async code synchronously
    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| RustlerError::Term(Box::new(format!("Runtime error: {}", e))))?;

    // Run async code synchronously
    let manager = runtime
        .block_on(Manager::new())
        .map_err(|e| RustlerError::Term(Box::new(format!("Manager error: {}", e))))?;

    let adapters = runtime
        .block_on(manager.adapters())
        .map_err(|e| RustlerError::Term(Box::new(format!("Adapter error: {}", e))))?;

    // Get the first available adapter
    let adapter = adapters
        .into_iter()
        .next()
        .ok_or_else(|| RustlerError::Term(Box::new("No available adapter")))?;

    println!("[Rust] CentralManager created successfully.");

    // Initialize CentralManagerState
    let state = CentralManagerState::new(env.pid(), manager, adapter);
    let resource = ResourceArc::new(CentralRef(Arc::new(Mutex::new(state))));

    // Return the initialized resource to Elixir
    Ok(resource)
}

#[rustler::nif]
pub fn find_peripheral(
    env: Env,
    resource: ResourceArc<CentralRef>,
    uuid: String,
) -> Result<ResourceArc<PeripheralRef>, RustlerError> {
    println!("[Rust] Finding Peripheral: {}", uuid);

    // Lock the central manager state
    let mut state = resource.0.lock().map_err(|_| {
        RustlerError::Term(Box::new("Failed to lock CentralManagerState".to_string()))
    })?;

    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| RustlerError::Term(Box::new(format!("Runtime error: {}", e))))?;

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
    let mut central_state = resource_arc.lock().unwrap();



    // Create a Tokio runtime to run async code synchronously
    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| RustlerError::Term(Box::new(format!("Runtime error: {}", e))))?;

    // Run async code synchronously
    let adapter = runtime
        .block_on(central_state.adapter)
        .ok_or_else(|| RustlerError::Term(Box::new("No available adapter")))?;

    tokio::spawn(async move {
        // Start scanning
        if let Err(e) = adapter.start_scan(ScanFilter::default()).await {
            println!("[Rust] Failed to start scan: {:?}", e);
            return;
        }

        println!("[Rust] Scan started. Listening for events...");
        let mut events = adapter.events().await.expect("Failed to get event stream");

        while let Some(event) = events.next().await {
            println!("[Rust] BLE Event: {:?}", event);
            match event {
                CentralEvent::DeviceDiscovered(peripheral_id) => {
                    println!("[Rust] Device Discovered: {:?}", peripheral_id);
                    // Handle device discovered event
                }
                CentralEvent::DeviceUpdated(peripheral_id) => {
                    // Handle device updated event
                    println!("[Rust] Device Updated: {:?}", peripheral_id);
                }
                CentralEvent::DeviceConnected(peripheral_id) => {
                    // Handle device connected event
                    println!("[Rust] Device Connected: {:?}", peripheral_id);
                }
                _ => {}
            }
        }
    });

    Ok(resource)
}