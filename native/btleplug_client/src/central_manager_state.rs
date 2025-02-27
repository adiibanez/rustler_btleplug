use crate::atoms;
use crate::peripheral::PeripheralRef;
use crate::peripheral::PeripheralState;

use log::{debug, info, warn};
use rustler::{Encoder, Env, Error as RustlerError, LocalPid, OwnedEnv, Resource, ResourceArc, Term};
use rustler::{NifStruct, NifUnitEnum};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::iter::FromIterator; 
use btleplug::api::{
    CharPropFlags, Central, CentralEvent, Manager as _, Peripheral, PeripheralProperties, ScanFilter,
};
use btleplug::platform::{Adapter, Manager};
use futures::StreamExt;

use crate::RUNTIME;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio::sync::RwLock;
use tokio::time::{sleep, timeout, Duration};

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