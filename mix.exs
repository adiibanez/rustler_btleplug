defmodule RustlerBtleplug.MixProject do
  use Mix.Project

  @version "0.0.5-alpha"
  @source_url "https://github.com/adiibanez/rustler_btleplug"
  @dev? String.ends_with?(@version, "-dev")
  @force_build? System.get_env("RUSTLER_BTLEPLUG_BUILD") in ["1", "true"]

  @nerves_rust_target_triple_mapping %{
    "armv6-nerves-linux-gnueabihf": "arm-unknown-linux-gnueabihf",
    "armv7-nerves-linux-gnueabihf": "armv7-unknown-linux-gnueabihf",
    "aarch64-nerves-linux-gnu": "aarch64-unknown-linux-gnu",
    "x86_64-nerves-linux-musl": "x86_64-unknown-linux-musl"
  }

  def project do
    if @force_build? == true or @dev? == true do
      IO.puts("Forcing rustler build for version #{@version} dev?: #{@dev?}")
    else
      IO.puts("Using precompiled NIFs #{@version} dev?: #{@dev?} #{System.get_env("RUSTLER_BTLEPLUG_BUILD") in ["1", "true"]}")
    end

    if is_binary(System.get_env("NERVES_SDK_SYSROOT")) do

      components =
        System.get_env("CC")
        |> tap(&System.put_env("RUSTFLAGS", "-C linker=#{&1}"))
        |> Path.basename()
        |> String.split("-")

      target_triple =
        components
        |> Enum.slice(0, Enum.count(components) - 1)
        |> Enum.join("-")

      mapping = Map.get(@nerves_rust_target_triple_mapping, String.to_atom(target_triple))

      if is_binary(mapping) do
        IO.puts("mapping: #{mapping}, TARGET_ARCH #{System.get_env("TARGET_ARCH")}, NERVES_SDK_SYSROOT #{System.get_env("NERVES_SDK_SYSROOT")}, RUSTFLAGS: #{IO.puts("NERVES_SDK_SYSROOT #{System.get_env("RUST_FLAGS")}")}")
        #System.put_env("TARGET_ARCH", "aarch64-unknown-linux-gnu")
        System.put_env("RUSTLER_TARGET", mapping)
      end
    end

    [
      app: :rustler_btleplug,
      name: "Rustler btleplug",
      description:
        "Elixir library providing Bluetooth Low Energy (BLE) client functionality through a Rustler NIF. Uses the btleplug crate to scan for, connect to, and interact with BLE peripherals. POC basic functionality and only client (central) mode is supported resp. usable currently.",
      version: @version,
      elixir: "~> 1.15",
      # elixirc_paths: elixirc_paths(Mix.env()),
      package: package(),
      deps: deps(),
      # docs: docs(),
      preferred_cli_env: [ci: :test],
      aliases: [
        "rust.lint": [
          "cmd cargo clippy --manifest-path=native/btleplug_client/Cargo.toml -- -Dwarnings"
        ],
        "rust.fmt": ["cmd cargo fmt --manifest-path=native/btleplug_client/Cargo.toml --all"],
        # "localstack.setup": ["cmd ./test/support/setup-localstack.sh"],
        ci: ["format", "rust.fmt", "rust.lint", "test"],
        fmt: ["format", "rust.fmt"]
      ],
      start_permanent: Mix.env() == :prod,
      dynamic_library_extension: :dylib
    ]
  end

  def application do
    [
      # mod: {RustlerBtleplug.Application, []},
      extra_applications: [:logger, :rustler]
    ]
  end

  defp package do
    [
      licenses: ["MIT"],
      links: %{
        GitHub: @source_url,
        LiveBook: "#{@source_url}/blob/main/livebooks/ble_demo.livemd"
      },
      files: [
        "lib",
        "native/btleplug_client/.cargo",
        "native/btleplug_client/src",
        "native/btleplug_client/Cargo*",
        "checksum-*.exs",
        "mix.exs"
      ]
    ]
  end

  defp deps do
    [
      {:rustler, ">= 0.31.0", optional: true},
      {:rustler_precompiled, "~> 0.7"},
      {:ex_doc, ">= 0.0.0", only: :dev, runtime: false}
    ]
  end
end
