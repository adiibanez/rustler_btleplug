defmodule RustlerBtleplug.NativeTest do
  use ExUnit.Case, async: false
  alias RustlerBtleplug.Native

  @ble_peripheral_name "Pressure"
  @ble_characteristic_uuid "61d20a90-71a1-11ea-ab12-0800200c9a66"

  @doc """
  Some Standard characteristic UUIDs
  heartrate: 00002a37-0000-1000-8000-00805f9b34fb
  batteryLevel: 00002a19-0000-1000-8000-00805f9b34fb
  deviceName: 00002a00-0000-1000-8000-00805f9b34fb
  """

  test "Test string" do
    test_string = "test string"
    assert Native.test_string(test_string) == test_string, "Expected #{inspect(test_string)}"
  end

  test "Test add" do
    assert Native.add(5, 5) == 10, "Expected 10"
  end

  test "Test map" do
    map = Native.get_map()
    assert is_map(map), "Expected map"
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

    Process.sleep(1000)

    assert_receive {:btleplug_scan_started, _msg}
    assert_receive {:btleplug_peripheral_discovered, _msg}

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
      {:btleplug_peripheral_discovered, _msg} -> :ok
    after
      500 -> flunk("Did not receive :btleplug_peripheral_discovered message")
    end

    receive do
      {:btleplug_scan_stopped, _msg} -> :ok
    after
      500 -> flunk("Did not receive :btleplug_scan_stopped message")
    end
  end

  test "BLE short scanning before timeout" do
    # {:ok, ble_resource} = Native.create_central()
    resource =
      Native.create_central()
      |> Native.start_scan(1000)

    assert is_reference(resource)

    # assert resource |> Native.is_scanning()
    assert_receive {:btleplug_scan_started, _msg}

    Process.sleep(500)
    messages = :erlang.process_info(self(), :messages)
    IO.inspect(messages, label: "messages")

    receive do
      {:btleplug_peripheral_discovered, _msg} ->
        :ok
        refute_receive {:btleplug_scan_stopped, _msg}
    after
      1000 -> flunk("Did not receive :btleplug_peripheral_discovered message")
    end


  end

  test "BLE fail to find unknown peripheral" do
    # {:ok, ble_resource} = Native.create_central()
    {status, msg} =
      Native.create_central()
      |> Native.start_scan()
      |> Native.find_peripheral("uuid_1234")

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
      {:btleplug_peripheral_discovered, peripheral_id} ->
        :ok

        peripheral_resource =
          central_resource
          |> Native.stop_scan()
          |> Native.find_peripheral(peripheral_id)

        assert is_reference(peripheral_resource)
    after
      2000 -> flunk("Did not receive :btleplug_peripheral_discovered message")
    end
  end

  test "BLE connect to peripheral" do
    timeout = 5000

    central_resource =
      Native.create_central()
      |> Native.start_scan()

    assert is_reference(central_resource)
    assert_receive {:btleplug_scan_started, _msg}, 1000

    Process.sleep(2000)

    receive do
      {:btleplug_peripheral_discovered, peripheral_id} ->
        :ok

        IO.puts("Found peripheral: #{peripheral_id}")

        peripheral_resource =
          central_resource
          |> Native.start_scan()
          |> Native.find_peripheral_by_name(@ble_peripheral_name)
          |> Native.connect()
          |> Native.subscribe(@ble_characteristic_uuid)

        assert is_reference(peripheral_resource)

        assert_receive {:btleplug_peripheral_updated, _msg},
                       timeout,
                       "No :btleplug_peripheral_updated received"

        assert_receive {:btleplug_peripheral_connected, _msg},
                       timeout,
                       "No :btleplug_peripheral_connected received"

        # assert_receive {:btleplug_services_advertisement, _msg}, timeout, "No :btleplug_services_advertisement received"
        # assert_receive {:btleplug_service_data_advertisement, _msg}, timeout, "No :btleplug_service_data_advertisement received"
        # assert_receive {:btleplug_peripheral_connected, _msg}, timeout, "No :btleplug_peripheral_connected received"
        assert_receive {:btleplug_characteristic_value_changed, _uuid, _value},
                       timeout,
                       "No :btleplug_characteristic_value_changed received"

        Process.sleep(1000)

        # messages = :erlang.process_info(self(), :messages)
        # IO.inspect(messages, label: "messages")
    after
      timeout * 2 -> flunk("Did not receive :btleplug_peripheral_discovered message")
    end
  end

  # @tag timeout: :infinity
  # test "Create GATT peripheral" do
  #   gatt_peripheral_resource =
  #     Native.create_gatt_peripheral("Movesense Rustler")

  #   assert is_reference(gatt_peripheral_resource)

  #   Process.sleep(1000)
  # end
end
