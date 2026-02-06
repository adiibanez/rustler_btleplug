//! BLE Peripheral (server/advertiser) mode NIF functions.
//!
//! This module provides NIF functions for creating a BLE peripheral that can
//! advertise services, handle read/write requests, and send notifications.

#![allow(dead_code)]
#![allow(unused_variables)]

use crate::atoms;
use crate::peripheral_manager_state::{
    PeripheralManagerRef, PeripheralManagerState, ReadRequestResponse, WriteRequestResponse,
};
use crate::peripheral_types::ServiceDefinition;
use crate::RUNTIME;

use ble_peripheral_rust::gatt::characteristic::Characteristic;
use ble_peripheral_rust::gatt::peripheral_event::{
    PeripheralEvent, ReadRequestResponse as BleReadResponse, RequestResponse,
    WriteRequestResponse as BleWriteResponse,
};
use ble_peripheral_rust::gatt::properties::{AttributePermission, CharacteristicProperty};
use ble_peripheral_rust::gatt::service::Service;
use ble_peripheral_rust::{Peripheral, PeripheralImpl};
use uuid::Uuid;

use log::{debug, info, warn};
use rustler::{Encoder, Env, Error as RustlerError, LocalPid, OwnedEnv, ResourceArc};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::channel;

/// Create a new BLE peripheral manager
#[rustler::nif]
pub fn create_peripheral(
    env: Env,
    pid: LocalPid,
) -> Result<ResourceArc<PeripheralManagerRef>, RustlerError> {
    info!("Creating PeripheralManager... {:?}", pid.as_c_arg());

    let state = PeripheralManagerState::new(pid);
    let peripheral_arc = state.peripheral.clone();
    let state_arc = Arc::new(Mutex::new(state));
    let resource = ResourceArc::new(PeripheralManagerRef(state_arc.clone()));

    // Create the peripheral in the background
    let state_clone = state_arc.clone();
    RUNTIME.spawn(async move {
        let mut msg_env = OwnedEnv::new();

        // Create event channel
        let (sender_tx, mut receiver_rx) = channel::<PeripheralEvent>(256);

        // Create the peripheral
        let peripheral = match Peripheral::new(sender_tx).await {
            Ok(p) => p,
            Err(e) => {
                warn!("Failed to create peripheral: {:?}", e);
                msg_env.send_and_clear(&pid, |env| {
                    (
                        atoms::btleplug_error(),
                        format!("Failed to create peripheral: {:?}", e),
                    )
                        .encode(env)
                });
                return;
            }
        };

        // Store the peripheral
        {
            let mut peripheral_lock = peripheral_arc.lock().await;
            *peripheral_lock = Some(peripheral);
        }

        // Wait for peripheral to be powered
        let mut powered = false;
        for _ in 0..50 {
            // Try for up to 5 seconds
            let is_powered = {
                let mut peripheral_lock = peripheral_arc.lock().await;
                if let Some(ref mut p) = *peripheral_lock {
                    p.is_powered().await.unwrap_or(false)
                } else {
                    false
                }
            };
            if is_powered {
                powered = true;
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        if !powered {
            warn!("Peripheral failed to power on");
            msg_env.send_and_clear(&pid, |env| {
                (
                    atoms::btleplug_error(),
                    "Peripheral failed to power on".to_string(),
                )
                    .encode(env)
            });
            return;
        }

        msg_env.send_and_clear(&pid, |env| {
            (atoms::btleplug_peripheral_created(), atoms::ok()).encode(env)
        });

        info!("✅ Peripheral created and powered on");

        // Handle events
        while let Some(event) = receiver_rx.recv().await {
            let current_pid = {
                let state_lock = state_clone.lock().unwrap();
                state_lock.pid
            };

            match event {
                PeripheralEvent::StateUpdate { is_powered } => {
                    debug!("State update: is_powered={}", is_powered);
                }

                PeripheralEvent::CharacteristicSubscriptionUpdate { request, subscribed } => {
                    debug!(
                        "Subscription update: char={}, service={}, subscribed={}",
                        request.characteristic, request.service, subscribed
                    );
                    msg_env.send_and_clear(&current_pid, |env| {
                        (
                            atoms::btleplug_peripheral_subscription_update(),
                            (
                                request.characteristic.to_string(),
                                request.service.to_string(),
                                subscribed,
                            ),
                        )
                            .encode(env)
                    });
                }

                PeripheralEvent::ReadRequest {
                    request,
                    offset,
                    responder,
                } => {
                    debug!(
                        "Read request: char={}, service={}, offset={}",
                        request.characteristic, request.service, offset
                    );

                    // Generate a unique request ID for tracking
                    let internal_request_id = {
                        let state = state_clone.lock().unwrap();
                        state.next_request_id()
                    };

                    // Create a oneshot channel for the response
                    let (response_tx, response_rx) =
                        tokio::sync::oneshot::channel::<ReadRequestResponse>();

                    // Store the pending request
                    {
                        let mut state = state_clone.lock().unwrap();
                        state.add_read_request(internal_request_id, response_tx);
                    }

                    // Send event to Elixir
                    msg_env.send_and_clear(&current_pid, |env| {
                        (
                            atoms::btleplug_peripheral_read_request(),
                            (
                                internal_request_id,
                                request.characteristic.to_string(),
                                request.service.to_string(),
                                offset,
                            ),
                        )
                            .encode(env)
                    });

                    // Wait for response from Elixir (with timeout)
                    let state_for_timeout = state_clone.clone();
                    tokio::spawn(async move {
                        match tokio::time::timeout(
                            tokio::time::Duration::from_secs(5),
                            response_rx,
                        )
                        .await
                        {
                            Ok(Ok(response)) => {
                                let ble_response = if response.success {
                                    RequestResponse::Success
                                } else {
                                    RequestResponse::RequestNotSupported
                                };
                                let _ = responder.send(BleReadResponse {
                                    value: response.value,
                                    response: ble_response,
                                });
                            }
                            Ok(Err(_)) => {
                                // Channel was dropped
                                let _ = responder.send(BleReadResponse {
                                    value: vec![],
                                    response: RequestResponse::RequestNotSupported,
                                });
                            }
                            Err(_) => {
                                // Timeout - clean up pending request
                                let mut state = state_for_timeout.lock().unwrap();
                                state.pending_read_requests.remove(&internal_request_id);
                                let _ = responder.send(BleReadResponse {
                                    value: vec![],
                                    response: RequestResponse::RequestNotSupported,
                                });
                            }
                        }
                    });
                }

                PeripheralEvent::WriteRequest {
                    request,
                    value,
                    offset,
                    responder,
                } => {
                    debug!(
                        "Write request: char={}, service={}, offset={}, value_len={}",
                        request.characteristic,
                        request.service,
                        offset,
                        value.len()
                    );

                    // Generate a unique request ID for tracking
                    let internal_request_id = {
                        let state = state_clone.lock().unwrap();
                        state.next_request_id()
                    };

                    // Create a oneshot channel for the response
                    let (response_tx, response_rx) =
                        tokio::sync::oneshot::channel::<WriteRequestResponse>();

                    // Store the pending request
                    {
                        let mut state = state_clone.lock().unwrap();
                        state.add_write_request(internal_request_id, response_tx);
                    }

                    // Send event to Elixir
                    msg_env.send_and_clear(&current_pid, |env| {
                        (
                            atoms::btleplug_peripheral_write_request(),
                            (
                                internal_request_id,
                                request.characteristic.to_string(),
                                request.service.to_string(),
                                value.clone(),
                                offset,
                            ),
                        )
                            .encode(env)
                    });

                    // Wait for response from Elixir (with timeout)
                    let state_for_timeout = state_clone.clone();
                    tokio::spawn(async move {
                        match tokio::time::timeout(
                            tokio::time::Duration::from_secs(5),
                            response_rx,
                        )
                        .await
                        {
                            Ok(Ok(response)) => {
                                let ble_response = if response.success {
                                    RequestResponse::Success
                                } else {
                                    RequestResponse::RequestNotSupported
                                };
                                let _ = responder.send(BleWriteResponse {
                                    response: ble_response,
                                });
                            }
                            Ok(Err(_)) => {
                                let _ = responder.send(BleWriteResponse {
                                    response: RequestResponse::RequestNotSupported,
                                });
                            }
                            Err(_) => {
                                let mut state = state_for_timeout.lock().unwrap();
                                state.pending_write_requests.remove(&internal_request_id);
                                let _ = responder.send(BleWriteResponse {
                                    response: RequestResponse::RequestNotSupported,
                                });
                            }
                        }
                    });
                }
            }
        }

        debug!("📴 Peripheral event handler closed");
    });

    Ok(resource)
}

/// Add a GATT service to the peripheral
#[rustler::nif]
pub fn add_service(
    env: Env,
    resource: ResourceArc<PeripheralManagerRef>,
    service_def: ServiceDefinition,
) -> Result<rustler::Atom, RustlerError> {
    let state_arc = resource.0.clone();
    let (pid, peripheral_arc) = {
        let state = state_arc.lock().unwrap();
        (state.pid, state.peripheral.clone())
    };

    // Parse service UUID
    let service_uuid = Uuid::from_str(&service_def.uuid)
        .map_err(|e| RustlerError::Term(Box::new(format!("Invalid service UUID: {}", e))))?;

    // Convert characteristics
    let mut characteristics = Vec::new();
    for char_def in &service_def.characteristics {
        let char_uuid = Uuid::from_str(&char_def.uuid)
            .map_err(|e| RustlerError::Term(Box::new(format!("Invalid characteristic UUID: {}", e))))?;

        let mut properties = Vec::new();
        let mut permissions = Vec::new();

        if char_def.properties.read {
            properties.push(CharacteristicProperty::Read);
            permissions.push(AttributePermission::Readable);
        }
        if char_def.properties.write {
            properties.push(CharacteristicProperty::Write);
            permissions.push(AttributePermission::Writeable);
        }
        if char_def.properties.write_without_response {
            properties.push(CharacteristicProperty::WriteWithoutResponse);
        }
        if char_def.properties.notify {
            properties.push(CharacteristicProperty::Notify);
        }
        if char_def.properties.indicate {
            properties.push(CharacteristicProperty::Indicate);
        }

        characteristics.push(Characteristic {
            uuid: char_uuid,
            properties,
            permissions,
            value: char_def.value.clone(),
            descriptors: Vec::new(),
        });
    }

    let service = Service {
        uuid: service_uuid,
        primary: service_def.primary,
        characteristics,
    };

    // Add service in the runtime
    let service_clone = service.clone();
    let service_uuid_str = service_uuid.to_string();
    RUNTIME.spawn(async move {
        let mut msg_env = OwnedEnv::new();

        let result = {
            let mut peripheral_lock = peripheral_arc.lock().await;
            if let Some(ref mut peripheral) = *peripheral_lock {
                Some(peripheral.add_service(&service_clone).await)
            } else {
                None
            }
        };

        match result {
            Some(Ok(_)) => {
                // Store service in state
                {
                    let mut state = state_arc.lock().unwrap();
                    state.services.push(service_clone);
                }

                msg_env.send_and_clear(&pid, |env| {
                    (
                        atoms::btleplug_peripheral_service_added(),
                        service_uuid_str.clone(),
                    )
                        .encode(env)
                });
                info!("✅ Service added: {}", service_uuid_str);
            }
            Some(Err(e)) => {
                warn!("Failed to add service: {:?}", e);
                msg_env.send_and_clear(&pid, |env| {
                    (
                        atoms::btleplug_error(),
                        format!("Failed to add service: {:?}", e),
                    )
                        .encode(env)
                });
            }
            None => {
                warn!("Peripheral not initialized");
                msg_env.send_and_clear(&pid, |env| {
                    (atoms::btleplug_error(), "Peripheral not initialized".to_string()).encode(env)
                });
            }
        }
    });

    Ok(atoms::ok())
}

/// Start advertising the peripheral
#[rustler::nif]
pub fn start_advertising(
    env: Env,
    resource: ResourceArc<PeripheralManagerRef>,
    device_name: String,
    service_uuids: Vec<String>,
) -> Result<rustler::Atom, RustlerError> {
    let state_arc = resource.0.clone();
    let (pid, peripheral_arc) = {
        let state = state_arc.lock().unwrap();
        (state.pid, state.peripheral.clone())
    };

    // Parse service UUIDs
    let mut uuids = Vec::new();
    for uuid_str in &service_uuids {
        let uuid = Uuid::from_str(uuid_str)
            .map_err(|e| RustlerError::Term(Box::new(format!("Invalid UUID: {}", e))))?;
        uuids.push(uuid);
    }

    let device_name_clone = device_name.clone();
    RUNTIME.spawn(async move {
        let mut msg_env = OwnedEnv::new();

        // Store device name
        {
            let mut state = state_arc.lock().unwrap();
            state.device_name = Some(device_name_clone.clone());
        }

        let result = {
            let mut peripheral_lock = peripheral_arc.lock().await;
            if let Some(ref mut peripheral) = *peripheral_lock {
                Some(peripheral.start_advertising(&device_name_clone, &uuids).await)
            } else {
                None
            }
        };

        match result {
            Some(Ok(_)) => {
                {
                    let mut state = state_arc.lock().unwrap();
                    state.is_advertising = true;
                }
                msg_env.send_and_clear(&pid, |env| {
                    (atoms::btleplug_peripheral_advertising_started(), atoms::ok()).encode(env)
                });
                info!("✅ Advertising started: {}", device_name_clone);
            }
            Some(Err(e)) => {
                warn!("Failed to start advertising: {:?}", e);
                msg_env.send_and_clear(&pid, |env| {
                    (
                        atoms::btleplug_error(),
                        format!("Failed to start advertising: {:?}", e),
                    )
                        .encode(env)
                });
            }
            None => {
                warn!("Peripheral not initialized");
                msg_env.send_and_clear(&pid, |env| {
                    (atoms::btleplug_error(), "Peripheral not initialized".to_string()).encode(env)
                });
            }
        }
    });

    Ok(atoms::ok())
}

/// Stop advertising
#[rustler::nif]
pub fn stop_advertising(
    env: Env,
    resource: ResourceArc<PeripheralManagerRef>,
) -> Result<rustler::Atom, RustlerError> {
    let state_arc = resource.0.clone();
    let (pid, peripheral_arc) = {
        let state = state_arc.lock().unwrap();
        (state.pid, state.peripheral.clone())
    };

    RUNTIME.spawn(async move {
        let mut msg_env = OwnedEnv::new();

        let result = {
            let mut peripheral_lock = peripheral_arc.lock().await;
            if let Some(ref mut peripheral) = *peripheral_lock {
                Some(peripheral.stop_advertising().await)
            } else {
                None
            }
        };

        match result {
            Some(Ok(_)) => {
                {
                    let mut state = state_arc.lock().unwrap();
                    state.is_advertising = false;
                }
                msg_env.send_and_clear(&pid, |env| {
                    (atoms::btleplug_peripheral_advertising_stopped(), atoms::ok()).encode(env)
                });
                info!("✅ Advertising stopped");
            }
            Some(Err(e)) => {
                warn!("Failed to stop advertising: {:?}", e);
                msg_env.send_and_clear(&pid, |env| {
                    (
                        atoms::btleplug_error(),
                        format!("Failed to stop advertising: {:?}", e),
                    )
                        .encode(env)
                });
            }
            None => {
                msg_env.send_and_clear(&pid, |env| {
                    (atoms::btleplug_error(), "Peripheral not initialized".to_string()).encode(env)
                });
            }
        }
    });

    Ok(atoms::ok())
}

/// Respond to a pending read request from Elixir
#[rustler::nif]
pub fn respond_to_read_request(
    env: Env,
    resource: ResourceArc<PeripheralManagerRef>,
    request_id: u64,
    value: Vec<u8>,
    success: bool,
) -> Result<rustler::Atom, RustlerError> {
    let state_arc = resource.0.clone();

    let response = ReadRequestResponse { value, success };

    let mut state = state_arc.lock().unwrap();
    if state.complete_read_request(request_id, response) {
        Ok(atoms::ok())
    } else {
        Err(RustlerError::Term(Box::new("Request not found or already completed")))
    }
}

/// Respond to a pending write request from Elixir
#[rustler::nif]
pub fn respond_to_write_request(
    env: Env,
    resource: ResourceArc<PeripheralManagerRef>,
    request_id: u64,
    success: bool,
) -> Result<rustler::Atom, RustlerError> {
    let state_arc = resource.0.clone();

    let response = WriteRequestResponse { success };

    let mut state = state_arc.lock().unwrap();
    if state.complete_write_request(request_id, response) {
        Ok(atoms::ok())
    } else {
        Err(RustlerError::Term(Box::new("Request not found or already completed")))
    }
}

/// Update a characteristic value and notify subscribers
#[rustler::nif]
pub fn update_characteristic(
    env: Env,
    resource: ResourceArc<PeripheralManagerRef>,
    characteristic_uuid: String,
    value: Vec<u8>,
) -> Result<rustler::Atom, RustlerError> {
    let state_arc = resource.0.clone();
    let (pid, peripheral_arc) = {
        let state = state_arc.lock().unwrap();
        (state.pid, state.peripheral.clone())
    };

    // Parse UUID
    let char_uuid_parsed = Uuid::from_str(&characteristic_uuid)
        .map_err(|e| RustlerError::Term(Box::new(format!("Invalid characteristic UUID: {}", e))))?;

    RUNTIME.spawn(async move {
        let mut msg_env = OwnedEnv::new();

        let result = {
            let mut peripheral_lock = peripheral_arc.lock().await;
            if let Some(ref mut peripheral) = *peripheral_lock {
                Some(peripheral.update_characteristic(char_uuid_parsed, value).await)
            } else {
                None
            }
        };

        match result {
            Some(Ok(_)) => {
                debug!("✅ Characteristic updated: {}", characteristic_uuid);
            }
            Some(Err(e)) => {
                warn!("Failed to update characteristic: {:?}", e);
                msg_env.send_and_clear(&pid, |env| {
                    (
                        atoms::btleplug_error(),
                        format!("Failed to update characteristic: {:?}", e),
                    )
                        .encode(env)
                });
            }
            None => {
                msg_env.send_and_clear(&pid, |env| {
                    (atoms::btleplug_error(), "Peripheral not initialized".to_string()).encode(env)
                });
            }
        }
    });

    Ok(atoms::ok())
}

/// Check if the peripheral is currently advertising
#[rustler::nif]
pub fn is_advertising(
    env: Env,
    resource: ResourceArc<PeripheralManagerRef>,
) -> Result<bool, RustlerError> {
    let state = resource.0.lock().unwrap();
    Ok(state.is_advertising)
}
