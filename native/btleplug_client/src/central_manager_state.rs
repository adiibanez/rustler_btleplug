use crate::peripheral::PeripheralRef;

use rustler::{LocalPid, ResourceArc};
use std::collections::HashMap;

use btleplug::api::CentralEvent;
use btleplug::platform::{Adapter, Manager};

use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use tokio::sync::RwLock;

lazy_static::lazy_static! {
    pub static ref DISCOVERED_SERVICES: RwLock<HashMap<String, Vec<String>>> = RwLock::new(HashMap::new());
}

const RSSI_HISTORY_LIMIT: usize = 10; // Limit number of entries per device

lazy_static::lazy_static! {
   pub static ref RSSI_CACHE: RwLock<HashMap<String, Vec<(i64, i16)>>> = RwLock::new(HashMap::new());
}

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

pub async fn cache_rssi(peripheral_id: &str, rssi: i16) {
    let mut cache = RSSI_CACHE.write().await;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let entry = cache
        .entry(peripheral_id.to_string())
        .or_insert_with(Vec::new);

    entry.push((timestamp, rssi)); // Store (timestamp, rssi)

    // Trim to the last N entries
    if entry.len() > RSSI_HISTORY_LIMIT {
        entry.drain(0..(entry.len() - RSSI_HISTORY_LIMIT));
    }
}

pub async fn get_peripheral_rssi_cache(peripheral_id: &str) -> Option<Vec<(i64, i16)>> {
    let cache = RSSI_CACHE.read().await;
    if let Some(entry) = cache.get(peripheral_id) {
        return Some(entry.clone());
    }
    None
}
