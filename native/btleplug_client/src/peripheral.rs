#![allow(unused_mut)]

use crate::atoms;
use crate::RUNTIME;
use log::{debug, info, warn};

use btleplug::api::{CentralEvent, CharPropFlags, Peripheral as ApiPeripheral};
use btleplug::platform::Peripheral;
use futures::StreamExt;
use rustler::{Encoder, Env, Error as RustlerError, LocalPid, OwnedEnv, ResourceArc};
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, RwLock};
use tokio::time::{timeout, Duration};

pub struct PeripheralRef(pub(crate) Arc<Mutex<PeripheralState>>);

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PeripheralStateEnum {
    Disconnected,
    Disconnecting,
    Connecting,
    Connected,
    DiscoveringServices,
    ServicesDiscovered,
}

pub struct PeripheralState {
    pub pid: LocalPid,
    pub peripheral: Arc<Peripheral>,
    pub state: PeripheralStateEnum,
    pub event_receiver: Arc<RwLock<mpsc::Receiver<CentralEvent>>>,
}

impl PeripheralState {
    pub fn new(
        pid: LocalPid,
        peripheral: Arc<Peripheral>,
        event_receiver: Arc<RwLock<mpsc::Receiver<CentralEvent>>>,
    ) -> Self {
        info!(
            "🔗 PeripheralState: new Peripheral: {:?} (Peripheral Ptr: {:p})",
            peripheral.id(),
            &peripheral as *const _
        );

        PeripheralState {
            pid,
            peripheral,
            state: PeripheralStateEnum::Disconnected,
            event_receiver: event_receiver,
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

    let (peripheral, event_receiver) = {
        let state_guard = peripheral_arc.lock().unwrap();
        (
            state_guard.peripheral.clone(),
            state_guard.event_receiver.clone(),
        )
    };

    info!(
        "🔍 Checking if services are already discovered for {:?}",
        peripheral.id()
    );

    let existing_services = peripheral_arc.lock().unwrap().peripheral.services();

    if !existing_services.is_empty() {
        info!(
            "✅ Services already discovered for {:?}: {:?}",
            peripheral.id(),
            existing_services.iter().map(|s| s.uuid).collect::<Vec<_>>() // Logs discovered service UUIDs
        );
        PeripheralState::set_state(peripheral_arc, PeripheralStateEnum::ServicesDiscovered);
        return true;
    } else {
        debug!("❌ No services found yet for {:?}", peripheral.id());
    }

    info!(
        "🔍 Waiting for service discovery event for {:?}",
        peripheral.id()
    );

    let mut receiver = event_receiver.write().await;
    let mut service_discovered = false;

    while let Some(event) = timeout(Duration::from_millis(timeout_ms), receiver.recv())
        .await
        .ok()
        .flatten()
    {
        if let CentralEvent::ServicesAdvertisement { id, .. } = &event {
            if id.to_string() == peripheral.id().to_string() {
                service_discovered = true;
                break;
            }
        }
    }

    if service_discovered {
        PeripheralState::set_state(peripheral_arc, PeripheralStateEnum::ServicesDiscovered);
        info!(
            "✅ Services discovered for peripheral: {:?}",
            peripheral.id()
        );
        return true;
    } else {
        warn!("❌ Service discovery timed out for {:?}", peripheral.id());
        return false;
    }
}

#[rustler::nif]
pub fn connect(
    env: Env,
    resource: ResourceArc<PeripheralRef>,
    timeout_ms: u64,
) -> Result<ResourceArc<PeripheralRef>, RustlerError> {
    let peripheral_arc = resource.0.clone();
    let env_pid = env.pid();

    RUNTIME.spawn(async move {
        let (peripheral, pid) = {
            let state_guard = peripheral_arc.lock().unwrap();
            (state_guard.peripheral.clone(), state_guard.pid)
        };

        info!(
            "🔗 Connecting to Peripheral: {:?} (Peripheral Ptr: {:p})",
            peripheral.id(),
            &peripheral as *const _
        );

        PeripheralState::set_state(&peripheral_arc, PeripheralStateEnum::Connecting);

        info!(
            "🔗 Connecting to Peripheral: {:?}, caller pid: {:?}, state pid: {:?}",
            peripheral.id(),
            env_pid.as_c_arg(),
            pid.as_c_arg()
        );

        let mut connected = false;
        for attempt in 1..=3 {
            match timeout(Duration::from_millis(timeout_ms), peripheral.connect()).await {
                Ok(Ok(_)) => {
                    info!("✅ Connected to peripheral: {:?}", peripheral.id());
                    connected = true;
                    info!("🔍 Manually calling discover_services() after connecting, wait 500ms");
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    peripheral.discover_services().await;
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

        info!(
            "🔍 Manually triggering service discovery for peripheral: {:?}",
            peripheral.id()
        );
        if let Err(e) = timeout(
            Duration::from_millis(timeout_ms),
            peripheral.discover_services(),
        )
        .await
        {
            warn!("❌ Service discovery failed: {:?}", e);
        }

        if !discover_services_internal(&peripheral_arc, timeout_ms).await {
            warn!("⚠️ No services discovered after manual and event-based discovery.");
        }

        info!(
            "✅ Returning PeripheralState for {:?} (Peripheral Ptr: {:p})",
            peripheral.id(),
            &peripheral as *const _
        );
    });

    Ok(resource)
}

#[rustler::nif]
pub fn disconnect(
    env: Env,
    resource: ResourceArc<PeripheralRef>,
    timeout_ms: u64,
) -> Result<ResourceArc<PeripheralRef>, RustlerError> {
    let peripheral_arc = resource.0.clone();

    let env_pid = env.pid();

    RUNTIME.spawn(async move {
        let (peripheral, pid) = {
            let state_guard = peripheral_arc.lock().unwrap();
            (state_guard.peripheral.clone(), state_guard.pid)
        };

        PeripheralState::set_state(&peripheral_arc, PeripheralStateEnum::Disconnecting);

        info!(
            "🔗 Disconnecting from Peripheral: {:?}, caller pid: {:?}, state pid: {:?}",
            peripheral.id(),
            env_pid.as_c_arg(),
            pid.as_c_arg()
        );

        let mut disconnected = false;
        match timeout(Duration::from_millis(timeout_ms), peripheral.disconnect()).await {
            Ok(Ok(_)) => {
                info!("✅ Disconnected from peripheral: {:?}", peripheral.id());
                disconnected = true;
            }
            Ok(Err(e)) => warn!("❌ Failed to disconnect: {:?}", e),
            Err(_) => warn!("⏳ Disconnect attempt timed out!"),
        }

        if !disconnected {
            warn!("❌ Disconnect failed.");
            PeripheralState::set_state(&peripheral_arc, PeripheralStateEnum::Connected);
            return;
        }

        PeripheralState::set_state(&peripheral_arc, PeripheralStateEnum::Disconnected);
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
    let env_pid = env.pid();

    RUNTIME.spawn(async move {
        let peripheral_clone = peripheral_arc.lock().unwrap().peripheral.clone();
        let state_clone = peripheral_arc.lock().unwrap().state;
        let pid_clone = peripheral_arc.lock().unwrap().pid;

        info!(
            "🔗 Subscribing to Peripheral: {:?}, caller pid: {:?}, state pid: {:?}",
            peripheral_clone.id(),
            env_pid.as_c_arg(),
            pid_clone.as_c_arg()
        );

        info!(
            "✅ Returning PeripheralState for {:?} (Peripheral Ptr: {:p})",
            peripheral_clone.id(),
            &peripheral_clone as *const _
        );

        if state_clone != PeripheralStateEnum::ServicesDiscovered {
            warn!("⚠️ Services not yet discovered. Manually triggering discovery...");
            if let Err(e) = timeout(
                Duration::from_millis(timeout_ms),
                peripheral_clone.discover_services(),
            )
            .await
            {
                warn!("❌ Service discovery failed: {:?}", e);
                return;
            }

            RUNTIME.spawn({
                let peripheral_arc_clone = peripheral_arc.clone();
                async move {
                    if !discover_services_internal(&peripheral_arc_clone, timeout_ms).await {
                        warn!("⚠️ No services discovered, but proceeding with subscription.");
                    }
                }
            });
        }

        info!("🔍 Waiting 2s before checking characteristics...");
        tokio::time::sleep(Duration::from_millis(2000)).await;

        let characteristics = peripheral_clone.characteristics();
        let characteristic = characteristics
            .iter()
            .find(|c| c.uuid.to_string() == characteristic_uuid)
            .cloned();

        if characteristic.is_none() {
            warn!(
                "❌ Characteristic {} not found! Available UUIDs: {:?}",
                characteristic_uuid,
                characteristics
                    .iter()
                    .map(|c| c.uuid.to_string())
                    .collect::<Vec<_>>()
            );
        }

        match characteristic {
            Some(char) => {
                debug!("🔔 Subscribing to characteristic: {:?}", char.uuid);
                info!(
                    "🔔 Found characteristic: {:?}, Properties: {:?}",
                    char.uuid, char.properties
                );

                if !char.properties.contains(CharPropFlags::NOTIFY) {
                    debug!(
                        "⚠️ Characteristic {:?} does NOT support notifications!",
                        char.uuid
                    );
                    return;
                }

                match timeout(
                    Duration::from_millis(timeout_ms),
                    peripheral_clone.subscribe(&char),
                )
                .await
                {
                    Ok(Ok(_)) => info!("✅ Subscribed to characteristic: {:?}", char.uuid),
                    _ => {
                        warn!("❌ Failed to subscribe to {:?}", char.uuid);
                        return;
                    }
                }

                tokio::spawn(async move {
                    let mut msg_env = OwnedEnv::new();

                    match timeout(
                        Duration::from_millis(timeout_ms),
                        peripheral_clone.notifications(),
                    )
                    .await
                    {
                        Ok(Ok(mut notifications)) => {
                            debug!("📡 Listening for characteristic updates...");

                            debug!("📡 Started listening for characteristic updates...");
                            while let Some(notification) = notifications.next().await {
                                debug!(
                                    "📩 Received Value Update: {:?} (UUID: {:?})",
                                    notification.value, notification.uuid
                                );

                                msg_env
                                    .send_and_clear(&pid_clone, |env| {
                                        (
                                            atoms::btleplug_characteristic_value_changed(),
                                            notification.uuid.to_string(),
                                            notification.value.clone(),
                                        )
                                            .encode(env)
                                    })
                                    .ok();
                            }
                            warn!(
                                "⚠️ Notifications stream ended for UUID: {:?}",
                                characteristic_uuid
                            );
                        }
                        Ok(Err(e)) => warn!("❌ Subscription failed for {:?}: {:?}", char.uuid, e),
                        Err(_) => warn!("⏳ Subscription attempt timed out!"),
                    }
                });
            }
            None => warn!(
                "❌ Characteristic with UUID {} not found! Available UUIDs: {:?}",
                characteristic_uuid,
                characteristics
                    .iter()
                    .map(|c| c.uuid.to_string())
                    .collect::<Vec<_>>()
            ),
        }
    });

    Ok(resource)
}

#[rustler::nif]
pub fn unsubscribe(
    env: Env,
    resource: ResourceArc<PeripheralRef>,
    characteristic_uuid: String,
    timeout_ms: u64,
) -> Result<ResourceArc<PeripheralRef>, RustlerError> {
    let peripheral_arc = resource.0.clone();
    let env_pid = env.pid();

    RUNTIME.spawn(async move {
        let (peripheral, state, pid) = {
            let state_guard = peripheral_arc.lock().unwrap();
            (
                state_guard.peripheral.clone(),
                state_guard.state,
                state_guard.pid,
            )
        };

        info!(
            "🔗 Unsubscribing from Peripheral: {:?}, caller pid: {:?}, state pid: {:?}",
            peripheral.id(),
            env_pid.as_c_arg(),
            pid.as_c_arg()
        );

        if state != PeripheralStateEnum::ServicesDiscovered {
            warn!("⚠️ Services not yet discovered. Waiting for service event...");
            let discovered = discover_services_internal(&peripheral_arc, timeout_ms).await;
            if !discovered {
                warn!("❌ Cannot proceed with unsubscribe. No services discovered.");
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
                debug!("🔔 Unsubscribing from characteristic: {:?}", char.uuid);

                if !char.properties.contains(CharPropFlags::NOTIFY) {
                    debug!(
                        "⚠️ Characteristic {:?} does NOT support notifications!",
                        char.uuid
                    );
                    return;
                }

                match timeout(
                    Duration::from_millis(timeout_ms),
                    peripheral.unsubscribe(&char),
                )
                .await
                {
                    Ok(Ok(_)) => info!("✅ Unsubscribed from characteristic: {:?}", char.uuid),
                    _ => {
                        warn!("❌ Failed to unsubscribe from {:?}", char.uuid);
                    }
                }
            }
            None => info!("⚠️ Characteristic not found: {}", characteristic_uuid),
        }
    });

    Ok(resource)
}
