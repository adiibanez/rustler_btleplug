defmodule RustlerBtleplug.Native do
  @moduledoc false

  use Rustler,
    otp_app: :rustler_btleplug,
    crate: :btleplug_client,
    skip_compilation?: true

  # @on_load :load_nifs
  # def load_nifs do
  #   :ok
  # end

  version = Mix.Project.config()[:version]

  # use RustlerPrecompiled,
  #   otp_app: :rustler_btleplug,
  #   crate: :btleplug_client,
  #   base_url: "https://github.com/adiibanez/rustler_btleplug/releases/download/v#{version}",
  #   force_build: System.get_env("RUSTLER_BTLEPLUG_BUILD") in ["1", "true"],
  #   version: version,
  #   max_retries: 0,
  #   targets: [
  #     "aarch64-apple-darwin",
  #     "x86_64-apple-darwin",
  #     "aarch64-apple-ios-sim",
  #     "aarch64-apple-ios",
  #     "x86_64-apple-ios",
  #     "aarch64-unknown-linux-gnu",
  #     "aarch64-unknown-linux-musl",
  #     "x86_64-pc-windows-msvc",
  #     "x86_64-unknown-linux-gnu",
  #     "x86_64-unknown-linux-musl"
  #   ]

  ## Type Definitions
  @type central() :: reference()
  @type peripheral() :: reference()
  @type gatt_peripheral() :: reference()
  @type uuid() :: String.t()
  @type mac() :: String.t()
  @type state_graph() :: String.t()
  # @type state_map() :: %{
  #         adapter: %RustlerBtleplug.AdapterInfo{},
  #         peripherals: %{uuid() => %RustlerBtleplug.PeripheralInfo{}},
  #         services: %{uuid() => %RustlerBtleplug.ServiceInfo{}},
  #         characteristics: %{uuid() => %RustlerBtleplug.CharacteristicInfo{}}
  #       }

  @default_timeout 2000

  ## Core BLE Functions
  @spec init(map()) :: {:ok, central()} | {:error, term()}
  def init(_opts \\ %{}), do: error()

  @spec create_central(Pid.t()) :: {:ok, central()} | {:error, term()}
  def create_central(_pid \\ self()), do: error()

  @spec start_scan(central(), number()) :: {:ok, central()} | {:error, term()}
  def start_scan(_central, _ms \\ 1000), do: error()

  @spec stop_scan(central()) :: {:ok, central()} | {:error, term()}
  def stop_scan(_central), do: error()

  ## Peripheral Discovery
  @doc """
  Find a peripheral by UUID.
  """
  @spec find_peripheral(central(), uuid(), number()) ::
          {:ok, peripheral()} | {:error, term()}
  def find_peripheral(_central, _uuid, _timeout \\ @default_timeout), do: error()

  @doc """
  Find a peripheral by name.
  """
  @spec find_peripheral_by_name(central(), String.t(), number()) ::
          {:ok, peripheral()} | {:error, term()}
  def find_peripheral_by_name(_central, _name, _timeout \\ @default_timeout), do: error()

  ## Peripheral Connection
  @spec connect(peripheral(), number()) :: {:ok, peripheral()} | {:error, term()}
  def connect(_peripheral, _timeout \\ @default_timeout), do: error()

  @spec disconnect(peripheral(), number()) :: {:ok, peripheral()} | {:error, term()}
  def disconnect(_peripheral, _timeout \\ @default_timeout), do: error()

  ## Notifications & Subscriptions
  @spec subscribe(peripheral(), uuid(), number()) :: {:ok, peripheral()} | {:error, term()}
  def subscribe(_peripheral, _characteristic, _timeout \\ @default_timeout), do: error()

  @spec unsubscribe(peripheral(), uuid(), number()) :: {:ok, peripheral()} | {:error, term()}
  def unsubscribe(_peripheral, _characteristic, _timeout \\ @default_timeout), do: error()

  ## Adapter State Queries (Graph & Mindmap)
  @doc """
  Retrieve the adapter state as a **GraphViz** or **Mermaid mindmap**.
  """
  @spec get_adapter_state_graph(central(), String.t()) :: {:ok, state_graph()} | {:error, term()}
  def get_adapter_state_graph(_central, _variant \\ "mindmap"), do: error()

  ## Adapter State as Structured Map (Elixir-friendly)
  @doc """
  Retrieve the **full adapter state** as a structured map.

  This map contains:
    - `adapter` (Info about the adapter)
    - `peripherals` (Map of discovered peripherals)
    - `services` (Map of services)
    - `characteristics` (Map of characteristics)
  """
  @spec get_adapter_state_map(central()) ::
          {:ok,
           %{
             adapter: RustlerBtleplug.AdapterInfo.t(),
             peripherals: %{String.t() => RustlerBtleplug.PeripheralInfo.t()},
             services: %{String.t() => RustlerBtleplug.ServiceInfo.t()},
             characteristics: %{String.t() => RustlerBtleplug.CharacteristicInfo.t()}
           }}
          | {:error, term()}
  def get_adapter_state_map(_central), do: error()

  ## Utility / Debug Functions
  @spec test_string(String.t()) :: {:ok, String.t()} | {:error, term()}
  def test_string(_string), do: error()

  @spec add(number(), number()) :: {:ok, number()} | {:error, term()}
  def add(_a, _b), do: error()

  @spec get_map() :: {:ok, map()} | {:error, term()}
  def get_map(), do: error()

  ## ============================================================
  ## Peripheral (Server/Advertiser) Mode Functions
  ## ============================================================

  @type peripheral_manager() :: reference()
  @type service_definition() :: %{
          uuid: String.t(),
          primary: boolean(),
          characteristics: [characteristic_definition()]
        }
  @type characteristic_definition() :: %{
          uuid: String.t(),
          properties: characteristic_properties(),
          value: binary() | nil
        }
  @type characteristic_properties() :: %{
          read: boolean(),
          write: boolean(),
          write_without_response: boolean(),
          notify: boolean(),
          indicate: boolean()
        }

  @doc """
  Create a new BLE peripheral manager for server/advertiser mode.
  Events will be sent to the calling process.
  """
  @spec create_peripheral(pid()) :: {:ok, peripheral_manager()} | {:error, term()}
  def create_peripheral(_pid \\ self()), do: error()

  @doc """
  Add a GATT service to the peripheral.
  """
  @spec add_service(peripheral_manager(), service_definition()) :: :ok | {:error, term()}
  def add_service(_peripheral_manager, _service_definition), do: error()

  @doc """
  Start advertising the peripheral with the given device name and service UUIDs.
  """
  @spec start_advertising(peripheral_manager(), String.t(), [String.t()]) ::
          :ok | {:error, term()}
  def start_advertising(_peripheral_manager, _device_name, _service_uuids), do: error()

  @doc """
  Stop advertising.
  """
  @spec stop_advertising(peripheral_manager()) :: :ok | {:error, term()}
  def stop_advertising(_peripheral_manager), do: error()

  @doc """
  Respond to a read request from a connected central.
  """
  @spec respond_to_read_request(peripheral_manager(), non_neg_integer(), binary(), boolean()) ::
          :ok | {:error, term()}
  def respond_to_read_request(_peripheral_manager, _request_id, _value, _success), do: error()

  @doc """
  Respond to a write request from a connected central.
  """
  @spec respond_to_write_request(peripheral_manager(), non_neg_integer(), boolean()) ::
          :ok | {:error, term()}
  def respond_to_write_request(_peripheral_manager, _request_id, _success), do: error()

  @doc """
  Update a characteristic value and notify subscribed centrals.
  """
  @spec update_characteristic(peripheral_manager(), String.t(), binary()) ::
          :ok | {:error, term()}
  def update_characteristic(_peripheral_manager, _characteristic_uuid, _value), do: error()

  @doc """
  Check if the peripheral is currently advertising.
  """
  @spec is_advertising(peripheral_manager()) :: {:ok, boolean()} | {:error, term()}
  def is_advertising(_peripheral_manager), do: error()

  ## Handle NIF errors when Rust module isn't loaded
  defp error, do: :erlang.nif_error(:nif_not_loaded)
end
