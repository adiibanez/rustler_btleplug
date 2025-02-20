use crate::peripheral::PeripheralRef;
use crate::peripheral::PeripheralState;
use crate::atoms;
// use crate::elixir_bridge::ElixirBridge;

// use rustler::types::pid::Pid;
use rustler::{Atom, Encoder, Env, ResourceArc, Term, LocalPid, Error as RustlerError};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use btleplug::platform::{Adapter, Manager};
use btleplug::api::{Central, Peripheral, ScanFilter, CentralEvent, Manager as _};
use tokio::sync::mpsc::Sender;
use tokio::spawn;
use tokio::runtime::Runtime;

pub struct CentralRef(pub(crate) Arc<Mutex<CentralManagerState>>);

pub struct CentralManagerState {
    // pub config: Config,
    pub pid: LocalPid,
    pub adapter: Adapter,
    pub manager: Manager,
    
    // apis: HashMap<String, Arc<API>>,
    // media_engines: HashMap<String, MediaEngine>,
    // peer_connections: HashMap<String, Sender<peer_connection::Msg>>,
    // registries: HashMap<String, Registry>,
    // local_static_sample_tracks: HashMap<String, Arc<TrackLocalStaticSample>>,
}

impl CentralManagerState {
    pub fn new(pid: LocalPid, manager: Manager, adapter: Adapter) -> Self {
        CentralManagerState {
            // config,
            pid,
            manager,
            adapter,
            
            // apis: HashMap::new(),
            // media_engines: HashMap::new(),
            // peer_connections: HashMap::new(),
            // registries: HashMap::new(),
            // local_static_sample_tracks: HashMap::new(),
        }
    }

    //***** API

    // pub(crate) fn add_api(&mut self, uuid: &str, api: API) -> &mut State {
    //     self.apis.insert(uuid.to_owned(), Arc::new(api));
    //     self
    // }

    // pub(crate) fn get_api(&self, uuid: Term) -> Option<&Arc<API>> {
    //     let id: &String = &uuid.decode().unwrap();
    //     self.apis.get(id)
    // }

    // //***** MediaEngine

    // pub(crate) fn add_media_engine(&mut self, uuid: &str, engine: MediaEngine) -> &mut State {
    //     self.media_engines.insert(uuid.to_owned(), engine);
    //     self
    // }

    // pub(crate) fn get_media_engine(&mut self, uuid: Term) -> Option<&MediaEngine> {
    //     // This could stand some error handling. The match implementation
    //     // fails with "creates a temporary which is freed while still in use."
    //     let id: &String = &uuid.decode().unwrap();
    //     self.media_engines.get(id)
    // }

    // pub(crate) fn get_media_engine_mut(&mut self, uuid: Term) -> Option<&mut MediaEngine> {
    //     let id: &String = &uuid.decode().unwrap();
    //     self.media_engines.get_mut(id)
    // }

    // pub(crate) fn remove_media_engine(&mut self, uuid: Term) -> Option<MediaEngine> {
    //     let id: &String = &uuid.decode().unwrap();
    //     self.media_engines.remove(id)
    // }

    // //***** RTCPeerConnection

    // pub(crate) fn add_peer_connection(
    //     &mut self,
    //     uuid: &str,
    //     pc: Sender<peer_connection::Msg>,
    // ) -> &mut State {
    //     self.peer_connections.insert(uuid.to_owned(), pc);
    //     self
    // }

    // pub(crate) fn get_peer_connection(&self, uuid: Term) -> Option<&Sender<peer_connection::Msg>> {
    //     let id: &String = &uuid.decode().unwrap();
    //     self.peer_connections.get(id)
    // }

    // pub(crate) fn remove_peer_connection(
    //     &mut self,
    //     uuid: Term,
    // ) -> Option<Sender<peer_connection::Msg>> {
    //     let id: &String = &uuid.decode().unwrap();
    //     self.peer_connections.remove(id)
    // }

    // //***** Registry

    // pub(crate) fn add_registry(&mut self, uuid: &str, registry: Registry) -> &mut State {
    //     self.registries.insert(uuid.to_owned(), registry);
    //     self
    // }

    // pub(crate) fn get_registry(&mut self, uuid: Term) -> Option<&Registry> {
    //     let id: &String = &uuid.decode().unwrap();
    //     self.registries.get(id)
    // }

    // pub(crate) fn remove_registry(&mut self, uuid: Term) -> Option<Registry> {
    //     let id: &String = &uuid.decode().unwrap();
    //     self.registries.remove(id)
    // }

    // //***** Track
    // pub(crate) fn add_track_local_static_sample(
    //     &mut self,
    //     uuid: &str,
    //     track: Arc<TrackLocalStaticSample>,
    // ) -> &mut State {
    //     self.local_static_sample_tracks
    //         .insert(uuid.to_owned(), track);
    //     self
    // }

    // pub(crate) fn get_track_local_static_sample(
    //     &mut self,
    //     uuid: &String,
    // ) -> Option<&Arc<TrackLocalStaticSample>> {
    //     self.local_static_sample_tracks.get(uuid)
    // }
}

pub fn load(env: Env) -> bool {
    rustler::resource!(CentralRef, env);
    true
}

/// Initialize the NIF, returning a reference to Elixir that can be
/// passed back into the NIF to retrieve or alter state.
// #[rustler::nif(name = "__init__")]
// fn init<'a>(env: Env<'a>, opts: Term<'a>) -> Term<'a> {
//     let config = match Config::parse(env, opts) {
//         Err(error) => return (atoms::error(), error).encode(env),
//         Ok(config) => config,
//     };

//     let state = State::new(config, env.pid());
//     let resource = ResourceArc::new(Ref(Arc::new(Mutex::new(state))));

//     (atoms::ok(), resource).encode(env)
// }




#[rustler::nif]
pub fn create_central(env: Env) -> Result<ResourceArc<CentralRef>, RustlerError> {
    println!("[Rust] Creating CentralManager...");


    spawn(async move {

    // let runtime = Arc::new(Runtime::new().map_err(|e| RustlerError::Term(Box::new(format!("Runtime error: {}", e))))?);

    // let manager = runtime.block_on(Manager::new())
    //     .map_err(|e| RustlerError::Term(Box::new(format!("Manager error: {}", e))))?;



        

    let manager = Manager::new().await;

    // let adapter = runtime.block_on(manager.adapters())
    let adapter = manager.adapters().await
        .map_err(|e| RustlerError::Term(Box::new(format!("Adapter error: {}", e))))?
        .into_iter()
        .next()
        .ok_or_else(|| RustlerError::Term(Box::new("No available adapter")))?;

    println!("[Rust] CentralManager created successfully.");

    let state = CentralManagerState::new(env.pid(), manager, adapter);
    let resource = ResourceArc::new(CentralRef(Arc::new(Mutex::new(state))));

    Ok(resource)
    });

    // atoms::ok(), 
    //    (resource).encode(env)
    // Ok(ResourceArc::new(CentralManagerResource {
    //     adapter: Arc::new(adapter),
    //     runtime,
    //     bridge: ElixirBridge::new(&env),
    // }))
}

#[rustler::nif]
pub fn find_peripheral(
    env: Env, 
    resource: ResourceArc<CentralRef>,
    uuid: String,
) -> Result<ResourceArc<PeripheralRef>, RustlerError> {
    println!("[Rust] Finding Peripheral: {}", uuid);

    let runtime = Arc::new(Runtime::new().map_err(|e| RustlerError::Term(Box::new(format!("Runtime error: {}", e))))?);

    let mut state = match resource.0.lock() {
        Err(_) => return Err(rustler::Error::Atom("Failed to lock peripheral state")),
        Ok(guard) => guard,
    };

    let peripherals = runtime.block_on(state.adapter.peripherals())
        .map_err(|e| RustlerError::Term(Box::new(format!("Manager error: {}", e))))?;

    println!("[Rust] Peripherals retrieved successfully.");

    for peripheral in peripherals {
        if peripheral.id().to_string() == uuid {
            let peripheral_state = PeripheralState::new(env.pid(), peripheral);
            let peripheral_resource = ResourceArc::new(PeripheralRef(Arc::new(Mutex::new(peripheral_state))));

            return Ok(peripheral_resource);
        }
    }

    Err(RustlerError::Term(Box::new("Peripheral not found")))
}


#[rustler::nif]
pub fn start_scan(env: Env, central: ResourceArc<CentralRef>) -> Result<ResourceArc<CentralRef>, RustlerError> {
    println!("[Rust] Starting BLE scan...");

    let adapter = central.0.adapter.clone();

    central.runtime.spawn(async move {
        if let Err(e) = adapter.start_scan(ScanFilter::default()).await {
            println!("[Rust] Failed to start scan: {:?}", e);
            return;
        }

        println!("[Rust] Scan started. Listening for events...");
        let mut events = adapter.events().await.expect("Failed to get event stream");

        while let Some(event) = events.next().await {
            println!("[Rust] BLE Event: {:?}", event);
            match event {
                CentralEvent::DeviceDiscovered(peripheral_id) => {
                    //bridge.send_message(("btleplug_device_discovered".encode(env), peripheral_id.to_string()));
                }
                CentralEvent::DeviceUpdated(peripheral_id) => {
                    //bridge.send_message(("btleplug_device_updated".encode(env), peripheral_id.to_string()));
                }
                CentralEvent::DeviceConnected(peripheral_id) => {
                    //bridge.send_message(("btleplug_device_connected".encode(env), peripheral_id.to_string()));
                }
                _ => {}
            }
        }
    });

    Ok(central)
}