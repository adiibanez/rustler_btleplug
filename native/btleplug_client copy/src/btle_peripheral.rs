use btleplug::api::Peripheral as PeripheralTrait; // ✅ Import the trait
use btleplug::api::{Characteristic, ValueNotification};
use btleplug::platform::Peripheral;
use futures::stream::StreamExt;
use std::sync::Arc;

#[derive(Clone)]
pub struct BtlePeripheral {
    pub id: String,
    peripheral: Arc<Peripheral>,
}

impl BtlePeripheral {
    pub async fn new(peripheral: Peripheral) -> Self {
        let id = peripheral.id().to_string();
        Self {
            id,
            peripheral: std::sync::Arc::new(peripheral),
        }
    }

    pub async fn connect(&self) -> Result<(), String> {
        self.peripheral
            .as_ref()
            .connect()
            .await
            .map_err(|e| format!("{:?}", e))
    }

    pub async fn read_characteristic(
        &self,
        characteristic: &Characteristic,
    ) -> Result<Vec<u8>, String> {
        self.peripheral
            .as_ref()
            .read(characteristic)
            .await
            .map_err(|e| format!("{:?}", e))
    }

    pub async fn subscribe_notifications(&self) {
        let mut stream = self.peripheral.as_ref().notifications().await.unwrap();
        while let Some(notification) = stream.next().await {
            println!("Received notification: {:?}", notification);
        }
    }
}
