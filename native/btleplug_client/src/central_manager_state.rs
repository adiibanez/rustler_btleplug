use crate::peripheral::PeripheralRef;

use rustler::{LocalPid, ResourceArc};
use std::collections::HashMap;
 
use btleplug::api::CentralEvent;
use btleplug::platform::{Adapter, Manager};

use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio::sync::RwLock;

pub struct CentralRef(pub(crate) Arc<Mutex<CentralManagerState>>);

pub struct CentralManagerState {
    pub pid: LocalPid,
    pub adapter: Adapter,
    pub manager: Manager,
    pub event_sender: mpsc::Sender<CentralEvent>,
    pub event_receiver: Arc<RwLock<mpsc::Receiver<CentralEvent>>>,
    pub discovered_peripherals: Arc<Mutex<HashMap<String, ResourceArc<PeripheralRef>>>>,
}

impl CentralManagerState {
    pub fn new(
        pid: LocalPid,
        manager: Manager,
        adapter: Adapter,
        event_sender: mpsc::Sender<CentralEvent>,
        event_receiver: Arc<RwLock<mpsc::Receiver<CentralEvent>>>,
    ) -> Self {
        CentralManagerState {
            pid,
            manager,
            adapter,
            event_sender,
            event_receiver,
            discovered_peripherals: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}