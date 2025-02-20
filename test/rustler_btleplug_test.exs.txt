defmodule RustlerBtleplugTest do
  use ExUnit.Case, async: true
  # doctest RustlerBtleplug
  alias RustlerBtleplug.Client

  @disabled true

  setup_all do
    case start_supervised({Client, [name: :btleplug_client]}) do
      {:ok, client} -> IO.puts("RustlerBtleplug started")
      case client.init() do
        {:ok, client} -> IO.puts("RustlerBtleplug initialized #{inspect(client)}")
      end
      {:error, error} -> IO.puts("RustlerBtleplug failed to start: #{inspect(error)}")
    end

   end

  # setup do
  #   # Ensure BTLE_MANAGER is initialized before each test
  #   :ok = RustlerBtleplug.Client.init(%{})
  #   {:ok, btleplug_messages: []} # Initialize messages
  # end

  test "add 2 numbers" do
    case RustlerBtleplug.Client.add(5, 5) do
      {:ok, number} ->
        IO.puts("ok: #{number}")
        assert number == 25

      result ->
        IO.puts("not sure: #{inspect(result)}")
        assert true
    end

    # IO.puts("Number: #{number}")
  end

  test "get map " do
    case RustlerBtleplug.Client.get_map() do
      {:ok, map} ->
        IO.puts("ok: #{inspect(map)}")
        assert is_map(map)

      result ->
        IO.puts("not sure: #{inspect(result)}")
    end

    # IO.puts("Number: #{number}")
  end

  test "scan" do
    case RustlerBtleplug.Client.scan() do
      # {:ok, map} ->
      #   IO.puts("ok 3: #{inspect(map)}")
      #   assert is_map(map)
      result ->
        IO.puts("not sure: #{inspect(result)}")
    end

    Process.sleep(1000)

    messages = :erlang.process_info(self(), :messages)
    IO.inspect(messages, label: "messages")

    assert_receive {:btleplug_got_central, _}
    # no_adapters_found,
    assert_receive {:btleplug_device_discovered, _}
  end

  test "connect" do
    # 1. Initialize (ensure NIF is loaded)
    # 2. Start the scan
    case RustlerBtleplug.Client.scan() do
      result ->
        # 3. Wait for device discovery messages
        # Increased timeout
        assert_receive {:btleplug_got_central, _}, 5000
        # Capture the device ID
        assert_receive {:btleplug_device_discovered, device_id}, 5000

        IO.puts("Discovered device ID: #{device_id}")

        # 4. Connect to the discovered device
        :ok = RustlerBtleplug.Client.connect(device_id)

        # 5. Assert that connection was successful
        assert_receive {:btleplug_device_connected, ^device_id}, 5000

        # Optionally, assert other messages (manufacturer data, etc.)
        assert_receive {:btleplug_manufacturer_data_advertisement, _}, 100
        assert_receive {:btleplug_services_advertisement, _}, 100
        assert_receive {:btleplug_service_data_advertisement, _}, 100

        # 6. Check messages (for debugging)
        messages = :erlang.process_info(self(), :messages)
        IO.inspect(messages, label: "Messages in mailbox")
    end
  end

  test "connect2" do
    case RustlerBtleplug.Client.scan() do
      result ->
        IO.puts("Scan: #{inspect(result)}")

        Process.sleep(1000)

        case RustlerBtleplug.Client.connect("ee2710bc-ffe1-c27b-8156-f11e0823d1b6") do
          result ->
            IO.puts("not sure: #{inspect(result)}")
        end
    end

    Process.sleep(1000)

    messages = :erlang.process_info(self(), :messages)
    IO.inspect(messages, label: "messages")

    assert_receive {:btleplug_got_central, _}
    assert_receive {:btleplug_manufacturer_data_advertisement, _}
    assert_receive {:btleplug_services_advertisement, _}
    assert_receive {:btleplug_service_data_advertisement, _}
    assert_receive {:btleplug_device_discovered, _}
    # assert_receive {:btleplug_device_connected, _}
    # no_adapters_found,
    # assert_receive {:btleplug_device_discovered, _}
  end
end
