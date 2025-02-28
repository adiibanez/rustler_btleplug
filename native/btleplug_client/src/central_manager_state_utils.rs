#![allow(unused_imports)]
use crate::central_manager_state::get_peripheral_rssi_cache;
use crate::central_manager_state::{CentralRef, DISCOVERED_SERVICES, RSSI_CACHE};
use crate::central_manager_utils::{
    get_characteristic_properties, get_peripheral_properties, properties_to_map,
};

use rustler::{Encoder, Env, Error as RustlerError, NifMap, NifStruct, ResourceArc, Term};
//use serde_rustler::{from_term, to_term};
use std::collections::{HashMap, HashSet};

use btleplug::api::{Central, Characteristic, Peripheral};

use log::{debug, info, warn};

use crate::RUNTIME;
use std::sync::Arc;

use serde_json::{Map, Value};

use btleplug::api::{CharPropFlags, PeripheralProperties};
use std::iter::FromIterator;

use btleplug::platform::Adapter;

#[derive(NifStruct)]
#[module = "RustlerBtleplug.AdapterInfo"]
struct AdapterInfo {
    name: String,
}

/// ✅ **NifStruct for Peripheral** (Now contains services directly)
#[derive(NifStruct)]
#[module = "RustlerBtleplug.PeripheralInfo"]
struct PeripheralInfo {
    id: String,
    name: String,
    rssi: Option<i16>,
    rssi_cache: Vec<(i64, i16)>,
    is_connected: bool,
    tx_power: Option<i16>,
    services: Vec<ServiceInfo>, // **Nested directly inside Peripheral**
}

/// ✅ **NifStruct for Service** (Now contains characteristics directly)
#[derive(NifStruct)]
#[module = "RustlerBtleplug.ServiceInfo"]
struct ServiceInfo {
    uuid: String,
    characteristics: Vec<CharacteristicInfo>, // **Nested directly inside Service**
}

/// ✅ **NifStruct for Characteristic**
#[derive(NifStruct, Debug, Clone, PartialEq, Eq, Hash)]
#[module = "RustlerBtleplug.CharacteristicInfo"]
struct CharacteristicInfo {
    uuid: String,
    properties: Vec<String>,
}

/// ✅ **NifMap: Main Struct for Adapter State**
#[derive(NifMap)]
struct AdapterState {
    adapter: AdapterInfo,
    peripherals: Vec<PeripheralInfo>, // **Now a list instead of a HashMap**
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

    let adapter_state = tokio::task::block_in_place(|| {
        let runtime = tokio::runtime::Runtime::new().expect("Failed to create runtime");
        runtime.block_on(adapter_state_to_map(&adapter))
    });

    Ok(adapter_state)
}

async fn adapter_state_to_map(adapter: &Adapter) -> AdapterState {
    let adapter_name = adapter
        .adapter_info()
        .await
        .unwrap_or_else(|_| "Unknown Adapter".to_string());

    let adapter_info = AdapterInfo { name: adapter_name };

    let peripherals = adapter.peripherals().await.unwrap_or_default();
    let mut peripherals_vec = Vec::new();

    // 🏷 Read Cached Advertised Services
    let cache = DISCOVERED_SERVICES.read().await;

    for peripheral in peripherals.iter() {
        let peripheral_id = peripheral.id().to_string();

        let properties = match get_peripheral_properties(adapter, &peripheral_id).await {
            Some((_, props)) => props,
            None => continue,
        };

        let is_connected = peripheral.is_connected().await.unwrap_or(false);
        let rssi_cache = get_peripheral_rssi_cache(&peripheral_id)
            .await
            .unwrap_or_default();

        //let peripheral_rssi_cache = rssi_cache.get(&peripheral_id).map(|v| v.clone());

        let mut service_map: HashMap<String, HashSet<CharacteristicInfo>> = HashMap::new();

        // 🔥 Merge advertised services from cache
        if let Some(advertised_services) = cache.get(&peripheral_id) {
            for service_uuid in advertised_services {
                service_map
                    .entry(service_uuid.clone())
                    .or_insert(HashSet::new());
            }
        }

        // 3️⃣ Fetch Live Services & Characteristics
        let services = peripheral.services();
        for service in services.iter() {
            let service_id = service.uuid.to_string();
            let char_set = service_map
                .entry(service_id.clone())
                .or_insert(HashSet::new());

            for char in service.characteristics.iter() {
                let char_props = get_characteristic_properties(&char);
                char_set.insert(CharacteristicInfo {
                    uuid: char.uuid.to_string(),
                    properties: char_props,
                });
            }
        }

        // Convert HashMap into Vec<ServiceInfo>
        let service_infos: Vec<ServiceInfo> = service_map
            .into_iter()
            .map(|(uuid, characteristics)| ServiceInfo {
                uuid,
                characteristics: characteristics.into_iter().collect(),
            })
            .collect();

        let peripheral_info = PeripheralInfo {
            id: peripheral_id.clone(),
            name: properties.local_name.unwrap_or(peripheral_id.clone()),
            rssi: properties.rssi,
            rssi_cache: rssi_cache,
            is_connected: is_connected,
            tx_power: properties.tx_power_level,
            services: service_infos,
        };

        peripherals_vec.push(peripheral_info);
    }

    AdapterState {
        adapter: adapter_info,
        peripherals: peripherals_vec,
    }
}

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
