# sh docs/git-roundtrip.sh
RUSTLER_PRECOMPILED_FORCE_BUILD_ALL=true mix rustler_precompiled.download RustlerBtleplug.Native --all

mix hex.build
mix hex.publish --yes --replace