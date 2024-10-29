use crate::config::def::WasmPluginDef;
use crate::core::plugin::{Plugin, PluginCtx};
use crate::plugins::wasm::qwe::penguin;
use crate::plugins::wasm::qwe::penguin::sdk::types::HostSession;
use crate::plugins::wasm::qwe::ProxyWasm;
use async_trait::async_trait;
use pingora::prelude::Session;
use std::sync::{Arc, OnceLock, RwLock};
use wasmtime::component::{Component, Resource, ResourceTable};
use wasmtime::Store;
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiView};
use crate::plugins::wasm::qwe::penguin::sdk::types;

pub async fn load_wasm_plugin(plugins: &[WasmPluginDef]) {
    let host = Arc::new(WasmHost::new());
    for plugin in plugins {
        let host = host.clone();
        let (plugin, store) = host.load_plugin(plugin).await;
        let plugin = ProxyPlugin::new(plugin, store);
    }
}

struct ProxyPlugin {
    plugin: RwLock<ProxyWasm>,
    store: RwLock<Store<WasmState>>,
}

unsafe impl Sync for ProxyPlugin {}

#[async_trait]
impl Plugin for ProxyPlugin {
    async fn request_filter(&self, session: &mut Session, _ctx: &mut PluginCtx) -> pingora::Result<bool> {
        let p = self.plugin.write().unwrap();
        let mut s = self.store.write().unwrap();
        let session: Box<dyn HostSession> = Box::new(session);
        let result = s.data_mut().table().push(session).unwrap();
        p.call_request_filter(&s, result).await.unwrap();
        Ok(true)
    }
}

#[async_trait]
impl HostSession for Session {
    async fn write_response(&mut self, self_: Resource<penguin::sdk::types::Session>, body: Vec<u8>, end_of_stream: bool) -> Result<(), ()> {
        self.write_response_body(Some(bytes::Bytes::from(body)), end_of_stream).await.unwrap();
        Ok(())
    }

    async fn drop(&mut self, _rep: Resource<penguin::sdk::types::Session>) -> wasmtime::Result<()> {
        Ok(())
    }
}

impl ProxyPlugin {
    fn new(plugin: ProxyWasm, store: Store<WasmState>) -> Self {
        Self {
            plugin: RwLock::new(plugin),
            store: RwLock::new(store),
        }
    }
}

fn wasm_engine() -> wasmtime::Engine {
    static WASM_ENGINE: OnceLock<wasmtime::Engine> = OnceLock::new();

    WASM_ENGINE
        .get_or_init(|| {
            let mut config = wasmtime::Config::new();
            config.wasm_component_model(true);
            config.async_support(true);
            wasmtime::Engine::new(&config).unwrap()
        })
        .clone()
}

mod qwe {
    wasmtime::component::bindgen!({
    async: true,
    path: "./proxywasm/wit",
});
}

pub(crate) struct WasmHost {
    engine: wasmtime::Engine,
}

struct WasmState {
    ctx: WasiCtx,
    table: ResourceTable,
}

impl WasiView for WasmState {
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }

    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.ctx
    }
}

impl WasmHost {
    pub fn new() -> Self {
        Self { engine: wasm_engine() }
    }

    pub async fn load_plugin(self: &Arc<Self>, plugin: &WasmPluginDef) -> (ProxyWasm, Store<WasmState>) {
        let wasm_bytes = Self::load_wasm(plugin).unwrap();
        let component = Component::from_binary(&self.engine, &wasm_bytes).unwrap();

        let mut builder = WasiCtxBuilder::new();

        let mut store = Store::new(
            &self.engine,
            WasmState {
                ctx: builder.build(),
                table: ResourceTable::new(),
            },
        );

        let mut linker = wasmtime::component::Linker::<WasmState>::new(&self.engine);
        wasmtime_wasi::add_to_linker_async(&mut linker).unwrap();
        let extension = ProxyWasm::instantiate_async(&mut store, &component, &linker).await.unwrap();

        extension.call_init_plugin(&mut store).await.unwrap();
        (extension, store)
    }

    fn load_wasm(plugin: &WasmPluginDef) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        Ok(std::fs::read(&plugin.url)?)
    }
}