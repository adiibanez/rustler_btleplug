use rustler::{Atom, Env, Error, Term};
use btleplug::api::{Central, Manager as _, Peripheral, ScanFilter, Characteristic, CharPropFlags};
use btleplug::platform::{Adapter, Manager};
use futures::stream::StreamExt;
use uuid::Uuid;

#[derive(Debug, Clone)]
enum DataType {
    Uint8,
    Uint16,
    Uint32,
    Int8,
    Int16,
    Int32,
    Float,
    Double,
    Boolean,
    Utf8String,
    RawData,
}

#[derive(Debug, Clone)]
struct CharacteristicValue {
    int_value: Option<i64>,
    double_value: Option<f64>,
    bool_value: Option<bool>,
    string_value: Option<String>,
    data_value: Option<Vec<u8>>,
}

impl CharacteristicValue {
    fn int(value: i64) -> Self {
        CharacteristicValue {
            int_value: Some(value),
            double_value: None,
            bool_value: None,
            string_value: None,
            data_value: None,
        }
    }

    fn double(value: f64) -> Self {
        CharacteristicValue {
            int_value: None,
            double_value: Some(value),
            bool_value: None,
            string_value: None,
            data_value: None,
        }
    }

    fn bool(value: bool) -> Self {
        CharacteristicValue {
            int_value: None,
            double_value: None,
            bool_value: Some(value),
            string_value: None,
            data_value: None,
        }
    }

    fn string(value: String) -> Self {
        CharacteristicValue {
            int_value: None,
            double_value: None,
            bool_value: None,
            string_value: Some(value),
            data_value: None,
        }
    }

    fn data(value: Vec<u8>) -> Self {
        CharacteristicValue {
            int_value: None,
            double_value: None,
            bool_value: None,
            string_value: None,
            data_value: Some(value),
        }
    }
}

use std::collections::HashMap;

use lazy_static::lazy_static;

lazy_static! {
    static ref UUID_MAP: HashMap<String, (String, DataType)> = {
        let mut m = HashMap::new();
        m.insert("00002A43-0000-1000-8000-00805F9B34FB".to_string(), ("alertCategoryId".to_string(), DataType::Uint8));
        m.insert("00002A06-0000-1000-8000-00805F9B34FB".to_string(), ("alertLevel".to_string(), DataType::Uint8));
        m.insert("00002A3F-0000-1000-8000-00805F9B34FB".to_string(), ("alertStatus".to_string(), DataType::Uint8));
        m.insert("00002AB3-0000-1000-8000-00805F9B34FB".to_string(), ("altitude".to_string(), DataType::Int32));
        m.insert("00002A58-0000-1000-8000-00805F9B34FB".to_string(), ("analog".to_string(), DataType::RawData));
        m.insert("00002A59-0000-1000-8000-00805F9B34FB".to_string(), ("analogOutput".to_string(), DataType::RawData));
        m.insert("00002A19-0000-1000-8000-00805F9B34FB".to_string(), ("batteryLevel".to_string(), DataType::Uint8));
        m.insert("00002A2B-0000-1000-8000-00805F9B34FB".to_string(), ("currentTime".to_string(), DataType::RawData));
        m.insert("00002A56-0000-1000-8000-00805F9B34FB".to_string(), ("digital".to_string(), DataType::RawData));
        m.insert("00002A57-0000-1000-8000-00805F9B34FB".to_string(), ("digitalOutput".to_string(), DataType::RawData));
        m.insert("00002A26-0000-1000-8000-00805F9B34FB".to_string(), ("firmwareRevisionString".to_string(), DataType::Utf8String));
        m.insert("00002A8A-0000-1000-8000-00805F9B34FB".to_string(), ("firstName".to_string(), DataType::Utf8String));
        m.insert("00002A00-0000-1000-8000-00805F9B34FB".to_string(), ("deviceName".to_string(), DataType::Utf8String));
        m.insert("00002A03-0000-1000-8000-00805F9B34FB".to_string(), ("reconnectionAddress".to_string(), DataType::RawData));
        m.insert("00002A05-0000-1000-8000-00805F9B34FB".to_string(), ("serviceChanged".to_string(), DataType::RawData));
        m.insert("00002A27-0000-1000-8000-00805F9B34FB".to_string(), ("hardwareRevisionString".to_string(), DataType::Utf8String));
        m.insert("00002A6F-0000-1000-8000-00805F9B34FB".to_string(), ("humidity".to_string(), DataType::Float));
        m.insert("00002A90-0000-1000-8000-00805F9B34FB".to_string(), ("lastName".to_string(), DataType::Utf8String));
        m.insert("00002AAE-0000-1000-8000-00805F9B34FB".to_string(), ("latitude".to_string(), DataType::Int32));
        m.insert("00002AAF-0000-1000-8000-00805F9B34FB".to_string(), ("longitude".to_string(), DataType::Int32));
        m.insert("00002A29-0000-1000-8000-00805F9B34FB".to_string(), ("manufacturerNameString".to_string(), DataType::Utf8String));
        m.insert("00002A21-0000-1000-8000-00805F9B34FB".to_string(), ("measurementInterval".to_string(), DataType::Int32));
        m.insert("00002A24-0000-1000-8000-00805F9B34FB".to_string(), ("modelNumberString".to_string(), DataType::Utf8String));
        m.insert("00002A6D-0000-1000-8000-00805F9B34FB".to_string(), ("pressure".to_string(), DataType::Float));
        m.insert("61D20A90-71A1-11EA-AB12-0800200C9A66".to_string(), ("pressure".to_string(), DataType::Float));
        m.insert("00002A78-0000-1000-8000-00805F9B34FB".to_string(), ("rainfall".to_string(), DataType::Int32));
        m.insert("00002A25-0000-1000-8000-00805F9B34FB".to_string(), ("serialNumberString".to_string(), DataType::Utf8String));
        m.insert("00002A3B-0000-1000-8000-00805F9B34FB".to_string(), ("serviceRequired".to_string(), DataType::Boolean));
        m.insert("00002A28-0000-1000-8000-00805F9B34FB".to_string(), ("softwareRevisionString".to_string(), DataType::Utf8String));
        m.insert("00002A3D-0000-1000-8000-00805F9B34FB".to_string(), ("string".to_string(), DataType::Utf8String));
        m.insert("00002A23-0000-1000-8000-00805F9B34FB".to_string(), ("systemId".to_string(), DataType::RawData));
        m.insert("00002A6E-0000-1000-8000-00805F9B34FB".to_string(), ("temperature".to_string(), DataType::Float));
        m.insert("00002A1F-0000-1000-8000-00805F9B34FB".to_string(), ("temperatureCelsius".to_string(), DataType::Float));
        m.insert("00002A20-0000-1000-8000-00805F9B34FB".to_string(), ("temperatureFahrenheit".to_string(), DataType::Float));
        m.insert("00002A15-0000-1000-8000-00805F9B34FB".to_string(), ("timeBroadcast".to_string(), DataType::RawData));
        m.insert("00002A37-0000-1000-8000-00805F9B34FB".to_string(), ("heartRateMeasurement".to_string(), DataType::RawData));
        m.insert("00002A5B-0000-1000-8000-00805F9B34FB".to_string(), ("cscMeasurement".to_string(), DataType::RawData));
        m.insert("00002902-0000-1000-8000-00805F9B34FB".to_string(), ("clientCharacteristicConfig".to_string(), DataType::RawData));
        m.insert("00001800-0000-1000-8000-00805F9B34FB".to_string(), ("genericAccess".to_string(), DataType::RawData));
        m.insert("00001811-0000-1000-8000-00805F9B34FB".to_string(), ("alertNotificationService".to_string(), DataType::RawData));
        m.insert("00001815-0000-1000-8000-00805F9B34FB".to_string(), ("automationIO".to_string(), DataType::RawData));
        m.insert("0000180F-0000-1000-8000-00805F9B34FB".to_string(), ("batteryService".to_string(), DataType::Uint8));
        m.insert("0000183B-0000-1000-8000-00805F9B34FB".to_string(), ("binarySensor".to_string(), DataType::RawData));
        m.insert("00001805-0000-1000-8000-00805F9B34FB".to_string(), ("currentTimeService".to_string(), DataType::RawData));
        m.insert("0000180A-0000-1000-8000-00805F9B34FB".to_string(), ("deviceInformation".to_string(), DataType::Utf8String));
        m.insert("0000183C-0000-1000-8000-00805F9B34FB".to_string(), ("emergencyConfiguration".to_string(), DataType::RawData));
        m.insert("0000181A-0000-1000-8000-00805F9B34FB".to_string(), ("environmentalSensing".to_string(), DataType::RawData));
        m.insert("00001801-0000-1000-8000-00805F9B34FB".to_string(), ("genericAttribute".to_string(), DataType::RawData));
        m.insert("00001812-0000-1000-8000-00805F9B34FB".to_string(), ("humanInterfaceDevice".to_string(), DataType::RawData));
        m.insert("00001802-0000-1000-8000-00805F9B34FB".to_string(), ("immediateAlert".to_string(), DataType::RawData));
        m.insert("00001821-0000-1000-8000-00805F9B34FB".to_string(), ("indoorPositioning".to_string(), DataType::RawData));
        m.insert("00001803-0000-1000-8000-00805F9B34FB".to_string(), ("linkLoss".to_string(), DataType::RawData));
        m.insert("00001819-0000-1000-8000-00805F9B34FB".to_string(), ("locationAndNavigation".to_string(), DataType::RawData));
        m.insert("00001825-0000-1000-8000-00805F9B34FB".to_string(), ("objectTransferService".to_string(), DataType::RawData));
        m.insert("00001824-0000-1000-8000-00805F9B34FB".to_string(), ("transportDiscovery".to_string(), DataType::RawData));
        m.insert("00001804-0000-1000-8000-00805F9B34FB".to_string(), ("txPower".to_string(), DataType::Int8));
        m.insert("0000181C-0000-1000-8000-00805F9B34FB".to_string(), ("userData".to_string(), DataType::RawData));
        m.insert("453B02B0-71A1-11EA-AB12-0800200C9A66".to_string(), ("pressureSensorService".to_string(), DataType::Int32));
        m.insert("00001523-1212-EFDE-1523-785FEABCD123".to_string(), ("nordicBlinkyService".to_string(), DataType::RawData));
        m.insert("0000180D-0000-1000-8000-00805F9B34FB".to_string(), ("mockHeartrate".to_string(), DataType::RawData));
        m.insert("00001524-1212-EFDE-1523-785FEABCD123".to_string(), ("buttonCharacteristic".to_string(), DataType::Uint8));
        m.insert("00001525-1212-EFDE-1523-785FEABCD123".to_string(), ("ledCharacteristic".to_string(), DataType::Uint8));
        m.insert("00002A38-0000-1000-8000-00805F9B34FB".to_string(), ("bodySensorLocation".to_string(), DataType::Uint8));
        m
    };
}

// Helper function to expand short UUIDs
fn expand_short_uuid(uuid_string: &str) -> String {
    if uuid_string.len() == 4 {
        format!("0000{}-0000-1000-8000-00805F9B34FB", uuid_string).to_uppercase()
    } else {
        uuid_string.to_uppercase()
    }
}

// Function to get the name for a given UUID
fn name_for_uuid(uuid_string: &str) -> String {
    let expanded_uuid_string = expand_short_uuid(uuid_string);
    match UUID_MAP.get(&expanded_uuid_string) {
        Some((name, _)) => name.clone(),
        None => format!("unknown {} {}", "uuid", uuid_string)
    }
}

// Function to get the data type for a given UUID
fn data_type_for_uuid(uuid_string: &str) -> Option<DataType> {
    let expanded_uuid_string = expand_short_uuid(uuid_string);
    match UUID_MAP.get(&expanded_uuid_string) {
        Some((_, data_type)) => Some(data_type.clone()),
        None => None,
    }
}

// Function to decode heart rate data
fn decode_heart_rate(data: &[u8]) -> Option<i64> {
    if data.is_empty() {
        println!("Data is too short to read heartrate (no flags)");
        return None;
    }

    let flags = data[0];
    let heart_rate_format_bit = flags & 0x01;

    if heart_rate_format_bit == 0 { // uint8 format
        if data.len() < 2 {
            println!("Data is too short for uint8 heartrate (flags present, but no value)");
            return None;
        }
        let heart_rate_value = data[1]; // Extract uint8
        println!("Decoded uint8 heartrate: {}", heart_rate_value);
        Some(i64::from(heart_rate_value))
    } else { // uint16 format
        if data.len() < 3 {
            println!("Data is too short for uint16 heartrate (flags set, but no 16-bit value)");
            return None;
        }
        if data.len() >= 3 {
            let heart_rate_value = u16::from_le_bytes([data[1], data[2]]);
            println!("Decoded uint16 heartrate: {}", heart_rate_value);
            Some(i64::from(heart_rate_value))
        } else {
            println!("Failed to extract uint16 value");
            None
        }
    }
}

// Function to decode value based on UUID and data
fn decode_value(uuid_string: &str, data: &[u8]) -> Option<CharacteristicValue> {
    if expand_short_uuid(uuid_string) == expand_short_uuid(&"2A37".to_string()) {
        // Decode as heart rate
        if let Some(heart_rate) = decode_heart_rate(data) {
            return Some(CharacteristicValue::int(heart_rate));
        } else {
            return None;
        }
    }

    let data_type = match data_type_for_uuid(uuid_string) {
        Some(data_type) => data_type,
        None => {
            println!("Unknown data type for UUID: {}", uuid_string);
            return None;
        }
    };

    match data_type {
        DataType::Uint8 => {
            if data.len() == 1 {
                Some(CharacteristicValue::int(i64::from(data[0])))
            } else { None }
        }
        DataType::Uint16 => {
            if data.len() == 2 {
                let value = u16::from_le_bytes([data[0], data[1]]);
                Some(CharacteristicValue::int(i64::from(value)))
            } else { None }
        }
        DataType::Uint32 => {
            if data.len() == 4 {
                let value = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
                Some(CharacteristicValue::int(i64::from(value)))
            } else { None }
        }
        DataType::Int8 => {
            if data.len() == 1 {
                Some(CharacteristicValue::int(i64::from(data[0] as i8)))
            } else { None }
        }
        DataType::Int16 =>  {
            if data.len() == 2 {
                let value = i16::from_le_bytes([data[0], data[1]]);
                Some(CharacteristicValue::int(i64::from(value)))
            } else { None }
        }
        DataType::Int32 => {
            if data.len() == 4 {
                let value = i32::from_le_bytes([data[0], data[1], data[2], data[3]]);
                Some(CharacteristicValue::int(i64::from(value)))
            } else { None }
        }
        DataType::Float => {
            if data.len() == 4 {
                let value = f32::from_le_bytes([data[0], data[1], data[2], data[3]]);
                Some(CharacteristicValue::double(f64::from(value)))
            } else { None }
        }
        DataType::Double => {
            if data.len() == 8 {
                let value = f64::from_le_bytes([data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7]]);
                Some(CharacteristicValue::double(value))
            } else { None }
        }
        DataType::Boolean => {
            if data.len() == 1 {
                Some(CharacteristicValue::bool(data[0] != 0))
            } else { None }
        }
        DataType::Utf8String => {
            if let Ok(s) = String::from_utf8(data.to_vec()) {
                Some(CharacteristicValue::string(s))
            } else { None }
        }
        DataType::RawData => {
            Some(CharacteristicValue::data(data.to_vec()))
        }
    }
}

// #[rustler::nif]
// fn decode_value_nif(uuid_str: String, data: Vec<u8>) -> Result<Option<CharacteristicValue>, Error> {
//     let decoded_value = decode_value(&uuid_str, &data);
//     Ok(decoded_value)
// }

// rustler::init!("Elixir.RustlerBtleplug.Native", [decode_value_nif]);