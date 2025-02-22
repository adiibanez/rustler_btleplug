use crate::atoms;
use crate::RUNTIME;
use log::{debug, error, info, warn};
use pretty_env_logger;

use btleplug::api::{Characteristic, CharPropFlags, Peripheral as ApiPeripheral, Service};
use btleplug::platform::Peripheral;
use futures::StreamExt;
use rustler::{Atom, Encoder, Env, Error as RustlerError, LocalPid, OwnedEnv, ResourceArc, Term};
use std::sync::{Arc, Mutex};

/// 🚀 Struct that holds the BLE peripheral state
pub struct PeripheralRef(pub(crate) Arc<Mutex<PeripheralState>>);

/// 🔧 Struct that manages BLE peripheral state and discovered services
pub struct PeripheralState {
    pub pid: LocalPid,
    pub peripheral: Peripheral,
    pub services_discovered: bool, // ✅ Track if services were discovered
}

impl PeripheralState {
    /// ✅ Constructor: Creates a new PeripheralState instance
    pub fn new(pid: LocalPid, peripheral: Peripheral) -> Self {
        PeripheralState {
            pid,
            peripheral,
            services_discovered: false,
        }
    }
}

impl Drop for PeripheralState {
    fn drop(&mut self) {
        debug!("💀 PeripheralResource destructor called.");
    }
}

#[rustler::nif]
pub fn connect(
    env: Env,
    resource: ResourceArc<PeripheralRef>,
) -> Result<ResourceArc<PeripheralRef>, RustlerError> {
    let pid = env.pid();
    let peripheral_arc = resource.0.clone();

    RUNTIME.spawn(async move {
        let mut msg_env = OwnedEnv::new();

        let peripheral = {
            let state_guard = peripheral_arc.lock().unwrap();
            state_guard.peripheral.clone()
        };

        debug!("🔗 Connecting to Peripheral: {:?}", peripheral.id());

        if let Err(e) = peripheral.connect().await {
            warn!("❌ Failed to connect: {:?}", e);
            return;
        }

        info!(
            "✅ Successfully connected to peripheral: {:?}",
            peripheral.id()
        );

        msg_env
            .send_and_clear(&pid, |env| {
                (
                    atoms::btleplug_device_connected(),
                    peripheral.id().to_string(),
                )
                    .encode(env)
            })
            .ok();

        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        
        let before_services = peripheral.services();
        debug!("🔍 [Before] Found {} services.", before_services.len());

        match peripheral.discover_services().await {
            Ok(_) => {
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;

                let after_services = peripheral.services();
                debug!("🔍 [After] Found {} services.", after_services.len());

                if !after_services.is_empty() {
                    let mut state_guard = peripheral_arc.lock().unwrap();
                    state_guard.services_discovered = true;
                    debug!("📝 Updated state: services_discovered = true");
                } else {
                    warn!("⚠️ Services not found even after discovery.");
                }
            }
            Err(e) => {
                warn!("⚠️ Failed to discover services: {:?}", e);
                msg_env
                    .send_and_clear(&pid, |env| {
                        (
                            atoms::btleplug_device_service_discovery_error(),
                            peripheral.id().to_string(),
                        )
                            .encode(env)
                    })
                    .ok();
            }
        }
    });

    Ok(resource)
}

#[rustler::nif]
pub fn subscribe(
    env: Env,
    resource: ResourceArc<PeripheralRef>,
    characteristic_uuid: String,
) -> Result<ResourceArc<PeripheralRef>, RustlerError> {
    let pid = env.pid();
    let peripheral_arc = resource.0.clone();

    RUNTIME.spawn(async move {
        let mut msg_env = OwnedEnv::new();

        let (peripheral, services_discovered) = {
            let state_guard = peripheral_arc.lock().unwrap();
            (
                state_guard.peripheral.clone(),
                state_guard.services_discovered,
            )
        };

        if !services_discovered {
            warn!("⚠️ Services not discovered yet! Calling discover_services()...");
            match peripheral.discover_services().await {
                Ok(_) => {
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    let services = peripheral.services();
                    debug!("🔍 [After] Found {} services.", services.len());

                    if !services.is_empty() {
                        let mut state_guard = peripheral_arc.lock().unwrap();
                        state_guard.services_discovered = true;
                    } else {
                        warn!("⚠️ Services not found even after discovery.");
                    }
                }
                Err(e) => warn!("❌ `discover_services()` failed: {:?}", e),
            }
        }

        let services = peripheral.services();
        info!("🔎 Found {} services.", services.len());

        let characteristics = peripheral.characteristics();
        info!("🔎 Found {} characteristics.", characteristics.len());

        for service in &services {
            debug!("📌 Service UUID: {:?}", service.uuid);
        }

        for characteristic in &characteristics {
            info!("🧬 Characteristic:");
            info!("   🆔 UUID: {:?}", characteristic.uuid);
            info!("   🔹 Properties: {:?}", characteristic.properties);

            if !characteristic.descriptors.is_empty() {
                info!("   📜 Descriptors: {:?}", characteristic.descriptors);
            } else {
                info!("   🚫 No Descriptors");
            }
        }

        let characteristic = characteristics
            .iter()
            .find(|c| c.uuid.to_string() == characteristic_uuid)
            .cloned();

        match characteristic {
            Some(char) => {
                info!("🔔 Subscribing to characteristic: {:?}", char.uuid);

                if !char.properties.contains(CharPropFlags::NOTIFY) {
                    warn!("⚠️ Characteristic {:?} does NOT support notifications!", char.uuid);
                    return;
                }

                if let Err(e) = peripheral.subscribe(&char).await {
                    warn!("❌ Failed to subscribe: {:?}", e);
                    return;
                }

                // ✅ **Persist the notifications task**
                let peripheral_clone = peripheral.clone();
                let pid_clone = pid.clone();

                tokio::spawn(async move {
                    let mut msg_env = OwnedEnv::new();

                    match peripheral_clone.notifications().await {
                        Ok(mut notifications) => {
                            info!("📡 Listening for characteristic updates...");

                            while let Some(notification) = notifications.next().await {
                                debug!(
                                    "📩 Received Notification: {:?} (from {:?})",
                                    notification.value, notification.uuid
                                );

                                // ✅ **Send data to Elixir**
                                msg_env.send_and_clear(&pid_clone, |env| {
                                    (
                                        atoms::btleplug_characteristic_value_changed(),
                                        notification.uuid.to_string(),
                                        notification.value.clone(),
                                    )
                                        .encode(env)
                                }).ok();
                            }
                        }
                        Err(e) => warn!("⚠️ Failed to get notifications: {:?}", e),
                    }
                });
            }
            None => info!("⚠️ Characteristic not found: {}", characteristic_uuid),
        }
    });

    Ok(resource)
}
