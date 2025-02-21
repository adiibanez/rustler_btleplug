defmodule RustlerBtleplug.MixProject do
  use Mix.Project

  @version "0.0.1-alpha"
  @source_url "https://github.com/adiibanez/rustler_btleplug"
  @dev? String.ends_with?(@version, "-dev")
  @force_build? System.get_env("BTLEPLUG_BUILD") in ["1", "true"]

  @nerves_rust_target_triple_mapping %{
    "armv6-nerves-linux-gnueabihf": "arm-unknown-linux-gnueabihf",
    "armv7-nerves-linux-gnueabihf": "armv7-unknown-linux-gnueabihf",
    "aarch64-nerves-linux-gnu": "aarch64-unknown-linux-gnu",
    "x86_64-nerves-linux-musl": "x86_64-unknown-linux-musl"
  }

  def project do
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

      IO.puts("RUSTLER_TARGET mapping #{inspect(mapping)}")

      # if is_binary(mapping) do
      #   System.put_env("RUSTLER_TARGET", mapping)
      # end
    end

    [
      app: :rustler_btleplug,
      name: "Rustler btleplug",
      description: "Allows ble communication between elixir and rust",
      version: @version,
      elixir: "~> 1.18",
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

  # Run "mix help compile.app" to learn about applications.
  def application do
    [
      extra_applications: [:logger, :rustler]
    ]
  end

  defp package do
    [
      files: [
        "lib",
        "native/example/.cargo",
        "native/example/src",
        "native/example/Cargo*",
        "checksum-*.exs",
        "mix.exs"
      ]
    ]
  end

  # Run "mix help deps" to learn about dependencies.
  defp deps do
    [
      # {:rustler, "~> 0.31.0"},
      {:rustler, ">= 0.31.0", optional: true},
      {:rustler_precompiled, "~> 0.7"}
      # {:dep_from_hexpm, "~> 0.3.0"},
      # {:dep_from_git, git: "https://github.com/elixir-lang/my_dep.git", tag: "0.1.0"}
    ]
  end
end
