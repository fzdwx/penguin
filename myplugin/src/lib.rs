use proxywasm::{register_plugin, Plugin, Session};

struct MyPlugin;

impl Plugin for MyPlugin {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {}
    }

    fn request_filter(&self, session: Session) -> Result<bool, String> {
        session.write_response_body(b"Hello, World 123123!", true);
        Ok(true)
    }
}

register_plugin!(MyPlugin);
