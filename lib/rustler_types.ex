defmodule RustlerBtleplug.AdapterInfo do
  @moduledoc false
  @enforce_keys [:name]
  defstruct [:name]

  @type t :: %__MODULE__{name: String.t()}
end

defmodule RustlerBtleplug.PeripheralInfo do
  @moduledoc false
  @enforce_keys [:id, :name, :rssi, :tx_power, :services]
  defstruct [:id, :name, :rssi, :tx_power, :services]

  @type t :: %__MODULE__{
          id: String.t(),
          name: String.t(),
          rssi: integer() | nil,
          tx_power: integer() | nil,
          services: [RustlerBtleplug.ServiceInfo.t()]
        }
end

defmodule RustlerBtleplug.ServiceInfo do
  @moduledoc false
  @enforce_keys [:uuid, :characteristics]
  defstruct [:uuid, :characteristics]

  @type t :: %__MODULE__{
          uuid: String.t(),
          characteristics: [RustlerBtleplug.CharacteristicInfo.t()]
        }
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
