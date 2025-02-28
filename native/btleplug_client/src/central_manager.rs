#![allow(dead_code)]
#![allow(unused_variables)]
use crate::atoms;

use crate::central_manager_state::cache_rssi;
use crate::central_manager_state::CentralManagerState;
use crate::central_manager_state::CentralRef;
use crate::central_manager_state::DISCOVERED_SERVICES;
use crate::central_manager_utils::*;

use log::{debug, info, warn};
use rustler::{Encoder, Env, Error as RustlerError, LocalPid, OwnedEnv, ResourceArc};

use btleplug::api::{Central, CentralEvent, Manager as _, Peripheral, ScanFilter};
use btleplug::platform::{Adapter, Manager};
use futures::StreamExt;

use crate::RUNTIME;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};

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
    info!("✅ Adapter initialized: {:?}", adapter_info);

    let (event_sender, event_receiver) = mpsc::channel::<CentralEvent>(100);
    let event_receiver = Arc::new(RwLock::new(event_receiver));
    let event_receiver_clone = event_receiver.clone();
    let event_sender_clone = event_sender.clone();

    let state =
        CentralManagerState::new(pid, manager, adapter.clone(), event_sender, event_receiver);
    let resource = ResourceArc::new(CentralRef(Arc::new(Mutex::new(state))));

    // 🛠️ **Spawn event handler**
    RUNTIME.spawn(async move {
        debug!("🎧 Listening for adapter events...");
        let mut events = match adapter.events().await {
            Ok(events) => events,
            Err(e) => {
                debug!("❌ Failed to get adapter events: {:?}", e);
                return;
            }
        };

        while let Some(event) = events.next().await {
            debug!("🔔 Adapter Event: {:?}", event);
            if let Err(e) = event_sender_clone.send(event).await {
                debug!("⚠️ Failed to forward event: {:?}", e);
                break;
            }
        }
        debug!("📴 Adapter event handler closed");
    });

    // 🏷️ **Handle BLE Events**
    RUNTIME.spawn(async move {
        debug!("🎧 Listening for BLE events...");
        let mut receiver = event_receiver_clone.write().await;

        while let Some(event) = receiver.recv().await {
            let mut msg_env = OwnedEnv::new();

            match event {
                CentralEvent::DeviceDiscovered(id) => {
                    let uuid = id.to_string();
                    info!("🔍 Device discovered: {}", uuid);

                    if let Some(peripheral) = find_peripheral_by_uuid(&adapter_clone, &uuid).await {
                        // 🔄 Ensure service discovery
                        if let Err(e) = peripheral.discover_services().await {
                            warn!("⚠️ Failed to discover services for {}: {:?}", uuid, e);
                        }

                        let properties_opt = peripheral.properties().await.ok().flatten();

                        if let Some(rssi) = properties_opt.as_ref().and_then(|p| p.rssi) {
                            cache_rssi(&uuid, rssi).await;
                        }

                        let is_connected = peripheral.is_connected().await.unwrap_or(false);

                        debug!(
                            "🔍 Peripheral: {:?}, Connected: {:?}",
                            properties_opt.as_ref().and_then(|p| p.local_name.clone()),
                            is_connected
                        );

                        match msg_env.send_and_clear(&pid, |env| {
                            (
                                atoms::btleplug_peripheral_discovered(),
                                uuid,
                                properties_opt
                                    .as_ref()
                                    .map(|props| properties_to_map(env, props))
                                    .unwrap_or_else(|| rustler::types::atom::nil().encode(env)),
                            )
                                .encode(env)
                        }) {
                            Ok(_) => debug!("✅ Sent device discovery message"),
                            Err(e) => debug!("⚠️ Failed to send discovery message: {:?}", e),
                        }
                    } else {
                        warn!("❌ Could not find peripheral: {}", uuid);
                    }
                }

                CentralEvent::DeviceUpdated(id) => {
                    let uuid = id.to_string();
                    info!("🔄 Device updated: {}", uuid);

                    if let Some(peripheral) = find_peripheral_by_uuid(&adapter_clone, &uuid).await {
                        let properties_opt = peripheral.properties().await.ok().flatten();
                        let is_connected = peripheral.is_connected().await.unwrap_or(false);

                        if let Some(rssi) = properties_opt.as_ref().and_then(|p| p.rssi) {
                            cache_rssi(&uuid, rssi).await;
                        }

                        debug!(
                            "🔄 Updated Peripheral: {:?} (Connected: {:?})",
                            properties_opt.as_ref().and_then(|p| p.local_name.clone()),
                            is_connected
                        );

                        match msg_env.send_and_clear(&pid, |env| {
                            (
                                atoms::btleplug_peripheral_updated(),
                                uuid,
                                properties_opt
                                    .as_ref()
                                    .map(|props| properties_to_map(env, props))
                                    .unwrap_or_else(|| rustler::types::atom::nil().encode(env)),
                            )
                                .encode(env)
                        }) {
                            Ok(_) => debug!("✅ Sent device updated message"),
                            Err(e) => debug!("⚠️ Failed to send update message: {:?}", e),
                        }
                    } else {
                        warn!("❌ Could not find peripheral: {}", uuid);
                    }
                }

                CentralEvent::DeviceConnected(id) => {
                    let uuid = id.to_string();
                    info!("🔗 Device connected: {}", uuid);
                    match msg_env.send_and_clear(&pid, |env| {
                        (atoms::btleplug_peripheral_connected(), uuid).encode(env)
                    }) {
                        Ok(_) => debug!("✅ Sent device connected message"),
                        Err(e) => debug!("⚠️ Failed to send connected message: {:?}", e),
                    }
                }

                CentralEvent::DeviceDisconnected(id) => {
                    let uuid = id.to_string();
                    info!("❌ Device disconnected: {}", uuid);
                    match msg_env.send_and_clear(&pid, |env| {
                        (atoms::btleplug_peripheral_disconnected(), uuid).encode(env)
                    }) {
                        Ok(_) => debug!("✅ Sent device disconnected message"),
                        Err(e) => debug!("⚠️ Failed to send disconnected message: {:?}", e),
                    }
                }

                CentralEvent::StateUpdate(state) => {
                    debug!("🔄 Adapter state changed: {:?}", state);
                    match msg_env.send_and_clear(&pid, |env| {
                        (
                            atoms::btleplug_adapter_status_update(),
                            format!("{:?}", state),
                        )
                            .encode(env)
                    }) {
                        Ok(_) => debug!("✅ Sent state update message"),
                        Err(e) => debug!("⚠️ Failed to send state update message: {:?}", e),
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

                    // ✅ Clone services BEFORE consuming it
                    let services_clone = services.clone();

                    let services: Vec<String> =
                        services.into_iter().map(|s| s.to_string()).collect();
                    debug!("Services from UUID: {} - Services: {:?}", uuid, services);

                    let service_uuids: Vec<String> =
                        services_clone.into_iter().map(|s| s.to_string()).collect();

                    let mut cache = DISCOVERED_SERVICES.write().await;
                    cache.insert(uuid.clone(), service_uuids.clone());

                    match msg_env.send_and_clear(&pid, |env| {
                        (
                            atoms::btleplug_services_advertisement(),
                            (uuid, service_uuids),
                        )
                            .encode(env)
                    }) {
                        Ok(_) => debug!("Successfully sent services message"),
                        Err(e) => debug!(
                            "Failed to send services message (Error: {:?}). \
            This might happen if the Elixir process has terminated.",
                            e
                        ),
                    }
                } //_ => debug!("🆕 Other event: {:?}", event),
            }
        }
        debug!("📴 Event receiver closed.");
    });

    Ok(resource)
}

/// 🔎 **Find a Peripheral by UUID Using Adapter's Live Data**
async fn find_peripheral_by_uuid(
    adapter: &Adapter,
    target_uuid: &str,
) -> Option<btleplug::platform::Peripheral> {
    let peripherals = adapter.peripherals().await.ok()?;
    peripherals
        .into_iter()
        .find(|p| p.id().to_string() == target_uuid)
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
