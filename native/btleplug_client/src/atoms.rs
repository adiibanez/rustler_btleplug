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

    btleplug_peripheral_discovered,
    btleplug_peripheral_discovery_error,
    btleplug_peripheral_connected,
    btleplug_peripheral_updated,
    btleplug_peripheral_disconnected,
    btleplug_peripheral_not_found,

    btleplug_peripheral_service_discovery_error,

    btleplug_manufacturer_data_advertisement,
    btleplug_service_data_advertisement,
    btleplug_services_advertisement,

    btleplug_characteristic_value_changed,

    // Peripheral (server) mode atoms
    btleplug_peripheral_created,
    btleplug_peripheral_advertising_started,
    btleplug_peripheral_advertising_stopped,
    btleplug_peripheral_service_added,
    btleplug_peripheral_read_request,
    btleplug_peripheral_write_request,
    btleplug_peripheral_subscription_update,
    btleplug_peripheral_central_connected,
    btleplug_peripheral_central_disconnected,
}
