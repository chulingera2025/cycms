//! `event` host 组。
//!
//! - `publish`：解析 `kind` 字符串为 [`EventKind`]，`payload-json` 解析为 `Value`，
//!   附加 `actor = plugin:<name>`，通过 [`EventBus::publish`] 分发。
//! - `subscribe`：把 guest 关心的 kind 追加到 `HostState.subscribed_event_kinds`；
//!   17.4 的 `load` 在 `on_enable` 返回后读取此列表，把 `on-event` 代理 handler
//!   订阅到 `EventBus`。多次 subscribe 同一 kind 做去重。

use cycms_events::{Event, EventKind};
use serde_json::Value;

use crate::bindings::cycms::plugin::event::Host;
use crate::host::HostState;

impl Host for HostState {
    async fn publish(
        &mut self,
        kind: String,
        payload_json: String,
    ) -> wasmtime::Result<Result<(), String>> {
        let payload: Value = if payload_json.trim().is_empty() {
            Value::Null
        } else {
            match serde_json::from_str(&payload_json) {
                Ok(v) => v,
                Err(e) => return Ok(Err(format!("event.publish: invalid payload json: {e}"))),
            }
        };
        let event_kind = EventKind::from(kind.as_str());
        let event = Event::new(event_kind)
            .with_actor(format!("plugin:{}", self.plugin_name))
            .with_payload(payload);
        self.event_bus.publish(event);
        Ok(Ok(()))
    }

    async fn subscribe(&mut self, kind: String) -> wasmtime::Result<Result<(), String>> {
        if !self.subscribed_event_kinds.iter().any(|k| k == &kind) {
            self.subscribed_event_kinds.push(kind);
        }
        Ok(Ok(()))
    }
}
