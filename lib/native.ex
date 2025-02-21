defmodule RustlerBtleplug.Native do
  # use Rustler, otp_app: :rustler_btleplug, crate: :btleplug_client, target: System.get_env("RUSTLER_TARGET")

  version = Mix.Project.config()[:version]

  use RustlerPrecompiled,
    otp_app: :rustler_btleplug,
    crate: :btleplug_client,
    base_url: "https://github.com/adiibanez/rustler_btleplug/releases/download/v#{version}",
    force_build: System.get_env("RUSTLER_PRECOMPILATION_EXAMPLE_BUILD") in ["1", "true"],
    version: version

  @type central() :: reference()
  @type peripheral() :: reference()
  @type uuid() :: String.t()
  @type mac() :: String.t()

  @spec init(map()) :: {:ok, central()} | {:error, term()}
  def init(_opts \\ %{}), do: error()

  @spec create_central() :: {:ok, central()} | {:error, term()}
  def create_central(), do: error()
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
  @spec find_peripheral(central(), uuid()) :: {:ok, peripheral()} | {:error, term()}
  def find_peripheral(_central, _uuid), do: error()

  @spec connect(peripheral()) :: {:ok, peripheral()} | {:error, term()}
  def connect(_peripheral), do: error()
  def subscribe(_peripheral, _characteristics), do: error()

  @spec test_string(String.t()) :: {:ok, String.t()} | {:error, term()}
  def test_string(_string), do: error()

  @spec add(Number.t(), Number.t()) :: {:ok, Number.t()} | {:error, term()}
  def add(_a, _b), do: error()

  @spec get_map() :: {:ok, map()} | {:error, term()}
  def get_map(), do: error()

  defp error, do: :erlang.nif_error(:nif_not_loaded)
end
