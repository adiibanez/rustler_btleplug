# defmodule RustlerBtleplug.ClientTest do
#   use ExUnit.Case, async: true
#   alias RustlerBtleplug.Client

#   setup do
#     {:ok, client} = start_supervised({Client, name: :btleplug_client})
#     {:ok, client: client}
#   end

#   test "BLE scan through GenServer", %{client: client} do
#     assert :ok = Client.scan()
#   end

#   test "Connect to a discovered device", %{client: client} do
#     assert :ok = Client.scan()
#     assert :ok = Client.connect_peripheral("device_uuid_123")
#   end
# end
