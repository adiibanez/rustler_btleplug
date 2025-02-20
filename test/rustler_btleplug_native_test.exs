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

  # test "BLE scanning lifecycle" do
  #   # {:ok, ble_resource} = Native.create_central()
  #   ble_resource = Native.create_central()
  #   assert _scan_resource = Native.start_scan(ble_resource)
  # end

  # test "Device connection" do
  #   # {:ok, ble_resource} = Native.create_central()
  #   ble_resource = Native.create_central()
  #   |> Native.start_scan()
  #   # |> Native.connect_peripheral()
  #   # {:ok, scanned_resource} = Native.start_scan(ble_resource)

  #   assert _connected_resource = Native.connect_peripheral(ble_resource, "device_uuid_123")
  # end
end
