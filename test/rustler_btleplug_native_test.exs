defmodule RustlerBtleplug.NativeTest do
  use ExUnit.Case, async: false
  alias RustlerBtleplug.Native

  test "Test string" do
    # assert {:ok, resource} = Native.create_central()
    assert resource = Native.test_string("whatever")
    IO.puts(inspect(resource))
    # assert_equals is_atom(resource)
  end

  test "BLE manager initialization" do
    # assert {:ok, resource} = Native.create_central()
    assert resource = Native.create_central()
    IO.puts(inspect(resource))
    assert is_reference(resource)
  end

  test "BLE default scanning" do
    # {:ok, ble_resource} = Native.create_central()
    resource =
      Native.create_central()
      |> Native.start_scan()

    assert_receive {:btleplug_scan_started, _msg}
    assert_receive {:btleplug_device_discovered, _msg}

    Process.sleep(1000)

    assert_receive {:btleplug_scan_stopped, _msg}

    assert is_reference(resource)
  end

  test "BLE short scanning" do
    # {:ok, ble_resource} = Native.create_central()
    resource =
      Native.create_central()
      |> Native.start_scan(500)

    assert is_reference(resource)

    # assert resource |> Native.is_scanning()
    assert_receive {:btleplug_scan_started, _msg}

    receive do
      {:btleplug_device_discovered, _msg} -> :ok
    after
      500 -> flunk("Did not receive :btleplug_device_discovered message")
    end

    receive do
      {:btleplug_scan_stopped, _msg} -> :ok
    after
      500 -> flunk("Did not receive :btleplug_device_discovered message")
    end

    # assert not resource |> Native.is_scanning()
  end

  test "BLE short scanning before timeout" do
    # {:ok, ble_resource} = Native.create_central()
    resource =
      Native.create_central()
      |> Native.start_scan(500)

    assert is_reference(resource)

    # assert resource |> Native.is_scanning()
    assert_receive {:btleplug_scan_started, _msg}

    receive do
      {:btleplug_device_discovered, _msg} ->
        :ok

        Process.sleep(100)
        refute_receive {:btleplug_scan_stopped, _msg}
    after
      300 -> flunk("Did not receive :btleplug_device_discovered message")
    end
  end

  test "BLE fail to find unknown peripheral" do
    # {:ok, ble_resource} = Native.create_central()
    {status, msg} =
      Native.create_central()
      |> Native.start_scan()
      |> Native.find_peripheral("device_uuid_123")

    assert status == :error
    assert msg == "Peripheral not found"
  end

  test "BLE find known peripheral" do
    # {:ok, ble_resource} = Native.create_central()
    central_resource =
      Native.create_central()
      |> Native.start_scan()

    assert is_reference(central_resource)

    assert_receive {:btleplug_scan_started, _msg}

    receive do
      {:btleplug_device_discovered, peripheral_id} ->
        :ok

        peripheral_resource =
          central_resource
          |> Native.stop_scan()
          |> Native.find_peripheral(peripheral_id)

        # Process.sleep(1000)

        # assert status == :ok
        assert is_reference(peripheral_resource)
    after
      500 -> flunk("Did not receive :btleplug_device_discovered message")
    end
  end

  test "BLE connect to peripheral" do
    # {:ok, ble_resource} = Native.create_central()
    central_resource =
      Native.create_central()
      |> Native.start_scan()

    assert is_reference(central_resource)

    assert_receive {:btleplug_scan_started, _msg}

    receive do
      {:btleplug_device_discovered, peripheral_id} ->
        :ok

        IO.puts("Found peripheral: #{peripheral_id}")

        # Process.sleep(500)

        # {status, peripheral_resource} = central_resource
        peripheral_resource =
          central_resource
          |> Native.stop_scan()
          |> Native.find_peripheral(peripheral_id)
          |> Native.connect()
          |> Native.subscribe("test")

        # Process.sleep(1000)

        # assert status == :ok
        assert is_reference(peripheral_resource)
    after
      500 -> flunk("Did not receive :btleplug_device_discovered message")
    end

    # assert_receive {:btleplug_device_discovered, peripheral_id}
  end

  @tag timeout: :infinity
  test "Create GATT peripheral" do
    gatt_peripheral_resource =
      Native.create_gatt_peripheral("Movesense Rustler")

    assert is_reference(gatt_peripheral_resource)

    Process.sleep(100000)
  end
end
