#![allow(unused_mut)]
use crate::atoms;
use crate::RUNTIME;
use log::{debug, error, info, warn};
use pretty_env_logger;

use btleplug::api::{Characteristic, CharPropFlags, Peripheral as ApiPeripheral, Service};
use btleplug::platform::Peripheral;
use futures::StreamExt;
use rustler::{Atom, Encoder, Env, Error as RustlerError, LocalPid, OwnedEnv, ResourceArc, Term};
use std::sync::{Arc, Mutex};
use tokio::time::{timeout, Duration};

/// 🚀 Enum for Peripheral State Management
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PeripheralStateEnum {
    Disconnected,
    Connecting,
    Connected,
    DiscoveringServices,
    ServicesDiscovered,
}

/// 🚀 PeripheralRef: Holds the BLE peripheral state
pub struct PeripheralRef(pub(crate) Arc<Mutex<PeripheralState>>);

/// 🔧 Peripheral State Management
pub struct PeripheralState {
    pub pid: LocalPid,
    pub peripheral: Peripheral,
    pub state: PeripheralStateEnum,
}

impl PeripheralState {
    pub fn new(pid: LocalPid, peripheral: Peripheral) -> Self {
        PeripheralState {
            pid,
            peripheral,
            state: PeripheralStateEnum::Disconnected,
        }
    }

    pub fn set_state(&mut self, new_state: PeripheralStateEnum) {
        self.state = new_state;
        debug!("📝 Updated state: {:?}", new_state);
    }
}

impl Drop for PeripheralState {
    fn drop(&mut self) {
        debug!("💀 PeripheralResource destructor called.");
    }
}

/// **🔗 Connect to a Peripheral with Robust Handling**
#[rustler::nif]
pub fn connect(
    env: Env,
    resource: ResourceArc<PeripheralRef>,
    timeout_ms: u64,
) -> Result<ResourceArc<PeripheralRef>, RustlerError> {
    let peripheral_arc = resource.0.clone();

    RUNTIME.spawn(async move {
        let mut msg_env = OwnedEnv::new();
        let (peripheral, pid, current_state) = {
            let state_guard = peripheral_arc.lock().unwrap();
            (
                state_guard.peripheral.clone(),
                state_guard.pid,
                state_guard.state,
            )
        };

        if current_state == PeripheralStateEnum::Connected {
            info!("⚠️ Already connected. Skipping connection.");
            return;
        }

        debug!("🔗 Connecting to Peripheral: {:?}", peripheral.id());

        // ✅ Update state before attempting connection
        {
            let mut state_guard = peripheral_arc.lock().unwrap();
            state_guard.set_state(PeripheralStateEnum::Connecting);
        }

        match timeout(Duration::from_millis(timeout_ms), peripheral.connect()).await {
            Ok(Ok(_)) => info!("✅ Successfully connected to peripheral: {:?}", peripheral.id()),
            Ok(Err(e)) => {
                warn!("❌ Failed to connect: {:?}", e);
                return;
            }
            Err(_) => {
                warn!("⏳ Timeout while connecting to peripheral!");
                return;
            }
        }

        // ✅ Update state to connected
        {
            let mut state_guard = peripheral_arc.lock().unwrap();
            state_guard.set_state(PeripheralStateEnum::Connected);
        }

        msg_env.send_and_clear(&pid, |env| {
            (atoms::btleplug_device_connected(), peripheral.id().to_string()).encode(env)
        }).ok();

        // ✅ Try discovering services
        discover_services_internal(&peripheral_arc, timeout_ms).await;
    });

    Ok(resource)
}

/// **🔎 Internal Function to Discover Services with Retries**
async fn discover_services_internal(peripheral_arc: &Arc<Mutex<PeripheralState>>, timeout_ms: u64) {
    let mut attempt = 0;
    let mut discovered_services = false;

    while attempt < 3 {
        debug!("🔍 [Attempt {}] Discovering services...", attempt + 1);

        let peripheral = {
            let state_guard = peripheral_arc.lock().unwrap();
            state_guard.peripheral.clone()
        };

        match timeout(Duration::from_millis(timeout_ms), peripheral.discover_services()).await {
            Ok(Ok(_)) => {
                tokio::time::sleep(Duration::from_millis(500)).await;

                let services = {
                    let state_guard = peripheral_arc.lock().unwrap();
                    state_guard.peripheral.services()
                };

                if !services.is_empty() {
                    discovered_services = true;
                    break;
                }
            }
            _ => warn!("⚠️ Service discovery failed or timed out."),
        }

        tokio::time::sleep(Duration::from_millis(500)).await;
        attempt += 1;
    }

    if discovered_services {
        let mut state_guard = peripheral_arc.lock().unwrap();
        state_guard.set_state(PeripheralStateEnum::ServicesDiscovered);
    } else {
        warn!("⚠️ Services not found even after retries.");
    }
}

#[rustler::nif]
pub fn subscribe(
    env: Env,
    resource: ResourceArc<PeripheralRef>,
    characteristic_uuid: String,
    timeout_ms: u64,
) -> Result<ResourceArc<PeripheralRef>, RustlerError> {
    let peripheral_arc = resource.0.clone();

    RUNTIME.spawn(async move {
        let mut msg_env = OwnedEnv::new();

        let (peripheral, services_discovered, pid) = {
            let state_guard = peripheral_arc.lock().unwrap();
            (
                state_guard.peripheral.clone(),
                state_guard.services_discovered,
                state_guard.pid,
            )
        };

        if !services_discovered {
            warn!("⚠️ Services not discovered yet! Calling discover_services()...");
            match timeout(Duration::from_millis(timeout_ms), peripheral.discover_services()).await {
                Ok(Ok(_)) => {
                    let services = peripheral.services();
                    debug!("🔍 [After] Found {} services.", services.len());
                    if !services.is_empty() {
                        let mut state_guard = peripheral_arc.lock().unwrap();
                        state_guard.services_discovered = true;
                    } else {
                        warn!("⚠️ Services not found even after discovery.");
                    }
                }
                Ok(Err(e)) => {
                    warn!("❌ `discover_services()` failed: {:?}", e);
                    return;
                }
                Err(_) => {
                    warn!("⏳ Timeout while discovering services!");
                    return;
                }
            }
        }

        let characteristics = peripheral.characteristics();
        info!("🔎 Found {} characteristics.", characteristics.len());

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

                match timeout(Duration::from_millis(timeout_ms), peripheral.subscribe(&char)).await {
                    Ok(Ok(_)) => info!("✅ Subscribed to characteristic: {:?}", char.uuid),
                    Ok(Err(e)) => {
                        warn!("❌ Failed to subscribe: {:?}", e);
                        return;
                    }
                    Err(_) => {
                        warn!("⏳ Timeout while subscribing to characteristic!");
                        return;
                    }
                }

                // ✅ **Ensure Notifications Are Received**
                let peripheral_clone = peripheral.clone();
                let pid_clone = pid.clone();
                tokio::spawn(async move {
                    let mut msg_env = OwnedEnv::new();

                    match timeout(Duration::from_millis(timeout_ms), peripheral_clone.notifications()).await {
                        Ok(Ok(mut notifications)) => {
                            info!("📡 Listening for characteristic updates...");
                            let mut received_any = false;

                            while let Some(notification) = notifications.next().await {
                                received_any = true;
                                debug!(
                                    "📩 Received Notification: {:?} (from {:?})",
                                    notification.value, notification.uuid
                                );

                                let send_result = msg_env.send_and_clear(&pid_clone, |env| {
                                    (
                                        atoms::btleplug_characteristic_value_changed(),
                                        notification.uuid.to_string(),
                                        notification.value.clone(),
                                    )
                                        .encode(env)
                                });

                                if let Err(e) = send_result {
                                    error!("🚨 Failed to send notification to Elixir: {:?}", e);
                                } else {
                                    debug!("✅ Notification sent to Elixir successfully.");
                                }
                            }

                            if !received_any {
                                warn!("⚠️ No notifications received for characteristic {:?}. Possible issue with device!", char.uuid);
                            }
                        }
                        Ok(Err(e)) => warn!("⚠️ Failed to get notifications: {:?}", e),
                        Err(_) => warn!("⏳ Timeout while waiting for notifications!"),
                    }
                });
            }
            None => info!("⚠️ Characteristic not found: {}", characteristic_uuid),
        }
    });

    Ok(resource)
}