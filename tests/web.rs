//! Test suite for the Web and headless browsers.

#![cfg(target_arch = "wasm32")]

extern crate wasm_bindgen_test;
use js_sys::WebAssembly;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use wasm_bindgen_test::*;
use wasmer_wasi_js::WASI;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn pass() {
    let window = web_sys::window().expect("Failed to get window.");

    let mut wasi = WASI::new(js_sys::Object::new().into()).expect("Failed to create wasi object");

    let bytes = JsFuture::from(window.fetch_with_str("https://deno.land/x/wasm/tests/demo.wasm"))
        .await
        .expect("Failed to fetch demo.wasm");

    let module = JsFuture::from(WebAssembly::compile(&bytes))
        .await
        .expect("Failed to compile wasm module.");

    let instance = wasi
        .instantiate(module, None)
        .expect("Failed to instaniate Webassembly module.");
}
