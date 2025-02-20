defmodule RustlerBtleplug.Client do
  use GenServer

  @type state() :: %{
          ble_resource: reference() | nil
        }

  ### --- CLIENT API ---

  @spec start_link(keyword()) :: GenServer.on_start()
  def start_link(opts \\ []) do
    GenServer.start_link(__MODULE__, opts, name: Keyword.get(opts, :name, __MODULE__))
  end

  @spec start_scan() :: :ok
  def start_scan do
    GenServer.call(__MODULE__, :start_scan)
  end

  @spec connect(String.t()) :: :ok
  def connect(device_id) do
    GenServer.call(__MODULE__, {:connect, device_id})
  end

  ### --- SERVER CALLBACKS ---

  @impl true
  def init(_opts) do
    case RustlerBtleplug.Native.init() do
      {:ok, ble_resource} ->
        {:ok, %{ble_resource: ble_resource}}

      {:error, reason} ->
        {:stop, reason}
    end
  end

  @impl true
  def handle_call(:start_scan, _from, %{ble_resource: ble_resource} = state) do
    case RustlerBtleplug.Native.start_scan(ble_resource) do
      {:ok, new_resource} ->
        {:reply, :ok, %{state | ble_resource: new_resource}}

      {:error, reason} ->
        {:reply, {:error, reason}, state}
    end
  end

  @impl true
  def handle_call({:connect, device_id}, _from, %{ble_resource: ble_resource} = state) do
    case RustlerBtleplug.Native.connect_peripheral(ble_resource, device_id) do
      {:ok, new_resource} ->
        {:reply, :ok, %{state | ble_resource: new_resource}}

      {:error, reason} ->
        {:reply, {:error, reason}, state}
    end
  end
end
