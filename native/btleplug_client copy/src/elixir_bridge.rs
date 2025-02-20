use crate::btle_peripheral::BtlePeripheral;
use rustler::env::OwnedEnv;
use rustler::types::LocalPid;
use rustler::{Atom, Encoder, Env, Term};
use std::sync::{Arc, Mutex};

pub struct ElixirBridge {
    pid: LocalPid,
    env: Mutex<OwnedEnv>,
}

impl ElixirBridge {
    pub fn new(env: &Env) -> Arc<Self> {
        Arc::new(Self {
            pid: env.pid(),
            env: Mutex::new(OwnedEnv::new()),
        })
    }

    pub fn send_message(&self, payload: (Atom, String)) {
        if let Ok(mut env) = self.env.lock() {
            println!("BRIDGE: send_message");
            env.send_and_clear(&self.pid, |env| payload.encode(env));
        } else {
            eprintln!("BRIDGE: Failed to acquire env lock: possible thread panic");
        }
    }

    // pub fn send_message(&self, message: &str) {
    //     let mut env = self.env.lock().unwrap();

    //     println!("BRIDGE: send_message");
    //     env.send_and_clear(&self.pid, |env| ("message", message.clone()).encode(env))
    //         .unwrap();
    // }

    pub fn send_device_discovered(&self, device: &BtlePeripheral) {
        let mut env = self.env.lock().unwrap();

        println!("BRIDGE: send_device_discovered");
        env.send_and_clear(&self.pid, |env| {
            ("btleplug_device_discovered", device.id.clone()).encode(env)
        })
        .unwrap();
    }

    pub fn send_device_connected(&self, device: &BtlePeripheral) {
        let mut env = self.env.lock().unwrap();

        println!("BRIDGE: send_device_connected");
        env.send_and_clear(&self.pid, |env| {
            ("btleplug_device_connected", device.id.clone()).encode(env)
        })
        .unwrap();
    }
}
