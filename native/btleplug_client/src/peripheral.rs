#![allow(unused_mut)]

use crate::atoms;
use crate::RUNTIME;
use log::{debug, error, info, warn};
use pretty_env_logger;

use btleplug::api::{CharPropFlags, Characteristic, Peripheral as ApiPeripheral, Service};
use btleplug::platform::Peripheral;
use futures::StreamExt;
use rustler::{Atom, Encoder, Env, Error as RustlerError, LocalPid, OwnedEnv, ResourceArc, Term};
use std::sync::{Arc, Mutex};
use tokio::time::{timeout, Duration};

pub struct PeripheralRef(pub(crate) Arc<Mutex<PeripheralState>>);

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PeripheralStateEnum {
    Disconnected,
    Connecting,
    Connected,
    DiscoveringServices,
    ServicesDiscovered,
}

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

    pub fn set_state(peripheral_arc: &Arc<Mutex<Self>>, new_state: PeripheralStateEnum) {
        let mut state_guard = peripheral_arc.lock().unwrap();
        debug!("🔄 State change: {:?} → {:?}", state_guard.state, new_state);
        state_guard.state = new_state;
    }
}

impl Drop for PeripheralState {
    fn drop(&mut self) {
        debug!("💀 PeripheralResource destructor called.");
    }
}

pub async fn discover_services_internal(
    peripheral_arc: &Arc<Mutex<PeripheralState>>,
    timeout_ms: u64,
) -> bool {
    PeripheralState::set_state(peripheral_arc, PeripheralStateEnum::DiscoveringServices);

    for attempt in 1..=5 {
        debug!("🔍 [Attempt {}] Discovering services...", attempt);

        let peripheral_clone = {
            let state_guard = peripheral_arc.lock().unwrap();
            state_guard.peripheral.clone()
        };

        let success = match timeout(Duration::from_millis(timeout_ms), peripheral_clone.discover_services()).await {
            Ok(Ok(_)) => true,
            _ => false,
        };

        if success {
            tokio::time::sleep(Duration::from_millis(250)).await;

            let services = {
                let state_guard = peripheral_arc.lock().unwrap();
                state_guard.peripheral.services()
            };

            if !services.is_empty() {
                PeripheralState::set_state(peripheral_arc, PeripheralStateEnum::ServicesDiscovered);
                debug!("✅ Services successfully discovered.");
                return true;
            } else {
                warn!("⚠️ Services not found even after successful discovery.");
            }
        } else {
            warn!("⚠️ Service discovery attempt {} failed.", attempt);
        }

        // back off a little in case of failures
        let sleep_duration = 200 * attempt;
        debug!("Sleeping a little after service discovery attempt nr: {} for {}ms.", attempt, sleep_duration);
        tokio::time::sleep(Duration::from_millis(sleep_duration)).await;
    }

    warn!("❌ All service discovery attempts failed.");
    false
}

#[rustler::nif]
pub fn connect(
    env: Env,
    resource: ResourceArc<PeripheralRef>,
    timeout_ms: u64,
) -> Result<ResourceArc<PeripheralRef>, RustlerError> {
    let peripheral_arc = resource.0.clone();

    let env_pid = env.pid().clone();

    RUNTIME.spawn(async move {
        let (peripheral, pid) = {
            let state_guard = peripheral_arc.lock().unwrap();
            (state_guard.peripheral.clone(), state_guard.pid)
        };

        PeripheralState::set_state(&peripheral_arc, PeripheralStateEnum::Connecting);

        info!("🔗 Connecting to Peripheral: {:?}, caller pid: {:?}, state pid: {:?}", peripheral.id(), env_pid.as_c_arg(), pid.as_c_arg());

        let mut connected = false;
        for attempt in 1..=3 {
            match timeout(Duration::from_millis(timeout_ms), peripheral.connect()).await {
                Ok(Ok(_)) => {
                    info!("✅ Connected to peripheral: {:?}", peripheral.id());
                    connected = true;
                    break;
                }
                Ok(Err(e)) => warn!("❌ Connection attempt {} failed: {:?}", attempt, e),
                Err(_) => warn!("⏳ Connection attempt {} timed out!", attempt),
            }

            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        if !connected {
            warn!("❌ All connection attempts failed.");
            PeripheralState::set_state(&peripheral_arc, PeripheralStateEnum::Disconnected);
            return;
        }

        PeripheralState::set_state(&peripheral_arc, PeripheralStateEnum::Connected);
        discover_services_internal(&peripheral_arc, timeout_ms).await;
    });

    Ok(resource)
}

#[rustler::nif]
pub fn subscribe(
    env: Env,
    resource: ResourceArc<PeripheralRef>,
    characteristic_uuid: String,
    timeout_ms: u64, 
) -> Result<ResourceArc<PeripheralRef>, RustlerError> {
    let peripheral_arc = resource.0.clone();

    let env_pid = env.pid().clone();

    RUNTIME.spawn(async move {

        let (peripheral, state, pid) = {
            let state_guard = peripheral_arc.lock().unwrap();
            (state_guard.peripheral.clone(), state_guard.state, state_guard.pid)
        };

        info!("🔗 Subscribing to Peripheral: {:?}, caller pid: {:?}, state pid: {:?}", peripheral.id(), env_pid.as_c_arg(), pid.as_c_arg());

        if state != PeripheralStateEnum::ServicesDiscovered {
            warn!("⚠️ Services not yet discovered. Retrying...");
            let discovered = discover_services_internal(&peripheral_arc, timeout_ms).await;
            if !discovered {
                warn!("❌ Cannot proceed with subscription. No services discovered.");
                return;
            }
        }

        let characteristics = peripheral.characteristics();
        let characteristic = characteristics
            .iter()
            .find(|c| c.uuid.to_string() == characteristic_uuid)
            .cloned();

        match characteristic {
            Some(char) => {
                debug!("🔔 Subscribing to characteristic: {:?}", char.uuid);

                if !char.properties.contains(CharPropFlags::NOTIFY) {
                    debug!("⚠️ Characteristic {:?} does NOT support notifications!", char.uuid);
                    return;
                }

                match timeout(Duration::from_millis(timeout_ms), peripheral.subscribe(&char)).await {
                    Ok(Ok(_)) => info!("✅ Subscribed to characteristic: {:?}", char.uuid),
                    _ => {
                        warn!("❌ Failed to subscribe to {:?}", char.uuid);
                        return;
                    }
                }

                tokio::spawn(async move {
                    let mut msg_env = OwnedEnv::new();

                    match timeout(Duration::from_millis(timeout_ms), peripheral.notifications()).await {
                        Ok(Ok(mut notifications)) => {
                            debug!("📡 Listening for characteristic updates...");

                            while let Some(notification) = notifications.next().await {
                                // ✅ **Log Values at INFO Level**
                                debug!(
                                    "📩 Value Update: {:?} (UUID: {:?})",
                                    notification.value, notification.uuid
                                );

                                msg_env.send_and_clear(&pid, |env| {
                                    (
                                        atoms::btleplug_characteristic_value_changed(),
                                        notification.uuid.to_string(),
                                        notification.value.clone(),
                                    )
                                        .encode(env)
                                }).ok();
                            }

                            warn!("⚠️ Notifications stream ended for UUID: {:?}", characteristic_uuid);
                        }
                        Ok(Err(e)) => warn!("❌ Failed to start notifications: {:?}", e),
                        Err(_) => warn!("⏳ Timeout while waiting for notifications."),
                    }
                });
            }
            None => info!("⚠️ Characteristic not found: {}", characteristic_uuid),
        }
    });

    Ok(resource)
}