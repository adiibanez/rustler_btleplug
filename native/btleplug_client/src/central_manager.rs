use crate::atoms;
use crate::peripheral::PeripheralRef;
use crate::peripheral::PeripheralState;
use rustler::{Atom, Encoder, Env, Error as RustlerError, LocalPid, ResourceArc, Term};
use std::sync::{Arc, Mutex};

use btleplug::api::{Central, CentralEvent, Manager as _, Peripheral, ScanFilter};
use btleplug::platform::{Adapter, Manager};
use futures::StreamExt;
use tokio::runtime::Runtime;
use tokio::spawn;
use tokio::sync::mpsc;

pub struct CentralRef(pub(crate) Arc<Mutex<CentralManagerState>>);

pub struct CentralManagerState {
    pub pid: LocalPid,
    pub adapter: Adapter,
    pub manager: Manager,
    pub runtime: Runtime,
    pub event_sender: mpsc::Sender<CentralEvent>, // Channel to send events
}

impl CentralManagerState {
    pub fn new(
        pid: LocalPid,
        manager: Manager,
        adapter: Adapter,
        runtime: Runtime,
        event_sender: mpsc::Sender<CentralEvent>,
    ) -> Self {
        CentralManagerState {
            pid,
            manager,
            adapter,
            runtime,
            event_sender,
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

    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| RustlerError::Term(Box::new(format!("Runtime error: {}", e))))?;

    let manager = runtime.block_on(Manager::new())
        .map_err(|e| RustlerError::Term(Box::new(format!("Manager error: {}", e))))?;

    let adapters = runtime.block_on(manager.adapters())
        .map_err(|e| RustlerError::Term(Box::new(format!("Adapter error: {}", e))))?;

    let adapter = adapters.into_iter().next()
        .ok_or_else(|| RustlerError::Term(Box::new("No available adapter")))?;

    //Create the channel here.
    let (event_sender, mut event_receiver) = mpsc::channel::<CentralEvent>(100);  // Adjust buffer size as needed

    let state = CentralManagerState::new(env.pid(), manager, adapter, runtime, event_sender);
    let resource = ResourceArc::new(CentralRef(Arc::new(Mutex::new(state))));

    let resource_clone = resource.clone();
    //Process the central events by reading from the new channel
    tokio::spawn(async move {
       let central_arc = resource_clone.0.clone();
       
        while let Some(event) = event_receiver.recv().await {
            match event {
                CentralEvent::DeviceDiscovered(peripheral_id) => {
                    println!("[Rust] Device Discovered: {:?}", peripheral_id);
                }
                CentralEvent::DeviceUpdated(peripheral_id) => {
                    println!("[Rust] Device Updated: {:?}", peripheral_id);
                }
                CentralEvent::DeviceConnected(peripheral_id) => {
                    println!("[Rust] Device Connected: {:?}", peripheral_id);
                }
                _ => {}
            }
        }
    });

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

    // Ensure `adapter` exists in `state`
    let peripherals = state.runtime.block_on(state.adapter.peripherals())
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

    Err(RustlerError::Term(Box::new("Peripheral not found".to_string())))
}


#[rustler::nif]
pub fn start_scan(
    env: Env,
    resource: ResourceArc<CentralRef>,
) -> Result<Atom, RustlerError> {
    println!("[Rust] Starting BLE scan...");

    let resource_arc = resource.0.clone();

    tokio::spawn(async move {
        let central_arc = resource_arc.clone();

        // Acquire the lock *before* any awaits
        let central_state = central_arc.lock().unwrap();

        // Clone the adapter and event_sender *while holding the lock*
        let adapter = central_state.adapter.clone();
        let event_sender = central_state.event_sender.clone();

        // *Drop the lock as soon as possible*
        //drop(central_state); //This will deallocate from the async thread context

        // Start scanning
        let scan_result = adapter.start_scan(ScanFilter::default()).await; //Added this scan result

        if let Err(e) = scan_result  { //Added this scan result
            println!("[Rust] Failed to start scan: {:?}", e);
            return;
        }

        println!("[Rust] Scan started. Listening for events...");
        let mut events = adapter.events().await.expect("Failed to get event stream");

        while let Some(event) = events.next().await {
             if let Err(e) = event_sender.send(event).await {
                 println!("[Rust] Error sending event: {:?}", e);
             }
        }
    });

    Ok(atoms::ok())
}