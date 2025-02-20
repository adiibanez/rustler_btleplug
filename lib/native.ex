defmodule RustlerBtleplug.Native do
  use Rustler, otp_app: :rustler_btleplug, crate: :btleplug_client

  @type central() :: reference()
  @type peripheral() :: reference()
  @type uuid() :: String.t()
  @type mac() :: String.t()

  @spec init(map()) :: {:ok, ble_resource()} | {:error, term()}
  def init(_opts \\ %{}), do: error()

  def create_central(), do: error()
  @spec start_scan(central()) :: {:ok, ble_resource()} | {:error, term()}
  def start_scan(_central), do: error()

  def add_peripheral(_central, _mac), do: error()
  def get_peripheral(_central, _uuid), do: error()
  def find_peripheral(_central, _uuid), do: error()

  @spec connect(peripheral(), uuid()) :: {:ok, peripheral()} | {:error, term()}
  def connect(_peripheral, _uuid), do: error()
  def subscribe(_peripheral, _characteristics), do: error()

  def test_string(_uuid), do: error()

  defp error, do: :erlang.nif_error(:nif_not_loaded)
end
