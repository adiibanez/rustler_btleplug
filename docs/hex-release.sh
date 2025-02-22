# sh docs/git-roundtrip.sh
mix rustler_precompiled.download RustlerBtleplug.Native --all

mix hex.build
mix hex.publish --yes --replace