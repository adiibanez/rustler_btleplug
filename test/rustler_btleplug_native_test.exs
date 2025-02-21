defmodule RustlerBtleplug.NativeTest do
  use ExUnit.Case, async: false
  alias RustlerBtleplug.Native

  test "Test string" do
    #assert {:ok, resource} = Native.create_central()
    assert resource = Native.test_string("whatever")
    IO.puts(inspect(resource))
    #assert_equals is_atom(resource)
  end

  test "BLE manager initialization" do
    #assert {:ok, resource} = Native.create_central()
    assert resource = Native.create_central()
    IO.puts(inspect(resource))
    assert is_reference(resource)
  end

  test "BLE default scanning lifecycle" do
    # {:ok, ble_resource} = Native.create_central()
    resource = Native.create_central()
    |> Native.start_scan()

    assert_receive {:btleplug_scan_started, _msg}
    assert_receive {:btleplug_device_discovered, _msg}

    Process.sleep(1000)

    assert_receive {:btleplug_scan_stopped, _msg}

    assert is_reference(resource)
  end


  test "BLE short scanning lifecycle" do
    # {:ok, ble_resource} = Native.create_central()
    resource = Native.create_central()
    |> Native.start_scan(100)

    assert is_reference(resource)

    # assert resource |> Native.is_scanning()
    assert_receive {:btleplug_scan_started, _msg}
    assert_receive {:btleplug_device_discovered, _msg}

    Process.sleep(100)

    assert_receive {:btleplug_scan_stopped, _msg}

    #assert not resource |> Native.is_scanning()
  end


  test "BLE short scanning lifecycle before timeout" do
    # {:ok, ble_resource} = Native.create_central()
    resource = Native.create_central()
    |> Native.start_scan(500)
    assert is_reference(resource)

    # assert resource |> Native.is_scanning()
    assert_receive {:btleplug_scan_started, _msg}
    receive do
      {:btleplug_device_discovered, _msg} -> :ok
    after
      300 -> flunk("Did not receive :btleplug_device_discovered message")
    end
    Process.sleep(100)
    refute_receive {:btleplug_scan_stopped, _msg}
  end


  test "BLE fail to find unknown peripheral" do
    # {:ok, ble_resource} = Native.create_central()
    {status, msg} = Native.create_central()
    |> Native.start_scan()
    |> Native.find_peripheral("device_uuid_123")

    assert status == :error
    assert msg == "Peripheral not found"
  end

  test "BLE find known peripheral" do
    # {:ok, ble_resource} = Native.create_central()
    central_resource = Native.create_central()
    |> Native.start_scan()

    assert is_reference(central_resource)

    assert_receive {:btleplug_scan_started, _msg}
    assert_receive {:btleplug_device_discovered, peripheral_id}

    # Process.sleep(1000)

    #{status, peripheral_resource} = central_resource
    peripheral_resource = central_resource
    |> Native.stop_scan()
    |> Native.find_peripheral(peripheral_id)

    # Process.sleep(1000)

    #assert status == :ok
    assert is_reference(peripheral_resource)
  end


  test "BLE connect to peripheral lifecycle" do
    # {:ok, ble_resource} = Native.create_central()
    central_resource = Native.create_central()
    |> Native.start_scan()

    assert is_reference(central_resource)

    assert_receive {:btleplug_scan_started, _msg}
    assert_receive {:btleplug_device_discovered, peripheral_id}

    IO.puts("Found peripheral: #{peripheral_id}")

    Process.sleep(500)

    #{status, peripheral_resource} = central_resource
    peripheral_resource = central_resource
    |> Native.stop_scan()
    |> Native.find_peripheral(peripheral_id)
    |> Native.connect()
    |> Native.subscribe("test")

    # Process.sleep(1000)

    #assert status == :ok
    assert is_reference(peripheral_resource)
  end







  # test "Device connection" do
  #   # {:ok, ble_resource} = Native.create_central()
  #   ble_resource = Native.create_central()
  #   |> Native.start_scan()
  #   # |> Native.connect_peripheral()
  #   # {:ok, scanned_resource} = Native.start_scan(ble_resource)

  #   assert _connected_resource = Native.connect_peripheral(ble_resource, "device_uuid_123")
  # end
end
