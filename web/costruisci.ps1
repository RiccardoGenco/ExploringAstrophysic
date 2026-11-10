# Costruisce la versione browser in web/pkg. La versione dello strumento deve
# coincidere con quella di wasm-bindgen in Cargo.lock.
$radice = Split-Path $PSScriptRoot
cargo build --manifest-path "$radice\Cargo.toml" --release -p colossi --target wasm32-unknown-unknown
& "$radice\target\strumenti\bin\wasm-bindgen.exe" --target web --no-typescript --out-dir "$radice\web\pkg" "$radice\target\wasm32-unknown-unknown\release\colossi.wasm"
