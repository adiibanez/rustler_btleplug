use crate::atoms;
use log::{debug, error, info, warn};
use pretty_env_logger;

use crate::RUNTIME;
use btleplug::api::{Characteristic, Peripheral as ApiPeripheral};
use btleplug::platform::Peripheral;
use futures::StreamExt;
use rustler::{Atom, Encoder, Env, Error as RustlerError, LocalPid, OwnedEnv, ResourceArc, Term};
use std::sync::{Arc, Mutex}; // ✅ Import trait

pub struct PeripheralRef(pub(crate) Arc<Mutex<PeripheralState>>);

pub struct PeripheralState {
    pub pid: LocalPid,
    pub peripheral: Arc<Peripheral>, // ✅ Store Peripheral as Arc<>
}

impl PeripheralState {
    pub fn new(pid: LocalPid, peripheral: Peripheral) -> Self {
        PeripheralState {
            pid,
            peripheral: Arc::new(peripheral), // ✅ Use Arc to allow cloning
        }
    }
}

impl Drop for PeripheralState {
    fn drop(&mut self) {
        debug!("PeripheralResource destructor called.");
    }
}

#[rustler::nif]
pub fn connect(
    env: Env,
    resource: ResourceArc<PeripheralRef>,
) -> Result<ResourceArc<PeripheralRef>, RustlerError> {
    let peripheral = {
        let peripheral_state = resource.0.lock().unwrap();
        peripheral_state.peripheral.clone() // ✅ Clone outside async block
    };

    let pid = env.pid();

    RUNTIME.spawn(async move {
        let mut msg_env = OwnedEnv::new();

        debug!("Connecting to Peripheral: {:?}", peripheral.id());

        if let Err(e) = peripheral.connect().await {
            warn!("Failed to connect: {:?}", e);
        } else {
            info!("Successfully connected to peripheral.");

            msg_env
                .send_and_clear(&pid, |env| {
                    (
                        atoms::btleplug_device_connected(),
                        peripheral.id().to_string(),
                    )
                        .encode(env)
                })
                .ok();
            // // ✅ Call instance method correctly
            // if let Err(e) = peripheral.discover_services().await {
            //     warn!("Failed to discover services: {:?}", e);

            //     msg_env
            //         .send_and_clear(&pid, |env| {
            //             (
            //                 atoms::btleplug_device_service_discovery_error(),
            //                 peripheral.id().to_string(),
            //             )
            //                 .encode(env)
            //         })
            //         .ok();
            // } else {
            //     debug!("Services discovered successfully.");
            // }
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
    let peripheral = {
        let peripheral_state = resource.0.lock().unwrap();
        peripheral_state.peripheral.clone() // ✅ Clone outside async block
    };

    let pid = env.pid();

    RUNTIME.spawn(async move {
        let mut msg_env = OwnedEnv::new();

        match peripheral.discover_services().await {
            Ok(services) => {
                info!("Services length: {:?}", services);
            }
            Err(e) => {
                warn!("Error discovering services {:?}", e);
            }
        };

        // ✅ Call instance method on Peripheral
        // let characteristics = peripheral.characteristics().await
        //     .map_err(|e| format!("Discover characteristics error: {:?}", e))
        //     .unwrap_or_else(|e| {
        //         warn!("Failed to discover characteristics: {:?}", e);
        //         Vec::new()
        //     });

        let characteristics = peripheral.characteristics();

        info!("Characteristics length: {:?}", characteristics.len());

        for characteristic in &characteristics {
            info!("Characteristic: {:?}", characteristic);
        }

        let characteristic = characteristics
            .iter()
            .find(|c| c.uuid.to_string() == characteristic_uuid)
            .cloned();

        match characteristic {
            Some(char) => {
                info!("Subscribing to characteristic: {:?}", char.uuid);

                if let Err(e) = peripheral.subscribe(&char).await {
                    warn!("Failed to subscribe: {:?}", e);
                    return;
                }

                match peripheral.notifications().await {
                    Ok(mut notifications) => {
                        while let Some(notification) = notifications.next().await {
                            debug!("Received Notification: {:?}", notification.value);

                            msg_env
                                .send_and_clear(&pid, |env| {
                                    (
                                        atoms::btleplug_characteristic_value_changed(),
                                        char.uuid.to_string(),
                                    )
                                        .encode(env)
                                })
                                .ok();
                        }
                    }
                    Err(e) => warn!("Failed to get notifications: {:?}", e),
                }
            }
            None => info!("Characteristic not found: {}", characteristic_uuid),
        }
    });

    Ok(resource)
}
