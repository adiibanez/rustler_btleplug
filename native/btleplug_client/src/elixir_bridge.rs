// //use crate::btle_peripheral::BtlePeripheral;
// use rustler::env::OwnedEnv;
// use rustler::types::LocalPid;
// use rustler::{Atom, Encoder, Env, Term};
// use std::sync::{Arc, Mutex};

// #[derive(Debug)]
// pub struct ElixirBridge {
//     pid: LocalPid,
//     env: Mutex<OwnedEnv>,
// }

// impl ElixirBridge {
//     pub fn new(env: &Env) -> Arc<Self> {
//         Arc::new(Self {
//             pid: env.pid(),
//             env: Mutex::new(OwnedEnv::new()),
//         })
//     }

//     pub fn send_message(&self, payload: (Atom, String)) {
//         if let Ok(mut env) = self.env.lock() {
//             println!("BRIDGE: send_message");
//             env.send_and_clear(&self.pid, |env| payload.encode(env));
//         } else {
//             eprintln!("BRIDGE: Failed to acquire env lock: possible thread panic");
//         }
//     }
// }
