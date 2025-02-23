defmodule RustlerBtleplug.Native do
  version = Mix.Project.config()[:version]

  use Rustler,
    otp_app: :rustler_btleplug,
    crate: :btleplug_client,
    target: System.get_env("RUSTLER_TARGET")

  # use RustlerPrecompiled,
  #   otp_app: :rustler_btleplug,
  #   crate: :btleplug_client,
  #   base_url: "https://github.com/adiibanez/rustler_btleplug/releases/download/v#{version}",
  #   force_build: System.get_env("RUSTLER_PRECOMPILATION_EXAMPLE_BUILD") in ["1", "true"],
  #   version: version

  @type central() :: reference()
  @type peripheral() :: reference()
  @type gatt_peripheral() :: reference()
  @type uuid() :: String.t()
  @type mac() :: String.t()

  @default_timeout 2000

  @spec init(map()) :: {:ok, central()} | {:error, term()}
  def init(_opts \\ %{}), do: error()

  @spec create_central(Pid.t()) :: {:ok, central()} | {:error, term()}
  def create_central(_pid \\ self()), do: error()
  @spec start_scan(central(), Number.t()) :: {:ok, central()} | {:error, term()}
  def start_scan(_central, _ms \\ 1000), do: error()

  @spec stop_scan(central()) :: {:ok, central()} | {:error, term()}
  def stop_scan(_central), do: error()

  # @spec is_scanning(central()) :: {:ok, boolean()} | {:error, term()}
  # def is_scanning(_central), do: error()

  # def add_peripheral(_central, _mac), do: error()

  # get peripheral from known peripherals, eg. earlier scan
  # def get_peripheral(_central, _uuid), do: error()

  # scan and find peripheral
  @spec find_peripheral(central(), uuid(), Number.t()) :: {:ok, peripheral()} | {:error, term()}
  def find_peripheral(_central, _uuid, _timeout \\ @default_timeout), do: error()

  @spec find_peripheral_by_name(central(), String.t(), Number.t()) ::
          {:ok, peripheral()} | {:error, term()}
  def find_peripheral_by_name(_central, _name, _timeout \\ @default_timeout), do: error()

  @spec connect(peripheral(), Number.t()) :: {:ok, peripheral()} | {:error, term()}
  def connect(_peripheral, _timeout \\ @default_timeout), do: error()

  @spec subscribe(peripheral(), uuid(), Number.t()) :: {:ok, peripheral()} | {:error, term()}
  def subscribe(_peripheral, _characteristic, _timeout \\ @default_timeout), do: error()

  # @spec create_gatt_peripheral(String.t(), Number.t()) ::
  #         {:ok, gatt_peripheral()} | {:error, term()}
  # def create_gatt_peripheral(_peripheral_adverstising_name, _advertising_duration_ms \\ 60000),
  #   do: error()

  @spec test_string(String.t()) :: {:ok, String.t()} | {:error, term()}
  def test_string(_string), do: error()

  @spec add(Number.t(), Number.t()) :: {:ok, Number.t()} | {:error, term()}
  def add(_a, _b), do: error()

  @spec get_map() :: {:ok, map()} | {:error, term()}
  def get_map(), do: error()

  defp error, do: :erlang.nif_error(:nif_not_loaded)
end
