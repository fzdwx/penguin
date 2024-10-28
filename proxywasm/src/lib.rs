use std::sync::OnceLock;

pub trait Plugin: Send + Sync {
    /// Returns a new instance of the extension.
    fn new() -> Self
    where
        Self: Sized;

    fn request_filter(&self, _session: Session) -> Result<bool, String>;
}

static PLUGIN: OnceLock<Box<dyn Plugin>> = OnceLock::new();

pub fn register_plugin(build: fn() -> Box<dyn Plugin>) {
    let _ = PLUGIN.set(build());
}

#[macro_export]
macro_rules! register_plugin {
    ($extension_type:ty) => {
        #[export_name = "init-plugin"]
        pub extern "C" fn __init_extension() {
            api::register_extension(|| Box::new(<$extension_type as api::Extension>::new()));
        }
    };
}

wit_bindgen::generate!({
    path: "./wit",
    world: "proxy-wasm",
    skip:["init-plugin"]
});


struct PluginImpl;
export!(PluginImpl);

impl Guest for PluginImpl {
    fn request_filter(session: Session) -> Result<bool, String> {
        match PLUGIN.get() {
            Some(ext) => ext.request_filter(session),
            None => Err("Extension not loaded".to_string()),
        }
    }
}
