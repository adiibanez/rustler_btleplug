use futures::{channel::mpsc::channel, prelude::*};
use rustler::{Atom, Encoder, Env, Error as RustlerError, LocalPid, ResourceArc, Term};
use std::{
    collections::HashSet,
    sync::{atomic, Arc, Mutex},
    thread,
    time::Duration,
};

use uuid::{Uuid, Builder, Bytes, Variant, Version};

use bluster::{
    gatt::{
        characteristic,
        characteristic::Characteristic,
        descriptor,
        descriptor::Descriptor,
        event::{Event, Response},
        service::Service,
    },
    Peripheral, SdpShortUuid,
};

use tokio::runtime::Runtime; // ✅ Add Tokio runtime

pub struct GattPeripheralRef(pub(crate) Arc<Mutex<GattPeripheralState>>);

pub struct GattPeripheralState {
    pub pid: LocalPid,
    pub peripheral_name: String,
    pub peripheral: Arc<Mutex<Peripheral>>,  // ✅ Use Arc<Mutex<>> for shared ownership
}

impl GattPeripheralState {
    pub fn new(pid: LocalPid, peripheral_name: String, peripheral: Arc<Mutex<Peripheral>>) -> Self {
        GattPeripheralState {
            pid,
            peripheral_name,
            peripheral,
        }
    }
}

impl Drop for GattPeripheralState {
    fn drop(&mut self) {
        println!("[Rust] GattPeripheralState destructor called.");
    }
}

const ADVERTISING_TIMEOUT: Duration = Duration::from_secs(60);

// fn short_uuid_to_uuid<T: Into<u32>>(short_uuid: T) -> SdpShortUuid {
//     SdpShortUuid::from_sdp_short_uuid(short_uuid)
// }

// fn short_uuid_to_uuid<T: Into<u32>>(short_uuid: T) -> impl SdpShortUuid {
//     SdpShortUuid::from_sdp_short_uuid(short_uuid)
// }

// fn short_uuid_to_uuid(short_uuid: u32) -> Uuid {
//     let mut builder = Builder::from_bytes([0; 16]); // Initialize with zero bytes

//     // Assign the short UUID to the appropriate bytes in the builder
//     builder.set_bytes(12, &(short_uuid as u32).to_be_bytes());

//     // Set the variant and version bits to match Bluetooth SIG-defined UUIDs
//     let msb = 0x1000;
//     let lsb = 0x8000_00805f9b34fb_u64;

//     let mut bytes: [u8; 16] = [0; 16];
//     bytes[0..8].copy_from_slice(&msb.to_be_bytes());
//     bytes[8..16].copy_from_slice(&lsb.to_be_bytes());
//     builder.set_variant(uuid::Variant::RFC4122);
//     builder.set_version(uuid::Version::Nil);

//     builder.build()
// }


fn short_uuid_to_uuid(short_uuid: u16) -> Uuid {
    // Bluetooth Base UUID: 00000000-0000-1000-8000-00805F9B34FB
    let mut bytes: Bytes = [
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00,
        0x80, 0x00, 0x00, 0x80, 0x5F, 0x9B, 0x34, 0xFB,
    ];

    // Insert the short UUID into bytes 12 and 13 (big-endian)
    bytes[12] = (short_uuid >> 8) as u8; // Most significant byte
    bytes[13] = (short_uuid & 0xFF) as u8; // Least significant byte

    let builder = Builder::from_bytes(bytes)
        .set_variant(Variant::RFC4122);
        //.set_version(Version::V4); // Or Version::Reserved if appropriate

    builder.into_uuid();
}

#[rustler::nif]
pub fn create_gatt_peripheral(
    env: Env,
    peripheral_name: String,
) -> Result<ResourceArc<GattPeripheralRef>, RustlerError> {
    let pid = env.pid(); // ✅ Capture `env.pid()` outside async
    let runtime = Runtime::new().map_err(|e| {
        RustlerError::Term(Box::new(format!("Runtime error: {}", e)))
    })?;

    // ✅ Ensure Peripheral is initialized successfully
    let peripheral = runtime.block_on(Peripheral::new()).map_err(|e| {
        RustlerError::Term(Box::new(format!("Failed to create peripheral: {}", e)))
    })?;

    let peripheral_arc = Arc::new(Mutex::new(peripheral)); // ✅ Wrap in Arc<Mutex<>>

    let state = Arc::new(Mutex::new(GattPeripheralState::new(
        pid,
        peripheral_name.clone(),
        peripheral_arc.clone(),
    )));

    let resource = ResourceArc::new(GattPeripheralRef(state.clone())); // ✅ Create `ResourceArc` before async

    runtime.spawn(async move {
        let (sender_characteristic, receiver_characteristic) = channel(1);
        let (sender_descriptor, receiver_descriptor) = channel(1);

        let mut characteristics: HashSet<Characteristic> = HashSet::new();
        characteristics.insert(Characteristic::new(
            short_uuid_to_uuid(0x2A3D_u16),
            characteristic::Properties::new(
                Some(characteristic::Read(characteristic::Secure::Insecure(
                    sender_characteristic.clone(),
                ))),
                Some(characteristic::Write::WithResponse(
                    characteristic::Secure::Insecure(sender_characteristic.clone()),
                )),
                Some(sender_characteristic),
                None,
            ),
            None,
            {
                let mut descriptors = HashSet::new();
                descriptors.insert(Descriptor::new(
                    short_uuid_to_uuid(0x2A3D as u16),
                    descriptor::Properties::new(
                        Some(descriptor::Read(descriptor::Secure::Insecure(
                            sender_descriptor.clone(),
                        ))),
                        Some(descriptor::Write(descriptor::Secure::Insecure(
                            sender_descriptor,
                        ))),
                    ),
                    None,
                ));
                descriptors
            },
        ));

        let characteristic_handler = async {
            let characteristic_value = Arc::new(Mutex::new(String::from("hi")));
            let notifying = Arc::new(atomic::AtomicBool::new(false));
            let mut rx = receiver_characteristic;
            while let Some(event) = rx.next().await {
                match event {
                    Event::ReadRequest(read_request) => {
                        let value = characteristic_value.lock().unwrap().clone();
                        read_request.response.send(Response::Success(value.clone().into())).unwrap();
                    }
                    Event::WriteRequest(write_request) => {
                        let new_value = String::from_utf8(write_request.data).unwrap();
                        *characteristic_value.lock().unwrap() = new_value;
                        write_request.response.send(Response::Success(vec![])).unwrap();
                    }
                    Event::NotifySubscribe(notify_subscribe) => {
                        notifying.store(true, atomic::Ordering::Relaxed);
                        thread::spawn(move || {
                            let mut count = 0;
                            loop {
                                if !notifying.load(atomic::Ordering::Relaxed) {
                                    break;
                                };
                                count += 1;
                                notify_subscribe
                                    .clone()
                                    .notification
                                    .try_send(format!("hi {}", count).into())
                                    .unwrap();
                                thread::sleep(Duration::from_secs(2));
                            }
                        });
                    }
                    Event::NotifyUnsubscribe => {
                        notifying.store(false, atomic::Ordering::Relaxed);
                    }
                };
            }
        };

        let descriptor_handler = async {
            let descriptor_value = Arc::new(Mutex::new(String::from("hi")));
            let mut rx = receiver_descriptor;
            while let Some(event) = rx.next().await {
                match event {
                    Event::ReadRequest(read_request) => {
                        let value = descriptor_value.lock().unwrap().clone();
                        read_request.response.send(Response::Success(value.clone().into())).unwrap();
                    }
                    Event::WriteRequest(write_request) => {
                        let new_value = String::from_utf8(write_request.data).unwrap();
                        *descriptor_value.lock().unwrap() = new_value;
                        write_request.response.send(Response::Success(vec![])).unwrap();
                    }
                    _ => panic!("Event not supported for Descriptors!"),
                };
            }
        };

        let service_uuid = short_uuid_to_uuid(0x1234_u16);

        peripheral.add_service(&Service::new(service_uuid, true, characteristics)).unwrap();

        let main_fut = async {
            while !peripheral.is_powered().await.unwrap() {}
            println!("Peripheral powered on");
            peripheral.register_gatt().await.unwrap();
            peripheral.start_advertising(&peripheral_name, &[service_uuid]).await.unwrap();
            println!("Peripheral started advertising");

            let ad_check = async { while !peripheral.is_advertising().await.unwrap() {} };
            let timeout = tokio::time::sleep(ADVERTISING_TIMEOUT);
            futures::join!(ad_check, timeout);
            
            peripheral.stop_advertising().await.unwrap();
            while peripheral.is_advertising().await.unwrap() {}
            println!("Peripheral stopped advertising");
        };

        futures::join!(characteristic_handler, descriptor_handler, main_fut);
    });

    Ok(resource) 
}
