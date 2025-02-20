// use crate::elixir_bridge::ElixirBridge;
use crate::atoms;


// use rustler::types::pid::Pid;
use rustler::{Atom, Encoder, Env, ResourceArc, Term, LocalPid, Error as RustlerError};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::Sender;

use btleplug::platform::{Adapter, Manager};
use btleplug::api::{Central, ScanFilter, Peripheral, CentralEvent, Manager as _};
use tokio::runtime::Runtime;
use tokio::spawn;

pub struct PeripheralRef(pub(crate) Arc<Mutex<PeripheralState>>);

pub struct PeripheralState {
    // pub config: Config,
    pub pid: LocalPid,
    pub peripheral: Peripheral,
    
    // apis: HashMap<String, Arc<API>>,
    // media_engines: HashMap<String, MediaEngine>,
    // peer_connections: HashMap<String, Sender<peer_connection::Msg>>,
    // registries: HashMap<String, Registry>,
    // local_static_sample_tracks: HashMap<String, Arc<TrackLocalStaticSample>>,
}

impl PeripheralState {
    pub fn new(pid: LocalPid, peripheral: Peripheral) -> Self {
        PeripheralState {
            // config,
            pid,
            peripheral,
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

impl Drop for PeripheralState {
    fn drop(&mut self) {
        println!("[Rust] PeripheralResource destructor called.");
    }
}

pub fn load(env: Env) -> bool {
    rustler::resource!(PeripheralRef, env);
    true
}


#[rustler::nif]
pub fn connect(
    env: Env,
    peripheral: ResourceArc<PeripheralRef>
) -> Result<ResourceArc<PeripheralRef>, RustlerError> {
    println!("[Rust] Connecting to Peripheral: {:?}", peripheral.peripheral.id());

    let peripheral_clone = peripheral.peripheral.clone();

    spawn(async move {
        if let Err(e) = peripheral_clone.connect().await {
            println!("[Rust] Failed to connect: {:?}", e);
        } else {
            println!("[Rust] Successfully connected to peripheral.");
        }
    });

    (atoms::ok(), peripheral).encode(env)
    
}

#[rustler::nif]
pub fn subscribe(
    env: Env,
    peripheral: ResourceArc<PeripheralRef>,
    characteristic_uuid: String,
) -> ResourceArc<PeripheralRef> {
    let peripheral_clone = peripheral.0.lock(); //peripheral.peripheral.clone();
    // let bridge = ElixirBridge::new(env);

    spawn(async move {
        let characteristics = peripheral_clone.characteristics().await.expect("Failed to get characteristics");
        let characteristic = characteristics.iter()
            .find(|c| c.uuid.to_string() == characteristic_uuid)
            .cloned();

        if let Some(char) = characteristic {
            println!("[Rust] Subscribing to characteristic: {:?}", char.uuid);

            if let Err(e) = peripheral_clone.subscribe(&char).await {
                println!("[Rust] Failed to subscribe: {:?}", e);
                return;
            }

            let mut notifications = peripheral_clone.notifications().await.unwrap();
            while let Some(notification) = notifications.next().await {
                //env.send_message(("btleplug_notification".encode(env), notification.value));
                env.send_and_clear(&peripheral.pid, |env| ("btleplug_notification".encode(env), notification.value).encode(env));
            }
        } else {
            println!("[Rust] Characteristic not found: {}", characteristic_uuid);
        }
    });

    (atoms::ok(), peripheral).encode(env)
}