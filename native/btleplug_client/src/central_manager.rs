#![allow(dead_code)]
#![allow(unused_variables)]
use crate::atoms;
use crate::peripheral::PeripheralRef;
use crate::peripheral::PeripheralState;
use log::{debug, info, warn};
use rustler::{Encoder, Env, Error as RustlerError, LocalPid, OwnedEnv, ResourceArc, Term};

use btleplug::api::{
    Central, CentralEvent, Manager as _, Peripheral, PeripheralProperties, ScanFilter,
};
use btleplug::platform::{Adapter, Manager};
use futures::StreamExt;

use std::collections::HashMap;

use crate::RUNTIME;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio::sync::RwLock;
use tokio::time::{sleep, timeout, Duration};

pub struct CentralRef(pub(crate) Arc<Mutex<CentralManagerState>>);

pub struct CentralManagerState {
    pub pid: LocalPid,
    pub adapter: Adapter,
    pub manager: Manager,
    pub event_sender: mpsc::Sender<CentralEvent>,
    pub event_receiver: Arc<RwLock<mpsc::Receiver<CentralEvent>>>,
    pub discovered_peripherals: Arc<Mutex<HashMap<String, ResourceArc<PeripheralRef>>>>,
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
            discovered_peripherals: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

async fn get_peripheral_properties(
    adapter: &Adapter,
    target_id: &str,
) -> Option<(Arc<btleplug::platform::Peripheral>, PeripheralProperties)> {
    if let Ok(peripherals) = adapter.peripherals().await {
        for peripheral in peripherals.iter() {
            if peripheral.id().to_string() == target_id {
                if let Ok(Some(properties)) = peripheral.properties().await {
                    return Some((Arc::new(peripheral.clone()), properties));
                }
            }
        }
    }
    None
}

#[rustler::nif]
pub fn create_central(env: Env, pid: LocalPid) -> Result<ResourceArc<CentralRef>, RustlerError> {
    info!("Creating CentralManager... {:?}", pid.as_c_arg());

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
    let adapter_clone = adapter.clone();
    let adapter_info = RUNTIME.block_on(adapter.adapter_info());
    info!("Adapter initialized: {:?}", adapter_info);

    let (event_sender, event_receiver) = mpsc::channel::<CentralEvent>(100);
    let event_receiver = Arc::new(RwLock::new(event_receiver));
    let event_receiver_clone = event_receiver.clone();
    let event_sender_clone = event_sender.clone();

    let state =
        CentralManagerState::new(pid, manager, adapter.clone(), event_sender, event_receiver);
    let resource = ResourceArc::new(CentralRef(Arc::new(Mutex::new(state))));
    // let pid = env.pid();

    // Spawn a task to handle adapter events
    RUNTIME.spawn(async move {
        debug!("Starting adapter event handler...");
        let mut events = match adapter.events().await {
            Ok(events) => events,
            Err(e) => {
                debug!("Failed to get adapter events: {:?}", e);
                return;
            }
        };

        while let Some(event) = events.next().await {
            debug!("Received adapter event: {:?}", event);
            if let Err(e) = event_sender_clone.send(event).await {
                debug!("Failed to forward event: {:?}", e);
                break;
            }
        }
        debug!("Adapter event handler closed");
    });

    RUNTIME.spawn(async move {
        debug!("Starting event receiver handler...");
        let mut receiver = event_receiver_clone.write().await;

        while let Some(event) = receiver.recv().await {
            let mut msg_env = OwnedEnv::new();
            match event {
                CentralEvent::DeviceDiscovered(id) => {
                    let uuid = id.to_string();
                    info!("🔍 Device discovered - UUID: {}", uuid);

                    if let Some((peripheral, properties)) =
                        get_peripheral_properties(&adapter_clone, &uuid).await
                    {
                        let is_connected = peripheral.is_connected().await.unwrap_or(false);
                        debug!(
                            "🔍 Peripheral: {:?}, Connected: {:?}",
                            properties.local_name, is_connected
                        );
                        debug_properties(&properties);

                        match msg_env.send_and_clear(&pid, |env| {
                            (
                                atoms::btleplug_peripheral_discovered(),
                                uuid,
                                properties_to_map(env, &properties),
                            )
                                .encode(env)
                        }) {
                            Ok(_) => debug!("✅ Successfully sent device discovery message"),
                            Err(e) => debug!(
                                "⚠️ Failed to send device discovery message (Error: {:?}). \
                This might happen if the Elixir process has terminated.",
                                e
                            ),
                        }
                    } else {
                        warn!(
                            "❌ Could not retrieve properties for discovered peripheral: {}",
                            uuid
                        );
                    }
                }
                CentralEvent::DeviceConnected(id) => {
                    let uuid = id.to_string();
                    info!("Device connected - UUID: {}", uuid);
                    match msg_env.send_and_clear(&pid, |env| {
                        (atoms::btleplug_peripheral_connected(), uuid).encode(env)
                    }) {
                        Ok(_) => debug!("Successfully sent device connected message"),
                        Err(e) => debug!(
                            "Failed to send device connected message (Error: {:?}). \
                    This might happen if the Elixir process has terminated.",
                            e
                        ),
                    }
                }
                CentralEvent::DeviceDisconnected(id) => {
                    let uuid = id.to_string();
                    info!("Device disconnected - UUID: {}", uuid);
                    match msg_env.send_and_clear(&pid, |env| {
                        (atoms::btleplug_peripheral_disconnected(), uuid).encode(env)
                    }) {
                        Ok(_) => debug!("Successfully sent device disconnected message"),
                        Err(e) => debug!(
                            "Failed to send device disconnected message (Error: {:?}). \
                    This might happen if the Elixir process has terminated.",
                            e
                        ),
                    }
                }
                CentralEvent::ManufacturerDataAdvertisement {
                    id,
                    manufacturer_data,
                } => {
                    let uuid = id.to_string();
                    debug!(
                        "Manufacturer data from UUID: {} - Data: {:?}",
                        uuid, manufacturer_data
                    );
                    match msg_env.send_and_clear(&pid, |env| {
                        (
                            atoms::btleplug_manufacturer_data_advertisement(),
                            (uuid, manufacturer_data),
                        )
                            .encode(env)
                    }) {
                        Ok(_) => debug!("Successfully sent manufacturer data message"),
                        Err(e) => debug!(
                            "Failed to send manufacturer data message (Error: {:?}). \
                    This might happen if the Elixir process has terminated.",
                            e
                        ),
                    }
                }
                CentralEvent::ServiceDataAdvertisement { id, service_data } => {
                    let uuid = id.to_string();
                    debug!(
                        "Service data from UUID: {} - Data: {:?}",
                        uuid, service_data
                    );

                    // Convert the HashMap with Uuid keys to String keys
                    let converted_data: HashMap<String, Vec<u8>> = service_data
                        .into_iter()
                        .map(|(k, v)| (k.to_string(), v))
                        .collect();

                    match msg_env.send_and_clear(&pid, |env| {
                        (
                            atoms::btleplug_service_data_advertisement(),
                            (uuid, converted_data),
                        )
                            .encode(env)
                    }) {
                        Ok(_) => debug!("Successfully sent service data message"),
                        Err(e) => debug!(
                            "Failed to send service data message (Error: {:?}). \
            This might happen if the Elixir process has terminated.",
                            e
                        ),
                    }
                }
                CentralEvent::ServicesAdvertisement { id, services } => {
                    let uuid = id.to_string();
                    let services: Vec<String> =
                        services.into_iter().map(|s| s.to_string()).collect();
                    debug!("Services from UUID: {} - Services: {:?}", uuid, services);
                    match msg_env.send_and_clear(&pid, |env| {
                        (atoms::btleplug_services_advertisement(), (uuid, services)).encode(env)
                    }) {
                        Ok(_) => debug!("Successfully sent services message"),
                        Err(e) => debug!(
                            "Failed to send services message (Error: {:?}). \
                    This might happen if the Elixir process has terminated.",
                            e
                        ),
                    }
                }
                CentralEvent::StateUpdate(state) => {
                    debug!("Adapter state changed: {:?}", state);
                    match msg_env.send_and_clear(&pid, |env| {
                        (
                            atoms::btleplug_adapter_status_update(),
                            format!("{:?}", state),
                        )
                            .encode(env)
                    }) {
                        Ok(_) => debug!("Successfully sent state update message"),
                        Err(e) => debug!(
                            "Failed to send state update message (Error: {:?}). \
                    This might happen if the Elixir process has terminated.",
                            e
                        ),
                    }
                }
                CentralEvent::DeviceUpdated(id) => {
                    let uuid = id.to_string();
                    debug!("Device updated - UUID: {}", uuid);

                    if let Some((peripheral, properties)) =
                        get_peripheral_properties(&adapter_clone, &uuid).await
                    {
                        let is_connected = peripheral.is_connected().await.unwrap_or(false);
                        debug!(
                            "🔍 Peripheral Updated: {:?}: is_connected: {:?}",
                            properties.local_name, is_connected
                        );

                        debug_properties(&properties);

                        match msg_env.send_and_clear(&pid, |env| {
                            (
                                atoms::btleplug_peripheral_updated(),
                                uuid,
                                properties_to_map(env, &properties),
                            )
                                .encode(env)
                        }) {
                            Ok(_) => debug!("✅ Successfully sent device discovery message"),
                            Err(e) => debug!(
                                "⚠️ Failed to send device discovery message (Error: {:?}). \
                This might happen if the Elixir process has terminated.",
                                e
                            ),
                        }
                    } else {
                        warn!(
                            "❌ Could not retrieve properties for discovered peripheral: {}",
                            uuid
                        );
                    }
                } // _ => {
                  //     debug!("Other event: {:?}", event);
                  // }
            }
        }
        debug!("Event receiver closed.");
    });

    Ok(resource)
}

pub fn debug_properties<'a>(properties: &PeripheralProperties) {
    let local_name = properties.local_name.as_deref().unwrap_or("(unknown)");
    let address = properties.address;
    let address_type = properties
        .address_type
        .map_or("Unknown".to_string(), |at| format!("{:?}", at));
    let tx_power_level = properties
        .tx_power_level
        .map_or("N/A".to_string(), |tx| tx.to_string());
    let rssi = properties.rssi.map_or("N/A".to_string(), |r| r.to_string());
    let manufacturer_data = properties.manufacturer_data.clone();
    let service_data = properties.service_data.clone();
    let services = properties
        .services
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<String>>();

    println!("🔍 **Discovered Peripheral:**");
    println!("   📛 Name: {:?}", local_name);
    println!("   🔢 Address: {:?}", address);
    println!("   🏷  Address Type: {:?}", address_type);
    println!("   📡 TX Power Level: {:?}", tx_power_level);
    println!("   📶 RSSI: {:?}", rssi);
    println!("   Services: {:?}", services);

    if !manufacturer_data.is_empty() {
        println!("   🏭 Manufacturer Data:");
        for (id, data) in manufacturer_data.iter() {
            println!("     - ID {}: {:?}", id, data);
        }
    }

    if !service_data.is_empty() {
        println!("   🔗 Service Data:");
        for (uuid, data) in service_data.iter() {
            println!("     - UUID {}: {:?}", uuid, data);
        }
    }
}

pub fn properties_to_map<'a>(env: Env<'a>, props: &PeripheralProperties) -> Term<'a> {
    let mut map = HashMap::new();

    map.insert("address", props.address.to_string().encode(env));
    map.insert(
        "address_type",
        props
            .address_type
            .map(|at| format!("{:?}", at))
            .unwrap_or_else(|| "Unknown".to_string())
            .encode(env),
    );
    map.insert(
        "local_name",
        props
            .local_name
            .as_deref()
            .unwrap_or("(unknown)")
            .encode(env),
    );
    map.insert(
        "tx_power_level",
        props
            .tx_power_level
            .map_or("N/A".into(), |tx| tx.to_string())
            .encode(env),
    );
    map.insert(
        "rssi",
        props
            .rssi
            .map_or("N/A".into(), |r| r.to_string())
            .encode(env),
    );

    // Convert manufacturer data
    let manufacturer_data: HashMap<String, Vec<u8>> = props
        .manufacturer_data
        .iter()
        .map(|(id, data)| (id.to_string(), data.clone()))
        .collect();
    map.insert("manufacturer_data", manufacturer_data.encode(env));

    // Convert service data
    let service_data: HashMap<String, Vec<u8>> = props
        .service_data
        .iter()
        .map(|(uuid, data)| (uuid.to_string(), data.clone()))
        .collect();
    map.insert("service_data", service_data.encode(env));

    // Convert services to a list of UUID strings
    let services: Vec<String> = props.services.iter().map(|s| s.to_string()).collect();
    map.insert("services", services.encode(env));

    map.encode(env)
}

#[rustler::nif]
pub fn start_scan(
    env: Env,
    resource: ResourceArc<CentralRef>,
    duration_ms: u64,
) -> Result<ResourceArc<CentralRef>, RustlerError> {
    let resource_arc = resource.0.clone();
    let resource_arc_stop = resource_arc.clone();

    let env_pid = env.pid();

    RUNTIME.spawn(async move {
        let mut msg_env = OwnedEnv::new();
        // let env_pid_str = pid.as_c_arg();

        let adapter = {
            let central_state = resource_arc.lock().unwrap();
            central_state.adapter.clone()
        };

        let pid = {
            let central_state = resource_arc.lock().unwrap();
            central_state.pid
        };

        info!(
            "Starting BLE scan for {} ms..., caller pid: {:?}, state pid: {:?}",
            duration_ms,
            env_pid.as_c_arg(),
            pid.as_c_arg()
        );

        if let Err(e) = adapter.start_scan(ScanFilter::default()).await {
            warn!("Failed to start scan: {:?}", e);
            return;
        }
        msg_env.send_and_clear(&pid, |env| {
            (
                atoms::btleplug_scan_started(),
                format!("Scan started: {:?} ms", duration_ms),
            )
                .encode(env)
        });

        debug!("Scan started successfully");

        // Wait for the specified duration
        sleep(Duration::from_millis(duration_ms)).await;

        // Stop the scan after timeout
        let adapter = {
            let central_state = resource_arc_stop.lock().unwrap();
            central_state.adapter.clone()
        };

        if let Err(e) = adapter.stop_scan().await {
            warn!("Failed to stop scan after timeout: {:?}", e);
            return;
        }

        msg_env.send_and_clear(&pid, |env| {
            (
                atoms::btleplug_scan_stopped(),
                format!("Scan stopped after timeout: {:?} ms", duration_ms),
            )
                .encode(env)
        });

        debug!("Scan stopped automatically after {} ms", duration_ms);
    });

    Ok(resource)
}

#[rustler::nif]
pub fn stop_scan(
    env: Env,
    resource: ResourceArc<CentralRef>,
) -> Result<ResourceArc<CentralRef>, RustlerError> {
    debug!("Stopping BLE scan...");

    let resource_arc = resource.0.clone();

    RUNTIME.spawn(async move {
        let adapter = {
            let central_state = resource_arc.lock().unwrap();
            central_state.adapter.clone()
        };

        if let Err(e) = adapter.stop_scan().await {
            warn!("Failed to stop scan: {:?}", e);
            return;
        }
        debug!("Scan stopped successfully");
    });
    Ok(resource)
}

#[rustler::nif(schedule = "DirtyIo")]
pub fn find_peripheral_by_name(
    env: Env,
    resource: ResourceArc<CentralRef>,
    name: String,
    timeout_ms: u64,
) -> Result<ResourceArc<PeripheralRef>, RustlerError> {
    let env_pid = env.pid();
    let (tx, rx) = tokio::sync::oneshot::channel::<Result<ResourceArc<PeripheralRef>, String>>();

    let resource_arc = resource.0.clone();
    let (adapter, pid, discovered_peripherals, event_receiver) = {
        let central_state = resource_arc.lock().unwrap();
        (
            central_state.adapter.clone(),
            central_state.pid,
            central_state.discovered_peripherals.clone(),
            central_state.event_receiver.clone(),
        )
    };

    let name_clone = name.clone();
    let discovered_peripherals_clone = discovered_peripherals.clone();

    RUNTIME.spawn(async move {
        info!(
            "🔍 Looking for peripheral with name: {}, caller pid: {:?}, state pid: {:?}",
            name_clone,
            env_pid.as_c_arg(),
            pid.as_c_arg()
        );

        let cached_peripherals: Vec<(String, ResourceArc<PeripheralRef>)> = {
            let cache = discovered_peripherals_clone.lock().unwrap();
            cache
                .iter()
                .map(|(id, p)| (id.clone(), p.clone()))
                .collect()
        };

        // for (id, cached_peripheral_ref) in cached_peripherals {
        //     info!("🔍 Checking cached PeripheralRef: {}", id);

        //     let _ = tx.send(Ok(cached_peripheral_ref.clone()));
        //     return;
        // }

        // **Scan for new peripherals**
        let peripherals =
            match timeout(Duration::from_millis(timeout_ms), adapter.peripherals()).await {
                Ok(Ok(peripherals)) => peripherals,
                Ok(Err(e)) => {
                    warn!("❌ Failed to get peripherals: {:?}", e);
                    let _ = tx.send(Err(format!("Failed to get peripherals: {}", e)));
                    return;
                }
                Err(_) => {
                    warn!("⏳ Timeout while fetching peripherals");
                    let _ = tx.send(Err("Timeout while fetching peripherals".to_string()));
                    return;
                }
            };

        for peripheral in peripherals {
            let properties =
                match timeout(Duration::from_millis(timeout_ms), peripheral.properties()).await {
                    Ok(Ok(Some(props))) => props,
                    _ => continue,
                };

            if let Some(peripheral_name) = properties.local_name {
                if peripheral_name.contains(&name_clone) {
                    let peripheral_state = PeripheralState::new(
                        pid,
                        Arc::new(peripheral.clone()),
                        event_receiver.clone(),
                    );
                    let peripheral_ref =
                        ResourceArc::new(PeripheralRef(Arc::new(Mutex::new(peripheral_state))));

                    info!(
                        "✅ Storing PeripheralRef in cache: {:?} (Peripheral Ptr: {:p})",
                        peripheral.id(),
                        Arc::as_ptr(&peripheral_ref.0)
                    );

                    discovered_peripherals_clone
                        .lock()
                        .unwrap()
                        .insert(peripheral.id().to_string(), peripheral_ref.clone());

                    let _ = tx.send(Ok(peripheral_ref.clone()));
                    return;
                }
            }
        }

        let _ = tx.send(Err("Peripheral not found".to_string()));
    });

    match rx.blocking_recv() {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(err_msg)) => Err(RustlerError::Term(Box::new(format!("{:?}", err_msg)))),
        Err(_) => Err(RustlerError::Term(Box::new(
            "Failed to retrieve result".to_string(),
        ))),
    }
}

#[rustler::nif(schedule = "DirtyIo")]
pub fn find_peripheral(
    env: Env,
    resource: ResourceArc<CentralRef>,
    uuid: String,
    timeout_ms: u64,
) -> Result<ResourceArc<PeripheralRef>, RustlerError> {
    let env_pid = env.pid();
    let (tx, rx) = tokio::sync::oneshot::channel::<Result<ResourceArc<PeripheralRef>, String>>();

    let resource_arc = resource.0.clone();
    let (adapter, pid, discovered_peripherals, event_receiver) = {
        let central_state = resource_arc.lock().unwrap();
        (
            central_state.adapter.clone(),
            central_state.pid,
            central_state.discovered_peripherals.clone(),
            central_state.event_receiver.clone(),
        )
    };

    let uuid_clone = uuid.clone();
    let discovered_peripherals_clone = discovered_peripherals.clone();

    RUNTIME.spawn(async move {
        info!(
            "🔍 Looking for peripheral with UUID: {}, caller pid: {:?}, state pid: {:?}",
            uuid_clone,
            env_pid.as_c_arg(),
            pid.as_c_arg()
        );

        // **Step 1: Check Cache First**
        {
            let cache = discovered_peripherals_clone.lock().unwrap();
            if let Some(cached_peripheral) = cache.get(&uuid_clone) {
                info!("✅ Found cached PeripheralRef by UUID: {}", uuid_clone);
                let _ = tx.send(Ok(cached_peripheral.clone()));
                return;
            }
        }

        // **Step 2: Scan for new peripherals**
        let peripherals =
            match timeout(Duration::from_millis(timeout_ms), adapter.peripherals()).await {
                Ok(Ok(peripherals)) => peripherals,
                Ok(Err(e)) => {
                    warn!("❌ Failed to get peripherals: {:?}", e);
                    let _ = tx.send(Err(format!("Failed to get peripherals: {}", e)));
                    return;
                }
                Err(_) => {
                    warn!("⏳ Timeout while fetching peripherals");
                    let _ = tx.send(Err("Timeout while fetching peripherals".to_string()));
                    return;
                }
            };

        for peripheral in peripherals {
            if peripheral.id().to_string() == uuid_clone {
                let peripheral_state =
                    PeripheralState::new(pid, Arc::new(peripheral.clone()), event_receiver.clone());
                let peripheral_ref =
                    ResourceArc::new(PeripheralRef(Arc::new(Mutex::new(peripheral_state))));

                info!(
                    "✅ Storing PeripheralRef in cache: {:?} (Peripheral Ptr: {:p})",
                    peripheral.id(),
                    Arc::as_ptr(&peripheral_ref.0)
                );

                discovered_peripherals_clone
                    .lock()
                    .unwrap()
                    .insert(peripheral.id().to_string(), peripheral_ref.clone());

                let _ = tx.send(Ok(peripheral_ref.clone()));
                return;
            }
        }

        let _ = tx.send(Err(format!(
            "Peripheral not found with UUID: {}",
            uuid_clone
        )));
    });

    match rx.blocking_recv() {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(err_msg)) => Err(RustlerError::Term(Box::new(format!("{:?}", err_msg)))),
        Err(_) => Err(RustlerError::Term(Box::new(
            "Failed to retrieve result".to_string(),
        ))),
    }
}
