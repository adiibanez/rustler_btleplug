rustler::atoms! {
    ok,
    error,

    // errors
    lock_fail,
    not_found,
    offer_error,

    candidate_error,

    btleplug_error,
    btleplug_got_central,
    btleplug_no_adapters_found,
    btleplug_adapter_status_update,
    btleplug_scan_started,
    btleplug_scan_stopped,

    btleplug_device_discovered,
    btleplug_device_discovery_error,
    btleplug_device_connected,
    btleplug_device_updated,
    btleplug_device_disconnected,
    btleplug_device_not_found,

    btleplug_device_service_discovery_error,

    btleplug_manufacturer_data_advertisement,
    btleplug_service_data_advertisement,
    btleplug_services_advertisement,

    btleplug_characteristic_value_changed,
}

//pub(crate)
// use self::*;
