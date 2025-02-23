# RustlerBtleplug

Elixir library providing Bluetooth Low Energy (BLE) client functionality through a Rustler NIF. Uses the btleplug crate to scan for, connect to, and interact with BLE peripherals. Currently, only client (central) mode is supported. Currently no data is decoded on rust side. Might make more sense to handle that in elixir. 
API is work progress. Feedback is welcome on how you would like to interact with a BLE api.
General modes are via genserver or piping. 

[![Run in Livebook](https://livebook.dev/badge/v1/blue.svg)](https://livebook.dev/run?url=https://github.com/adiibanez/rustler_btleplug/blob/main/livebooks/ble_demo.livemd)

## Features

- Central and peripheral resource management via refs. Elixir manages the livecycle of Rust resources. 
- BLE device scanning
- Service and characteristic discovery
- Peripheral connection
- Subscription to characteristic notifications

## Installation

Add `rustler_btleplug` to your list of dependencies in `mix.exs`:

```elixir
def deps do
  [
    {:rustler_btleplug, "~> 0.0.6-alpha"},
    {:rustler, ">= 0.31.0", optional: true} # rustler_precompiled dependency
  ]
end
```

```livebook
Mix.install([
  {:rustler_btleplug, "~> 0.0.6-alpha"}
])
```

## Usage Examples

### Basic Scanning

```elixir
alias RustlerBtleplug.Native

# Create a central manager and start scanning
central = Native.create_central()
  |> Native.start_scan(central)

# Receive scan events
receive do
  {:btleplug_peripheral_discovered, uuid} -> 
    IO.puts "Found device: #{uuid}"
end
```

### Connect to a Device

Some Standard characteristic UUIDs
- heartrate: 00002a37-0000-1000-8000-00805f9b34fb
- batteryLevel: 00002a19-0000-1000-8000-00805f9b34fb
- deviceName: 00002a00-0000-1000-8000-00805f9b34fb

```elixir
# Find and connect to a specific peripheral
peripheral = Native.create_central()
|> Native.start_scan()
|> Native.find_peripheral_by_name("Pressure") # rust checks contains
|> Native.connect()

# Subscribe to notifications
peripheral = Native.subscribe(peripheral, "61d20a90-71a1-11ea-ab12-0800200c9a66")
```

### Using the GenServer

```elixir
# Start the GenServer, it will register with name :ble_genserver
{:ok, _pid} = RustlerBtleplug.Genserver.start_link([])

# Create central and start scanning
{:ok, central_ref} = RustlerBtleplug.Genserver.create_central()
RustlerBtleplug.Genserver.start_scan()

# see livebook for more examples
```

## Running Tests

```bash
mix test
```

Example test cases:

```elixir
# Scan for devices
test "BLE default scanning" do
  resource = Native.create_central()
  |> Native.start_scan()

  assert_receive {:btleplug_scan_started, _msg}
  assert_receive {:btleplug_peripheral_discovered, _msg}
end

# Connect to a specific device
test "BLE connect to peripheral" do
  central_resource = Native.create_central()
  |> Native.start_scan()
  |> Native.find_peripheral_by_name("Pressure")
  |> Native.connect()
  |> Native.subscribe("61d20a90-71a1-11ea-ab12-0800200c9a66")

  assert_receive {:btleplug_peripheral_connected, _msg}
  assert_receive {:btleplug_characteristic_value_changed, _uuid, _value}
end
```

## Precompiled NIFs

Following targets are supported as precompiled NIFs. Infos mostly based on explorer project.
Feedback on platform support of these precompiled NIFs and PRs are more than welcome.

Compilation on host can be forced via env variable: 

```
export RUSTLER_BTLEPLUG_BUILD=true
export RUSTLER_BTLEPLUG_BUILD=1

RUSTLER_BTLEPLUG_BUILD=1 mix test
```

```
{ target: arm-unknown-linux-gnueabihf, os: ubuntu-20.04, use-cross: true }
{ target: aarch64-unknown-linux-gnu, os: ubuntu-20.04, use-cross: true }
{ target: aarch64-apple-darwin, os: macos-14 }
{ target: x86_64-apple-darwin, os: macos-13 }
{ target: x86_64-pc-windows-gnu, os: windows-2022, use-cross: true, rustflags: "-C target-feature=+fxsr,+sse,+sse2,+sse3,+ssse3,+sse4.1,+sse4.2,+popcnt,+avx,+fma" }
{ target: x86_64-pc-windows-gnu, os: windows-2022, use-cross: true, variant: "legacy_cpu" }
{ target: x86_64-pc-windows-msvc, os: windows-2019, rustflags: "-C target-feature=+fxsr,+sse,+sse2,+sse3,+ssse3,+sse4.1,+sse4.2,+popcnt,+avx,+fma" }
{ target: x86_64-pc-windows-msvc, os: windows-2019, variant: "legacy_cpu" }
{ target: x86_64-unknown-linux-gnu, os: ubuntu-20.04, rustflags: "-C target-feature=+fxsr,+sse,+sse2,+sse3,+ssse3,+sse4.1,+sse4.2,+popcnt,+avx,+fma" }
{ target: x86_64-unknown-linux-gnu, os: ubuntu-20.04, variant: "legacy_cpu" }
{ target: aarch64-unknown-linux-musl, os: ubuntu-22.04, use-cross: true } # , rustflags: "-C target-feature=-crt-static -C link-arg=-static" 
{ target: x86_64-unknown-linux-musl, os: ubuntu-22.04, use-cross: true }
```

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Documentation

Documentation can be generated with [ExDoc](https://github.com/elixir-lang/ex_doc)
and published on [HexDocs](https://hexdocs.pm). Once published, the docs can
be found at [https://hexdocs.pm/rustler_btleplug](https://hexdocs.pm/rustler_btleplug).
