defmodule RustlerBtleplug.MixProject do
  use Mix.Project

  @version "0.0.1-alpha"
  @source_url "https://github.com/adiibanez/rustler_btleplug"
  @dev? String.ends_with?(@version, "-dev")
  @force_build? System.get_env("BTLEPLUG_BUILD") in ["1", "true"]

  def project do
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
      mod: {RustlerBtleplug.Application, []},
      extra_applications: [:logger, :rustler]
    ]
  end

  defp package do
    [
      licenses: ["MIT"],
      links: ["https://github.com/adiibanez/rustler_btleplug"],
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

  # Run "mix help deps" to learn about dependencies.
  defp deps do
    [
      {:rustler, ">= 0.31.0", optional: true},
      {:rustler_precompiled, "~> 0.7"}
      # {:dep_from_hexpm, "~> 0.3.0"},
      # {:dep_from_git, git: "https://github.com/elixir-lang/my_dep.git", tag: "0.1.0"}
    ]
  end
end
