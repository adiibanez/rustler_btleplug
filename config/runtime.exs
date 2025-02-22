import Config

config :rustler_precompiled,
      ignore_unavailable: true,
      attempts: 0,
      targets: ["arm-unknown-linux-gnueabihf","aarch64-unknown-linux-gnu","aarch64-unknown-linux-musl","x86_64-apple-darwin","x86_64-pc-windows-gnu","x86_64-pc-windows-gnu","x86_64-pc-windows-msvc","x86_64-pc-windows-msvc",
"x86_64-unknown-linux-gnu","x86_64-unknown-linux-gnu","x86_64-unknown-linux-musl"]
