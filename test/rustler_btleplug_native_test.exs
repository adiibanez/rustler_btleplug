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

  test "BLE scanning lifecycle" do
    # {:ok, ble_resource} = Native.create_central()
    resource = Native.create_central()
    |> Native.start_scan()

    Process.sleep(1000)

    assert is_reference(resource)
  end


  test "BLE find unknown peripheral lifecycle" do
    # {:ok, ble_resource} = Native.create_central()
    {status, msg} = Native.create_central()
    |> Native.start_scan()
    |> Native.find_peripheral("device_uuid_123")

    #Process.sleep(1000)

    assert status == :error
    assert msg == "Peripheral not found"
  end

  test "BLE find known peripheral lifecycle" do
    # {:ok, ble_resource} = Native.create_central()
    central_resource = Native.create_central()
    |> Native.start_scan()

    assert is_reference(central_resource)


    Process.sleep(1000)

    #{status, peripheral_resource} = central_resource
    peripheral_resource = central_resource
    |> Native.stop_scan()
    |> Native.find_peripheral("b8fb0ba6-ce1d-5200-e513-a1ccb6620d43")

    # Process.sleep(1000)

    #assert status == :ok
    assert is_reference(peripheral_resource)
  end


  test "BLE connect to peripheral lifecycle" do
    # {:ok, ble_resource} = Native.create_central()
    central_resource = Native.create_central()
    |> Native.start_scan()

    assert is_reference(central_resource)

    Process.sleep(500)

    #{status, peripheral_resource} = central_resource
    peripheral_resource = central_resource
    |> Native.stop_scan()
    |> Native.find_peripheral("b8fb0ba6-ce1d-5200-e513-a1ccb6620d43")
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
