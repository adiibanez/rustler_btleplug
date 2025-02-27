#![allow(dead_code)]
#![allow(unused_variables)]
use crate::atoms;
use crate::peripheral::PeripheralRef;
use crate::peripheral::PeripheralState;

use crate::central_manager_state::CentralRef;
use crate::central_manager_state::CentralManagerState;
use crate::central_manager_utils::*;

use log::{debug, info, warn};
use rustler::{Encoder, Env, Error as RustlerError, LocalPid, OwnedEnv, ResourceArc, Term};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::iter::FromIterator; 
use btleplug::api::{
    CharPropFlags, Central, CentralEvent, Manager as _, Peripheral, PeripheralProperties, ScanFilter,
};
use btleplug::platform::{Adapter, Manager};
use futures::StreamExt;

use crate::RUNTIME;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio::sync::RwLock;
use tokio::time::{sleep, timeout, Duration};

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

