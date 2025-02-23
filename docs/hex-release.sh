# sh docs/git-roundtrip.sh
#RUSTLER_PRECOMPILED_FORCE_BUILD_ALL=true mix rustler_precompiled.download RustlerBtleplug.Native --all
mix rustler_precompiled.download RustlerBtleplug.Native --all --ignore-unavailable

# /Users/adrianibanez//Library/Caches/rustler_precompiled/precompiled_nifs/libbtleplug_client-v0.0.6-alpha-nif-2.15-aarch64-apple-darwin.so.tar.gz

mix hex.build
mix hex.publish --yes --replace