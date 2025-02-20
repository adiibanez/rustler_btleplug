use crate::atoms;
use crate::btle_peripheral::BtlePeripheral;
use crate::elixir_bridge::ElixirBridge;
use btleplug::api::{Central, CentralEvent, Characteristic, Manager as _, Peripheral, ScanFilter, WriteType};
use btleplug::platform::{Adapter, Manager};
use std::sync::{Arc};
use uuid::Uuid;
//use futures_util::stream::StreamExt;
use tokio_stream::StreamExt;

use rustler::{Env, ResourceArc, Term};
use std::collections::HashMap;
use tokio::sync::Mutex; // Use Tokio Mutex
use std::str::FromStr;

pub struct BtleDeviceConnection {
    // Store connection specific state here, like handles to characteristics
    peripheral: btleplug::platform::Peripheral,
    characteristics: Vec<Characteristic>,
}

pub struct BtleManager {
    manager: Manager,
    adapter: Arc<Adapter>,
    bridge: Arc<ElixirBridge>,
    //devices: Arc<Mutex<Vec<BtlePeripheral>>>,  // Remove this
    connections: Mutex<HashMap<String, BtleDeviceConnection>>, // Device ID -> Connection
}


impl Drop for BtleDeviceConnection {
    fn drop(&mut self) {
        println!("Dropping BtleConnection for device: {}", self.peripheral);
        // Add any cleanup code here (e.g., disconnect from the device)
    }
}


impl BtleManager {
    pub async fn new(env: &Env<'_>) -> Result<Self, String> {
        let manager = Manager::new().await.map_err(|e| format!("{:?}", e))?;
        let adapter = manager
            .adapters()
            .await
            .map_err(|e| format!("{:?}", e))?
            .into_iter()
            .next()
            .ok_or("No Bluetooth adapter found")?;

        Ok(Self {
            manager,
            bridge: ElixirBridge::new(env),
            adapter: Arc::new(adapter),
            //devices: Arc::new(Mutex::new(Vec::new())), // Remove this
            connections: Mutex::new(HashMap::new()),
        })
    }

     pub fn get_info(&self) -> String {
        format!("DEBUG get info: BtleManager instance address: {:p}", self)
    }

    pub async fn scan(&self, bridge: Arc<ElixirBridge>) -> Result<Vec<String>, String> {
        let central = self.adapter.clone();
        let mut device_ids = Vec::new(); // Vector to store discovered device IDs

        println!("DEBUG scan: BtleManager instance address: {:p}", self);

        bridge.send_message((
            atoms::btleplug_got_central(),
            format!("Got central: Info {:?}", central.adapter_info().await),
        ));

        let mut events = central.events().await.expect("Event stream failed");
        let _ = central.start_scan(ScanFilter::default()).await.map_err(|e| format!("Start scan error: {:?}", e))?;

        while let Some(event) = events.next().await {
            match event {
                CentralEvent::DeviceDiscovered(id) => {
                    println!("DEBUG: Discovered device ID: {:?}", id);
                    let id_str = id.to_string();
                    device_ids.push(id_str.clone()); // Store the device ID
                    bridge.send_message((
                        atoms::btleplug_device_discovered(),
                        format!("Device discovered: Id {:?}", id),
                    ));
                }
                CentralEvent::StateUpdate(state) => {
                    println!("AdapterStatusUpdate {:?}", state);

                    bridge.send_message((
                        atoms::btleplug_adapter_status_update(),
                        format!("AdapterStatusUpdate: State {:?}", state),
                    ));
                }
                CentralEvent::DeviceConnected(id) => {
                    println!("DeviceConnected: {:?}", id);
                    bridge.send_message((
                        atoms::btleplug_device_connected(),
                        format!("DeviceConnected: ID {:?}", id),
                    ));
                }
                CentralEvent::DeviceDisconnected(id) => {
                    println!("DeviceDisconnected: {:?}", id);
                    bridge.send_message((
                        atoms::btleplug_device_disconnected(),
                        format!("DeviceDisconnected: ID {:?}", id),
                    ));
                }
                CentralEvent::ManufacturerDataAdvertisement {
                    id,
                    manufacturer_data,
                } => {
                    println!(
                        "ManufacturerDataAdvertisement: {:?}, {:?}",
                        id, manufacturer_data
                    );
                    bridge.send_message((
                        atoms::btleplug_manufacturer_data_advertisement(),
                        format!(
                            "ManufacturerDataAdvertisement: ID {:?}, DATA: {:?}",
                            id, manufacturer_data
                        ),
                    ));
                }
                CentralEvent::ServiceDataAdvertisement { id, service_data } => {
                    println!("ServiceDataAdvertisement: {:?}, {:?}", id, service_data);

                    bridge.send_message((
                        atoms::btleplug_service_data_advertisement(),
                        format!(
                            "ServiceDataAdvertisement: ID {:?}, DATA: {:?}",
                            id, service_data
                        ),
                    ));
                }
                CentralEvent::ServicesAdvertisement { id, services } => {
                    let services: Vec<String> =
                        services.into_iter().map(|s| s.to_string()).collect();
                    println!("ServicesAdvertisement: {:?}, {:?}", id, services);

                    bridge.send_message((
                        atoms::btleplug_services_advertisement(),
                        format!(
                            "ServicesAdvertisement: ID {:?}, SERVICES: {:?}",
                            id, services
                        ),
                    ));
                }

                _ => {}
            }
        }
        Ok(device_ids) // Return the list of device IDs
    }

    pub async fn connect_device(
        &self,
        peripheral_id: String
    ) -> Result<(), String> {
         println!("DEBUG connect: BtleManager instance address: {:p}", self);

        //let peripheral = self.adapter.peripheral(match Uuid::parse_str(&peripheral_id)).await.map_err(|e| format!("Peripheral error: {:?}", e))?;

        // self.adapter.connect_device(&peripheral_id).await
        //    .map(|_| ())
        //    .map_err(|e| format!("Connect error: {:?}", e))?;

        // let peripheral = self.connections.get(&peripheral_id).expect("Device not found");

      println!("DEBUG: Device found! Attempting to connect...");
        peripheral.connect().await.map_err(|e| format!("Connect error: {:?}", e))?;

        let characteristics = peripheral.discover_characteristics().await.map_err(|e| format!("Discover characteristics error: {:?}", e))?;

        let connection = BtleDeviceConnection {
            peripheral: peripheral,
            characteristics: characteristics,
        };

        let mut connections = self.connections.lock().await;
        connections.insert(peripheral_id.clone(), connection);

        self.bridge.send_device_connected(&peripheral_id);

            Ok(())
    }



    pub async fn read_characteristic(
    &self,
    device_id: String,
    characteristic_id: String,
) -> Result<Vec<u8>, String> {
    let connections = self.connections.lock().await; // Acquire the lock asynchronously

    if let Some(connection) = connections.get(&device_id) {
        // connection is a reference to a BtleDeviceConnection, so you can access its fields
        // You might need to find the specific characteristic you want to read
        // and then call peripheral.read() on it

        // Example: Assuming you have a way to identify the characteristic:
        if let Some(characteristic) = connection.characteristics.iter().find(|c| c.uuid.to_string() == characteristic_id) {
            connection.peripheral.read(characteristic).await.map_err(|e| format!("{:?}", e))
        } else {
            Err(format!("Characteristic {} not found on device {}", characteristic_id, device_id))
        }

    } else {
        Err("Device not connected".to_string())
    }
}


}