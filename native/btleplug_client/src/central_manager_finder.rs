use crate::peripheral::PeripheralRef;
use crate::peripheral::PeripheralState;

use crate::central_manager_state::*;

use log::{info, warn};
use rustler::{Env, Error as RustlerError, ResourceArc};
 
use btleplug::api::{
    Central, Peripheral,
};

use crate::RUNTIME;
use std::sync::{Arc, Mutex};
use tokio::time::{timeout, Duration};


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
