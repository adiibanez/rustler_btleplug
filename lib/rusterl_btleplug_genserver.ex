defmodule RustlerBtleplug.Genserver do
  @name :ble_genserver

  use GenServer
  require Logger

  def start_link(_opts) do
    GenServer.start_link(__MODULE__, %{}, name: @name)
  end

  def child_spec(init_arg) do
    Supervisor.child_spec(
      %{
        id: init_arg,
        start: {__MODULE__, :start_link, [init_arg]}
      },
      []
    )
  end

  def init(opts) do
    Process.flag(:trap_exit, true)
    IO.puts("#{__MODULE__} init #{inspect(opts)}")

    {:ok,
     %{
       central: nil,
       peripheral: nil,
       messages: []
     }}
  end

  def handle_info({:btleplug_peripheral_discovered, uuid}, state) do
    Logger.debug("NIF Peripheral Discovered: #{uuid}")
    new_state = update_state_with_message(state, {:btleplug_peripheral_discovered, uuid})

    {:noreply, new_state}
  end

  def handle_info({:btleplug_peripheral_connected, uuid}, state) do
    Logger.debug("NIF Peripheral Connected: #{uuid}")
    new_state = update_state_with_message(state, {:btleplug_peripheral_connected, uuid})
    {:noreply, new_state}
  end

  def handle_info({:btleplug_peripheral_disconnected, uuid}, state) do
    Logger.debug("NIF Peripheral Disconnected: #{uuid}")
    new_state = update_state_with_message(state, {:btleplug_peripheral_disconnected, uuid})
    {:noreply, new_state}
  end

  def handle_info({:btleplug_characteristic_value_changed, uuid, value}, state) do
    Logger.debug("NIF Characteristic Value Changed for #{uuid}: #{value}")

    new_state =
      update_state_with_message(state, {:btleplug_characteristic_value_changed, uuid, value})

    {:noreply, new_state}
  end

  def handle_info(msg, state) do
    Logger.debug("NIF msg received : #{inspect(msg)}")
    new_state = update_state_with_message(state, msg)
    {:noreply, new_state}
  end

  def create_central() do
    GenServer.call(@name, {:create_central})
  end

  def start_scan() do
    Logger.debug("client :start_scan")
    GenServer.cast(@name, {:start_scan})
  end

  def stop_scan() do
    Logger.debug("client :stop_scan")
    GenServer.cast(@name, {:stop_scan})
  end

  def find_peripheral_by_name(device_name) do
    Logger.debug("client :find_peripheral_by_name #{device_name}")
    GenServer.call(@name, {:find_peripheral_by_name, device_name})
  end

  def connect() do
    Logger.debug("client :connect")
    GenServer.call(@name, {:connect})
  end

  def subscribe(uuid) do
    Logger.debug("client :subscribe characteristic uuid: #{uuid}")
    GenServer.call(@name, {:subscribe, uuid})
  end

  def get_messages() do
    GenServer.call(@name, {:get_messages})
  end

  def handle_cast({:set_central, central_ref}, state) do
    Logger.debug("handle_cast :set_central #{inspect(central_ref)}")

    new_state = %{state | central: central_ref}

    Logger.debug("handle_cast :set_central new_state: #{inspect(new_state)}")
    {:noreply, new_state}
  end

  def handle_cast({:start_scan}, state) do
    Logger.debug("handle_cast :start_scan #{inspect(state)}")

    case state.central do
      nil ->
        Logger.debug("No central reference to start scan.")
        {:noreply, state}

      central_ref ->
        # Call NIF to stop the scan using the central reference
        case RustlerBtleplug.Native.start_scan(central_ref) do
          {:error, reason} ->
            Logger.debug("Failed to start scan: #{reason}")
            {:noreply, state}

          _central_ref ->
            Logger.debug("Scan Started.")
            Process.sleep(1000)
            {:noreply, state}
        end
    end
  end

  def handle_cast({:stop_scan}, state) do
    Logger.debug("handle_cast :stop_scan #{inspect(state)}")

    case state.central do
      nil ->
        Logger.debug("No central reference to stop scan.")
        {:noreply, state}

      central_ref ->
        # Call NIF to stop the scan using the central reference
        case RustlerBtleplug.Native.stop_scan(central_ref) do
          {:error, reason} ->
            Logger.debug("Failed to stop scan: #{reason}")
            {:noreply, state}

          _central_ref ->
            Logger.debug("Scan Stopped.")
            {:noreply, state}
        end
    end
  end

  def handle_call({:create_central}, _from, state) do
    case RustlerBtleplug.Native.create_central() do
      {:error, reason} ->
        {:error, reason}

      central_ref ->
        GenServer.cast(@name, {:set_central, central_ref})
        Logger.debug("Central Created and Reference Stored!")
        {:reply, {:ok, central_ref}, state}
    end
  end

  def handle_call({:find_peripheral_by_name, device_name}, _from, state) do
    case state.central do
      nil ->
        Logger.debug("No central reference to find_peripheral_by_name.")
        {:noreply, state}

      central_ref ->
        case RustlerBtleplug.Native.find_peripheral_by_name(central_ref, device_name) do
          {:error, reason} ->
            Logger.debug("Failed to find #{device_name}: #{reason}")
            {:noreply, state}

          peripheral_ref ->
            Logger.debug("Peripheral #{device_name} found #{inspect(peripheral_ref)}")
            {:reply, {:ok, peripheral_ref}, %{state | peripheral: peripheral_ref}}
        end
    end
  end

  def handle_call({:connect, _peripheral_ref}, _from, state) do
    case state.peripheral do
      nil ->
        Logger.debug("No peripheral reference to connect.")
        {:noreply, state}

      peripheral_ref ->
        case RustlerBtleplug.Native.connect(peripheral_ref) do
          {:error, reason} ->
            Logger.debug("Failed to connect to #{inspect(peripheral_ref)}: #{reason}")
            {:noreply, state}

          peripheral_ref ->
            Logger.debug("Connecting to #{inspect(peripheral_ref)}")
            {:reply, {:ok, peripheral_ref}, %{state | peripheral: peripheral_ref}}
        end
    end
  end

  def handle_call({:subscribe, uuid}, _from, state) do
    case state.peripheral do
      nil ->
        Logger.debug("No peripheral reference to subscribe to.")
        {:noreply, state}

      peripheral_ref ->
        case RustlerBtleplug.Native.subscribe(peripheral_ref, uuid) do
          {:error, reason} ->
            Logger.debug("Failed to subscribe to #{uuid}: #{reason}")
            {:noreply, state}

          peripheral_ref ->
            Logger.debug("Subscribing to #{uuid} #{inspect(peripheral_ref)}")
            {:reply, {:ok, peripheral_ref}, %{state | peripheral: peripheral_ref}}
        end
    end
  end

  def handle_call({:get_messages}, _from, state) do
    Logger.debug("handle_call :get_messages")
    {:reply, state.messages, state}
  end

  defp update_state_with_message(state, message) do
    %{state | messages: [message | state.messages]}
  end
end
