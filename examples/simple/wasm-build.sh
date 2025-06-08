set -e


#rustup target add wasm32-unknown-unknown
#cargo install -f wasm-bindgen-cli
#cargo install simple-http-server

cargo build --target wasm32-unknown-unknown -p simple

wasm-bindgen ../../target/wasm32-unknown-unknown/debug/simple.wasm --target web --no-typescript --out-dir ../../target/generated --out-name simple --debug --keep-debug

cp index.html ../../target/generated

simple-http-server ../../target/generated -c wasm,html,js -i --coep --coop --ip 127.0.0.1
