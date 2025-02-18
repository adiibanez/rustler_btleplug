defmodule RustlerBtleplug.Native do
  use Rustler, otp_app: :rustler_btleplug, crate: :btleplug_client

  # @enforce_keys [:native]
  # defstruct [:native]

  # @opaque t() :: String.t()

  @type init_options() :: %{
          optional(:some_option) => String.t()
        }

  def init(opts \\ %{})
  def init(%{} = opts), do: __init__(opts)
  def init(_), do: {:error, :invalid_options}

  def __init__(_opts), do: error()
  def add(_a, _b), do: error()
  def get_map(), do: error()
  def scan(), do: error()
  def connect(_uuid), do: error()
  defp error, do: :erlang.nif_error(:nif_not_loaded)
end
