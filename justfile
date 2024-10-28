wit:
    @wit-bindgen rust  proxywasm/wit/proxywasm.wit --out-dir proxywasm/wit

build_plugin:
    cargo component build --package myplugin --release

init:
    cargo install wit-bindgen-cli
    cargo install cargo-component
