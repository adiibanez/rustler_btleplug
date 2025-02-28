#![allow(unused_imports)]
use crate::central_manager_state::CentralRef;

use rustler::{Encoder, Env, Error as RustlerError, ResourceArc, Term, NifStruct};
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