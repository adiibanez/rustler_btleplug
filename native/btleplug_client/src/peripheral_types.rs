//! Type definitions for BLE Peripheral (server/advertiser) mode.
//!
//! These types are used to define services and characteristics that the
//! peripheral will expose to connected centrals.

use rustler::{Decoder, Encoder, Env, NifResult, NifStruct, Term};
use std::collections::HashMap;

/// Characteristic properties flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharacteristicProperties {
    pub read: bool,
    pub write: bool,
    pub write_without_response: bool,
    pub notify: bool,
    pub indicate: bool,
}

impl Default for CharacteristicProperties {
    fn default() -> Self {
        Self {
            read: false,
            write: false,
            write_without_response: false,
            notify: false,
            indicate: false,
        }
    }
}

impl<'a> Decoder<'a> for CharacteristicProperties {
    fn decode(term: Term<'a>) -> NifResult<Self> {
        let map: HashMap<String, bool> = term.decode()?;
        Ok(Self {
            read: *map.get("read").unwrap_or(&false),
            write: *map.get("write").unwrap_or(&false),
            write_without_response: *map.get("write_without_response").unwrap_or(&false),
            notify: *map.get("notify").unwrap_or(&false),
            indicate: *map.get("indicate").unwrap_or(&false),
        })
    }
}

impl Encoder for CharacteristicProperties {
    fn encode<'a>(&self, env: Env<'a>) -> Term<'a> {
        let mut map = HashMap::new();
        map.insert("read".to_string(), self.read);
        map.insert("write".to_string(), self.write);
        map.insert("write_without_response".to_string(), self.write_without_response);
        map.insert("notify".to_string(), self.notify);
        map.insert("indicate".to_string(), self.indicate);
        map.encode(env)
    }
}

/// A BLE characteristic definition for peripheral mode
#[derive(Debug, Clone)]
pub struct CharacteristicDefinition {
    pub uuid: String,
    pub properties: CharacteristicProperties,
    pub value: Option<Vec<u8>>,
}

impl<'a> Decoder<'a> for CharacteristicDefinition {
    fn decode(term: Term<'a>) -> NifResult<Self> {
        let map: HashMap<String, Term<'a>> = term.decode()?;

        let uuid: String = map
            .get("uuid")
            .ok_or(rustler::Error::BadArg)?
            .decode()?;

        let properties: CharacteristicProperties = map
            .get("properties")
            .map(|t| t.decode())
            .transpose()?
            .unwrap_or_default();

        let value: Option<Vec<u8>> = map
            .get("value")
            .map(|t| t.decode())
            .transpose()?;

        Ok(Self {
            uuid,
            properties,
            value,
        })
    }
}

impl Encoder for CharacteristicDefinition {
    fn encode<'a>(&self, env: Env<'a>) -> Term<'a> {
        let mut map: HashMap<&str, Term<'a>> = HashMap::new();
        map.insert("uuid", self.uuid.encode(env));
        map.insert("properties", self.properties.encode(env));
        if let Some(ref value) = self.value {
            map.insert("value", value.encode(env));
        }
        map.encode(env)
    }
}

/// A BLE service definition for peripheral mode
#[derive(Debug, Clone)]
pub struct ServiceDefinition {
    pub uuid: String,
    pub primary: bool,
    pub characteristics: Vec<CharacteristicDefinition>,
}

impl<'a> Decoder<'a> for ServiceDefinition {
    fn decode(term: Term<'a>) -> NifResult<Self> {
        let map: HashMap<String, Term<'a>> = term.decode()?;

        let uuid: String = map
            .get("uuid")
            .ok_or(rustler::Error::BadArg)?
            .decode()?;

        let primary: bool = map
            .get("primary")
            .map(|t| t.decode())
            .transpose()?
            .unwrap_or(true);

        let characteristics: Vec<CharacteristicDefinition> = map
            .get("characteristics")
            .ok_or(rustler::Error::BadArg)?
            .decode()?;

        Ok(Self {
            uuid,
            primary,
            characteristics,
        })
    }
}

impl Encoder for ServiceDefinition {
    fn encode<'a>(&self, env: Env<'a>) -> Term<'a> {
        let mut map: HashMap<&str, Term<'a>> = HashMap::new();
        map.insert("uuid", self.uuid.encode(env));
        map.insert("primary", self.primary.encode(env));
        map.insert("characteristics", self.characteristics.encode(env));
        map.encode(env)
    }
}

/// Read request information sent to Elixir
#[derive(Debug, Clone, NifStruct)]
#[module = "RustlerBtleplug.Peripheral.ReadRequest"]
pub struct ReadRequest {
    pub characteristic_uuid: String,
    pub service_uuid: String,
    pub offset: u64,
}

/// Write request information sent to Elixir
#[derive(Debug, Clone, NifStruct)]
#[module = "RustlerBtleplug.Peripheral.WriteRequest"]
pub struct WriteRequest {
    pub characteristic_uuid: String,
    pub service_uuid: String,
    pub value: Vec<u8>,
    pub offset: u64,
}

/// Subscription update information sent to Elixir
#[derive(Debug, Clone, NifStruct)]
#[module = "RustlerBtleplug.Peripheral.SubscriptionUpdate"]
pub struct SubscriptionUpdate {
    pub characteristic_uuid: String,
    pub service_uuid: String,
    pub subscribed: bool,
}
