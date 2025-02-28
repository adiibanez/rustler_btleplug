defmodule RustlerBtleplug.NativeBlestateTest do
  @moduledoc false
  use ExUnit.Case, async: true
  alias RustlerBtleplug.Native

  @ble_peripheral_name "Pressure"
  @ble_characteristic_uuid "61d20a90-71a1-11ea-ab12-0800200c9a66"

  @doc """
  Some Standard characteristic UUIDs
  heartrate: 00002a37-0000-1000-8000-00805f9b34fb
  batteryLevel: 00002a19-0000-1000-8000-00805f9b34fb
  deviceName: 00002a00-0000-1000-8000-00805f9b34fb
  """

  test "BLE manager initialization" do
    # assert {:ok, resource} = Native.create_central()
    assert resource = Native.create_central()
    IO.puts(inspect(resource))
    assert is_reference(resource)
  end

  test "BLE get state map after scan" do
    # {:ok, ble_resource} = Native.create_central()
    resource =
      Native.create_central()
      |> Native.start_scan()

    Process.sleep(1000)

    assert is_reference(resource)

    assert_receive {:btleplug_scan_started, _msg}
    assert_receive {:btleplug_peripheral_discovered, _msg, _props}

    assert_receive {:btleplug_scan_stopped, _msg}


    state_map = Native.get_adapter_state_map(resource)

    assert is_map(state_map)

    IO.inspect(state_map)
  end

  test "BLE get state map after connect" do
    # {:ok, ble_resource} = Native.create_central()
    central_resource =
      Native.create_central()
      |> Native.start_scan()

    assert is_reference(central_resource)

    assert_receive {:btleplug_scan_started, _msg}
    Process.sleep(2000)
    assert_receive {:btleplug_peripheral_discovered, _msg, _props}

    peripheral_resource = Native.find_peripheral_by_name(central_resource, @ble_peripheral_name)
    |> Native.connect()
    |> Native.subscribe(@ble_characteristic_uuid)

    Process.sleep(1000)

    assert is_reference(peripheral_resource)
    state_map = Native.get_adapter_state_map(central_resource)

    assert is_map(state_map)
    IO.inspect(state_map)

    assert_receive {:btleplug_scan_stopped, _msg}
  end

end
