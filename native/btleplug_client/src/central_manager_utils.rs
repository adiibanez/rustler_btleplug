#![allow(unused_imports)]
use crate::central_manager_state::CentralRef;

use rustler::{Encoder, Env, Error as RustlerError, ResourceArc, Term};
use std::collections::HashMap;

use btleplug::api::{Central, Peripheral};

use log::{debug, info, warn};

use crate::RUNTIME;
use std::sync::Arc;

use serde_json::{Map, Value};

use btleplug::api::{CharPropFlags, PeripheralProperties};
use std::iter::FromIterator;

use btleplug::platform::Adapter;

pub async fn get_peripheral_properties(
    adapter: &Adapter,
    target_id: &str,
) -> Option<(Arc<btleplug::platform::Peripheral>, PeripheralProperties)> {
    if let Ok(peripherals) = adapter.peripherals().await {
        for peripheral in peripherals.iter() {
            if peripheral.id().to_string() == target_id {
                if let Ok(Some(properties)) = peripheral.properties().await {
                    return Some((Arc::new(peripheral.clone()), properties));
                }
            }
        }
    }
    None
}

pub fn debug_properties(properties: &PeripheralProperties) {
    let local_name = properties.local_name.as_deref().unwrap_or("(unknown)");
    let address = properties.address;
    let address_type = properties
        .address_type
        .map_or("Unknown".to_string(), |at| format!("{:?}", at));
    let tx_power_level = properties
        .tx_power_level
        .map_or("N/A".to_string(), |tx| tx.to_string());
    let rssi = properties.rssi.map_or("N/A".to_string(), |r| r.to_string());
    let manufacturer_data = properties.manufacturer_data.clone();
    let service_data = properties.service_data.clone();
    let services = properties
        .services
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<String>>();

    debug!("🔍 **Discovered Peripheral:**");
    debug!("   📛 Name: {:?}", local_name);
    debug!("   🔢 Address: {:?}", address);
    debug!("   🏷  Address Type: {:?}", address_type);
    debug!("   📡 TX Power Level: {:?}", tx_power_level);
    debug!("   📶 RSSI: {:?}", rssi);
    debug!("   Services: {:?}", services);

    if !manufacturer_data.is_empty() {
        debug!("   🏭 Manufacturer Data:");
        for (id, data) in manufacturer_data.iter() {
            debug!("     - ID {}: {:?}", id, data);
        }
    }

    if !service_data.is_empty() {
        debug!("   🔗 Service Data:");
        for (uuid, data) in service_data.iter() {
            debug!("     - UUID {}: {:?}", uuid, data);
        }
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


pub fn properties_to_map<'a>(env: Env<'a>, props: &PeripheralProperties) -> Term<'a> {
    let mut map = HashMap::new();

    map.insert("address", props.address.to_string().encode(env));
    map.insert(
        "address_type",
        props
            .address_type
            .map(|at| format!("{:?}", at))
            .unwrap_or_else(|| "Unknown".to_string())
            .encode(env),
    );
    map.insert(
        "local_name",
        props
            .local_name
            .as_deref()
            .unwrap_or("(unknown)")
            .encode(env),
    );
    map.insert(
        "tx_power_level",
        props
            .tx_power_level
            .map_or("N/A".into(), |tx| tx.to_string())
            .encode(env),
    );
    map.insert(
        "rssi",
        props
            .rssi
            .map_or("N/A".into(), |r| r.to_string())
            .encode(env),
    );

    // Convert manufacturer data
    let manufacturer_data: HashMap<String, Vec<u8>> = props
        .manufacturer_data
        .iter()
        .map(|(id, data)| (id.to_string(), data.clone()))
        .collect();
    map.insert("manufacturer_data", manufacturer_data.encode(env));

    // Convert service data
    let service_data: HashMap<String, Vec<u8>> = props
        .service_data
        .iter()
        .map(|(uuid, data)| (uuid.to_string(), data.clone()))
        .collect();
    map.insert("service_data", service_data.encode(env));

    // Convert services to a list of UUID strings
    let services: Vec<String> = props.services.iter().map(|s| s.to_string()).collect();
    map.insert("services", services.encode(env));

    map.encode(env)
}

pub async fn adapter_state_to_map(adapter: &Adapter) -> HashMap<String, Value> {
    let mut result = HashMap::new();

    // 1️⃣ Adapter Name
    let adapter_name = adapter
        .adapter_info()
        .await
        .unwrap_or_else(|_| "Unknown Adapter".to_string());
    result.insert("adapter".to_string(), Value::String(adapter_name));

    // 2️⃣ Fetch Peripherals
    let peripherals = adapter.peripherals().await.unwrap_or_default();
    let mut peripherals_list = Vec::new();

    for peripheral in peripherals.iter() {
        let peripheral_id = peripheral.id().to_string();
        let peripheral_name = match peripheral.properties().await {
            Ok(Some(props)) => props.local_name.unwrap_or(peripheral_id.clone()),
            _ => peripheral_id.clone(),
        };

        let mut peripheral_map = HashMap::new();
        peripheral_map.insert("id".to_string(), Value::String(peripheral_id.clone()));
        peripheral_map.insert("name".to_string(), Value::String(peripheral_name.clone()));

        // 3️⃣ Fetch Services
        let services = peripheral.services();
        let mut services_list = Vec::new();

        for service in services.iter() {
            let service_id = service.uuid.to_string();
            let mut service_map = HashMap::new();
            service_map.insert("uuid".to_string(), Value::String(service_id.clone()));

            // 4️⃣ Fetch Characteristics
            let mut characteristics_list = Vec::new();
            for char in service.characteristics.iter() {
                let char_id = char.uuid.to_string();
                let char_props = if char.properties.contains(CharPropFlags::NOTIFY) {
                    "Notify"
                } else if char.properties.contains(CharPropFlags::READ) {
                    "Read"
                } else {
                    "Unknown"
                };

                let mut char_map = HashMap::new();
                char_map.insert("uuid".to_string(), Value::String(char_id.clone()));
                char_map.insert(
                    "properties".to_string(),
                    Value::String(char_props.to_string()),
                );

                characteristics_list.push(Value::Object(Map::from_iter(char_map)));
                // ✅ FIXED HERE
            }

            service_map.insert(
                "characteristics".to_string(),
                Value::Array(characteristics_list),
            );
            services_list.push(Value::Object(Map::from_iter(service_map))); // ✅ FIXED HERE
        }

        peripheral_map.insert("services".to_string(), Value::Array(services_list));
        peripherals_list.push(Value::Object(Map::from_iter(peripheral_map))); // ✅ FIXED HERE
    }

    result.insert("peripherals".to_string(), Value::Array(peripherals_list));
    result
}



pub async fn adapter_state_to_mermaid_mindmap(adapter: &Adapter) -> String {
    let mut output = String::from("mindmap\n");

    // 1️⃣ Adapter Node
    let adapter_name = adapter
        .adapter_info()
        .await
        .unwrap_or_else(|_| "Unknown Adapter".to_string());
    output.push_str(&format!("  root((Adapter: {}))\n", adapter_name));

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
            "  root --> {}((Peripheral: {}<br>{}<br>{}))\n",
            peripheral_id, peripheral_name, rssi_display, tx_power_display
        ));

        // 3️⃣ Fetch Services
        let services = peripheral.services();
        for service in services.iter() {
            let service_id = service.uuid.to_string();
            output.push_str(&format!(
                "  {} --> {}((Service: {}))\n",
                peripheral_id, service_id, service_id
            ));

            // 4️⃣ Fetch Characteristics
            for char in service.characteristics.iter() {
                let char_props = char.properties;
                let char_id = char.uuid.to_string();
                let char_flags = [
                    if char_props.contains(CharPropFlags::READ) { "Read" } else { "" },
                    if char_props.contains(CharPropFlags::WRITE) { "Write" } else { "" },
                    if char_props.contains(CharPropFlags::NOTIFY) { "Notify" } else { "" },
                    if char_props.contains(CharPropFlags::INDICATE) { "Indicate" } else { "" },
                ]
                .iter()
                .filter(|s| !s.is_empty())
                .cloned()
                .collect::<Vec<_>>()
                .join(", ");

                output.push_str(&format!(
                    "  {} --> {}((Characteristic: {} {}))\n",
                    service_id, char_id, char_id, char_flags
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
