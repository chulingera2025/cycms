#[allow(warnings)]
mod bindings;

use bindings::cycms::plugin::{event, kv, log, route, settings};
use bindings::Guest;

struct HelloPlugin;

impl Guest for HelloPlugin {
    fn on_enable() -> Result<(), String> {
        log::info("hello-plugin: on-enable called");
        settings::set("enabled", "\"true\"").map_err(|e| format!("settings.set: {e}"))?;
        kv::set("init", "done").map_err(|e| format!("kv.set: {e}"))?;
        event::subscribe("content.created").map_err(|e| format!("event.subscribe: {e}"))?;
        route::register("/hello", "GET").map_err(|e| format!("route.register: {e}"))?;
        Ok(())
    }

    fn on_disable() -> Result<(), String> {
        log::info("hello-plugin: on-disable called");
        settings::set("enabled", "\"false\"").map_err(|e| format!("settings.set: {e}"))?;
        Ok(())
    }

    fn on_event(kind: String, payload_json: String) -> Result<(), String> {
        log::info(&format!(
            "hello-plugin: on-event kind={kind} payload={payload_json}"
        ));
        kv::set("last-event-kind", &kind).map_err(|e| format!("kv.set: {e}"))?;
        Ok(())
    }

    fn handle_http(request_json: String) -> Result<String, String> {
        log::info(&format!("hello-plugin: handle-http req={request_json}"));
        // 返回固定 200 响应，body 为 base64("hello from wasm")
        // "hello from wasm" 的 base64 = "aGVsbG8gZnJvbSB3YXNt"
        Ok(r#"{"status":200,"headers":[["content-type","text/plain"]],"body-base64":"aGVsbG8gZnJvbSB3YXNt"}"#.to_string())
    }
}

bindings::export!(HelloPlugin with_types_in bindings);
