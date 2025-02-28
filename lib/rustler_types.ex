defmodule RustlerBtleplug.AdapterInfo do
  @moduledoc false
  @enforce_keys [:name]
  defstruct [:name]

  @type t :: %__MODULE__{name: String.t()}
end

defmodule RustlerBtleplug.PeripheralInfo do
  @moduledoc false
  @enforce_keys [:id, :name, :rssi, :tx_power]
  defstruct [:id, :name, :rssi, :tx_power]

  @type t :: %__MODULE__{
          id: String.t(),
          name: String.t(),
          rssi: integer() | nil,
          tx_power: integer() | nil
        }
end

defmodule RustlerBtleplug.ServiceInfo do
  @moduledoc false
  @enforce_keys [:uuid]
  defstruct [:uuid]

  @type t :: %__MODULE__{uuid: String.t()}
end

defmodule RustlerBtleplug.CharacteristicInfo do
  @moduledoc false
  @enforce_keys [:uuid, :properties]
  defstruct [:uuid, :properties]

  @type t :: %__MODULE__{
          uuid: String.t(),
          properties: [String.t()]
        }
end
