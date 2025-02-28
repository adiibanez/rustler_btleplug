#![allow(unused_imports)]
use crate::central_manager_state::CentralRef;
use crate::central_manager_utils::{get_peripheral_properties, properties_to_map};

use rustler::{Encoder, Env, Error as RustlerError, ResourceArc, Term, NifStruct, NifMap};
//use serde_rustler::{from_term, to_term};
use std::collections::HashMap;

use btleplug::api::{Central, Peripheral};

use log::{debug, info, warn};

use crate::RUNTIME;
use std::sync::Arc;

use serde_json::{Map, Value};

use btleplug::api::{CharPropFlags, PeripheralProperties};
use std::iter::FromIterator;

use btleplug::platform::Adapter;

#[rustler::nif]
pub fn get_adapter_state_graph(
    env: Env<'_>,
    resource: ResourceArc<CentralRef>,
    variant: String, // Accepts "graph" or "mindmap"
) -> Result<Term<'_>, RustlerError> {
    let resource_arc = resource.0.clone();

    let (adapter, _pid) = {
        let central_state = resource_arc.lock().unwrap();
        (central_state.adapter.clone(), central_state.pid)
    };

    let (tx, rx) = tokio::sync::oneshot::channel();

    RUNTIME.spawn(async move {
        let state_graph = match variant.as_str() {
            "graph" => adapter_state_to_mermaid_graph(&adapter).await,
            _ => adapter_state_to_mermaid_mindmap(&adapter).await, // Default to graph TD
        };
        let _ = tx.send(state_graph);
    });

    match rx.blocking_recv() {
        Ok(graph) => Ok(graph.encode(env)),
        Err(_) => Err(RustlerError::Term(Box::new(
            "Failed to retrieve adapter state graph".to_string(),
        ))),
    }
}



#[derive(NifStruct)]
#[module = "RustlerBtleplug.AdapterInfo"]
struct AdapterInfo {
    name: String,
}

#[derive(NifStruct)]
#[module = "RustlerBtleplug.PeripheralInfo"]
struct PeripheralInfo {
    id: String,
    name: String,
    rssi: Option<i16>,
    tx_power: Option<i16>,
}

#[derive(NifStruct)]
#[module = "RustlerBtleplug.ServiceInfo"]
struct ServiceInfo {
    uuid: String,
    peripheral_id: String,
}

/// ✅ Struct to store characteristic information
#[derive(NifStruct)]
#[module = "RustlerBtleplug.CharacteristicInfo"]
struct CharacteristicInfo {
    uuid: String,
    service_uuid: String,
    properties: String,
}

#[derive(NifMap)]
struct AdapterState {
    adapter: AdapterInfo,
    peripherals: HashMap<String, PeripheralInfo>,
    services: HashMap<String, ServiceInfo>,
    characteristics: HashMap<String, CharacteristicInfo>,
}


#[rustler::nif(schedule = "DirtyIo")]
pub fn get_adapter_state_map(
    env: Env<'_>,
    resource: ResourceArc<CentralRef>,
) -> Result<AdapterState, RustlerError> {
    let resource_arc = resource.0.clone();

    let (adapter, _pid) = {
        let central_state = resource_arc.lock().unwrap();
        (central_state.adapter.clone(), central_state.pid)
    };

    // 🏁 **Ensure the async function runs synchronously**
    let adapter_state = tokio::task::block_in_place(|| {
        let runtime = tokio::runtime::Runtime::new().expect("Failed to create runtime");
        runtime.block_on(adapter_state_to_map(&adapter))
    });

    Ok(adapter_state)
}

// #[derive(NifStruct)]
// #[module = "Peripheral"]
// struct Peripheral {
//     id: String,
//     name: String,
//     services: Vec<Service>,
// }

// #[derive(NifStruct)]
// #[module = "Service"]
// struct Service {
//     uuid: String,
//     characteristics: Vec<Characteristic>,
// }

// #[derive(NifStruct)]
// #[module = "Characteristic"]
// struct Characteristic {
//     uuid: String,
//     properties: String,
// }


//use btleplug::api::{Adapter, CharPropFlags};

async fn adapter_state_to_map(adapter: &Adapter) -> AdapterState {
    // 1️⃣ Get Adapter Info
    let adapter_name = adapter
        .adapter_info()
        .await
        .unwrap_or_else(|_| "Unknown Adapter".to_string());

    let adapter_info = AdapterInfo { name: adapter_name };

    // 2️⃣ Get Peripherals
    let peripherals = adapter.peripherals().await.unwrap_or_default();
    let mut peripherals_map = HashMap::new();
    let mut services_map = HashMap::new();
    let mut characteristics_map = HashMap::new();

    for peripheral in peripherals.iter() {
        let peripheral_id = peripheral.id().to_string();

        // 🔍 Get Peripheral Properties
        let properties = match get_peripheral_properties(adapter, &peripheral_id).await {
            Some((_, props)) => props,
            None => continue, // Skip if no properties found
        };

        let peripheral_info = PeripheralInfo {
            id: peripheral_id.clone(),
            name: properties.local_name.unwrap_or(peripheral_id.clone()),
            rssi: properties.rssi,
            tx_power: properties.tx_power_level,
        };

        peripherals_map.insert(peripheral_id.clone(), peripheral_info);

        // 3️⃣ Fetch Services
        let services = peripheral.services();
        for service in services.iter() {
            let service_id = service.uuid.to_string();

            let service_info = ServiceInfo {
                uuid: service_id.clone(),
                peripheral_id: peripheral_id.clone(),
            };

            services_map.insert(service_id.clone(), service_info);

            // 4️⃣ Fetch Characteristics
            for char in service.characteristics.iter() {
                let char_id = char.uuid.to_string();
                let char_props = [
                    if char.properties.contains(CharPropFlags::READ) { "Read" } else { "" },
                    if char.properties.contains(CharPropFlags::WRITE) { "Write" } else { "" },
                    if char.properties.contains(CharPropFlags::NOTIFY) { "Notify" } else { "" },
                    if char.properties.contains(CharPropFlags::INDICATE) { "Indicate" } else { "" },
                ]
                .iter()
                .filter(|s| !s.is_empty())
                .cloned()
                .collect::<Vec<_>>()
                .join(", ");

                let characteristic_info = CharacteristicInfo {
                    uuid: char_id.clone(),
                    service_uuid: service_id.clone(),
                    properties: char_props,
                };

                characteristics_map.insert(char_id.clone(), characteristic_info);
            }
        }
    }

    AdapterState {
        adapter: adapter_info,
        peripherals: peripherals_map,
        services: services_map,
        characteristics: characteristics_map,
    }
}

pub async fn adapter_state_to_mermaid_mindmap(adapter: &Adapter) -> String {
    let mut output = String::from("mindmap\n");

    // 1️⃣ Root Node: Adapter
    let adapter_name = adapter
        .adapter_info()
        .await
        .unwrap_or_else(|_| "Unknown Adapter".to_string());
    output.push_str(&format!("adapter((Adapter: {}))\n", adapter_name));

    // 2️⃣ Fetch Peripherals
    let peripherals = adapter.peripherals().await.unwrap_or_default();
    for peripheral in peripherals.iter() {
        let peripheral_id = peripheral.id().to_string();

        // 🔍 Get peripheral properties
        let properties = match get_peripheral_properties(adapter, &peripheral_id).await {
            Some((_, props)) => props,
            None => continue, // Skip if no properties found
        };

        let peripheral_name = properties
            .local_name
            .unwrap_or_else(|| peripheral_id.clone());

        let rssi_display = properties
            .rssi
            .map(|rssi| format!("RSSI: {}dBm", rssi))
            .unwrap_or_else(|| "RSSI: N/A".to_string());

        let tx_power_display = properties
            .tx_power_level
            .map(|tx| format!("TX Power: {}dBm", tx))
            .unwrap_or_else(|| "TX Power: N/A".to_string());

        let connection_status = if peripheral.is_connected().await.unwrap_or(false) {
            "**Connected** :::green"
        } else {
            "**Disconnected** :::red"
        };

        // 🏷️ Peripheral Node (Indented under Adapter)
        output.push_str(&format!(
            "    adapter\n        {}((Peripheral: **{}**))\n",
            peripheral_id, peripheral_name
        ));
        output.push_str(&format!("            {}\n", rssi_display));
        output.push_str(&format!("            {}\n", tx_power_display));
        output.push_str(&format!("            {}\n", connection_status));

        // 3️⃣ Fetch Services
        let services = peripheral.services();
        for service in services.iter() {
            let service_id = service.uuid.to_string();
            output.push_str(&format!(
                "            {}[Service: {}]\n",
                service_id, service_id
            ));

            // 4️⃣ Fetch Characteristics
            for char in service.characteristics.iter() {
                let char_id = char.uuid.to_string();
                let char_props = char.properties;
                let char_flags = format!(
                    "[{}]",
                    [
                        if char_props.contains(CharPropFlags::READ) {
                            "Read"
                        } else {
                            ""
                        },
                        if char_props.contains(CharPropFlags::WRITE) {
                            "Write"
                        } else {
                            ""
                        },
                        if char_props.contains(CharPropFlags::NOTIFY) {
                            "Notify"
                        } else {
                            ""
                        },
                        if char_props.contains(CharPropFlags::INDICATE) {
                            "Indicate"
                        } else {
                            ""
                        },
                    ]
                    .iter()
                    .filter(|s| !s.is_empty())
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
                );
                output.push_str(&format!(
                    "                {}[Characteristic: {} {}]\n",
                    char_id, char_id, char_flags
                ));
            }
        }
    }
    output
}

pub async fn adapter_state_to_mermaid_graph(adapter: &Adapter) -> String {
    let mut output = String::from("graph TD\n");

    // 1️⃣ Adapter Node
    let adapter_name = adapter
        .adapter_info()
        .await
        .unwrap_or_else(|_| "Unknown Adapter".to_string());
    output.push_str(&format!("    Adapter[\"Adapter: {}\"]\n", adapter_name));

    // 2️⃣ Fetch Peripherals
    let peripherals = adapter.peripherals().await.unwrap_or_default();
    for peripheral in peripherals.iter() {
        let peripheral_id = peripheral.id().to_string();

        // 🔍 Get peripheral properties
        let properties = match get_peripheral_properties(adapter, &peripheral_id).await {
            Some((_, props)) => props,
            None => continue, // Skip if no properties found
        };

        let peripheral_name = properties
            .local_name
            .unwrap_or_else(|| peripheral_id.clone());
        let rssi_display = properties
            .rssi
            .map(|rssi| format!("RSSI: {}dBm", rssi))
            .unwrap_or_else(|| "RSSI: N/A".to_string());

        let tx_power_display = properties
            .tx_power_level
            .map(|tx| format!("TX Power: {}dBm", tx))
            .unwrap_or_else(|| "TX Power: N/A".to_string());

        output.push_str(&format!(
            "    Adapter --> {}[\"Peripheral: {}<br>{}<br>{}\"]\n",
            peripheral_id, peripheral_name, rssi_display, tx_power_display
        ));

        // 3️⃣ Fetch Services
        let services = peripheral.services();
        for service in services.iter() {
            let service_id = service.uuid.to_string();
            output.push_str(&format!(
                "    {} --> {}[\"Service: {}\"]\n",
                peripheral_id, service_id, service_id
            ));

            // 4️⃣ Fetch Characteristics
            for char in service.characteristics.iter() {
                let char_props = char.properties;
                let char_id = char.uuid.to_string();
                let char_flags = format!(
                    "({})",
                    [
                        if char_props.contains(CharPropFlags::READ) {
                            "Read"
                        } else {
                            ""
                        },
                        if char_props.contains(CharPropFlags::WRITE) {
                            "Write"
                        } else {
                            ""
                        },
                        if char_props.contains(CharPropFlags::NOTIFY) {
                            "Notify"
                        } else {
                            ""
                        },
                        if char_props.contains(CharPropFlags::INDICATE) {
                            "Indicate"
                        } else {
                            ""
                        },
                    ]
                    .iter()
                    .filter(|s| !s.is_empty())
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
                );

                output.push_str(&format!(
                    "    {} --> {}[\"Characteristic: {} {}\"]\n",
                    service_id, char_id, char_id, char_flags
                ));
            }
        }
    }

    output
}
