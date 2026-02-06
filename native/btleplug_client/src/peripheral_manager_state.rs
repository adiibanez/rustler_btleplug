//! State management for BLE Peripheral mode.
//!
//! This module handles the state for the peripheral manager, including
//! tracking pending read/write requests from connected centrals.

use ble_peripheral_rust::gatt::service::Service;
use ble_peripheral_rust::Peripheral;
use rustler::LocalPid;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;

/// Response to a read request from a central
#[derive(Debug, Clone)]
pub struct ReadRequestResponse {
    pub value: Vec<u8>,
    pub success: bool,
}

/// Response to a write request from a central
#[derive(Debug, Clone)]
pub struct WriteRequestResponse {
    pub success: bool,
}

/// State for the peripheral manager (sync-safe parts)
pub struct PeripheralManagerState {
    /// The Elixir process to send events to
    pub pid: LocalPid,
    /// The ble-peripheral-rust Peripheral instance (wrapped in async mutex for async access)
    pub peripheral: Arc<tokio::sync::Mutex<Option<Peripheral>>>,
    /// Pending read requests waiting for responses from Elixir
    pub pending_read_requests: HashMap<u64, oneshot::Sender<ReadRequestResponse>>,
    /// Pending write requests waiting for responses from Elixir
    pub pending_write_requests: HashMap<u64, oneshot::Sender<WriteRequestResponse>>,
    /// Counter for generating unique request IDs
    request_counter: AtomicU64,
    /// Registered services
    pub services: Vec<Service>,
    /// Whether the peripheral is currently advertising
    pub is_advertising: bool,
    /// Device name for advertising
    pub device_name: Option<String>,
}

impl PeripheralManagerState {
    pub fn new(pid: LocalPid) -> Self {
        Self {
            pid,
            peripheral: Arc::new(tokio::sync::Mutex::new(None)),
            pending_read_requests: HashMap::new(),
            pending_write_requests: HashMap::new(),
            request_counter: AtomicU64::new(0),
            services: Vec::new(),
            is_advertising: false,
            device_name: None,
        }
    }

    /// Generate a new unique request ID
    pub fn next_request_id(&self) -> u64 {
        self.request_counter.fetch_add(1, Ordering::SeqCst)
    }

    /// Add a pending read request
    pub fn add_read_request(&mut self, request_id: u64, sender: oneshot::Sender<ReadRequestResponse>) {
        self.pending_read_requests.insert(request_id, sender);
    }

    /// Complete a pending read request
    pub fn complete_read_request(&mut self, request_id: u64, response: ReadRequestResponse) -> bool {
        if let Some(sender) = self.pending_read_requests.remove(&request_id) {
            sender.send(response).is_ok()
        } else {
            false
        }
    }

    /// Add a pending write request
    pub fn add_write_request(&mut self, request_id: u64, sender: oneshot::Sender<WriteRequestResponse>) {
        self.pending_write_requests.insert(request_id, sender);
    }

    /// Complete a pending write request
    pub fn complete_write_request(&mut self, request_id: u64, response: WriteRequestResponse) -> bool {
        if let Some(sender) = self.pending_write_requests.remove(&request_id) {
            sender.send(response).is_ok()
        } else {
            false
        }
    }
}

/// Resource wrapper for the peripheral manager state
pub struct PeripheralManagerRef(pub Arc<Mutex<PeripheralManagerState>>);

impl PeripheralManagerRef {
    pub fn new(state: PeripheralManagerState) -> Self {
        Self(Arc::new(Mutex::new(state)))
    }
}

impl Drop for PeripheralManagerState {
    fn drop(&mut self) {
        log::debug!("PeripheralManagerState destructor called.");
    }
}
