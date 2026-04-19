use std::collections::HashMap;
use std::sync::{Arc, PoisonError, RwLock};

use tokio::sync::broadcast;

use crate::event::{Event, EventKind};

/// 单桶 broadcast channel 默认容量。最慢订阅者超出容量时会收到 `Lagged(n)` 并
/// 丢弃最旧消息。v0.1 写死 256，v0.2 接入 `SettingsConfig`。
pub const DEFAULT_CHANNEL_CAPACITY: usize = 256;

/// 进程内异步事件总线。
///
/// 内部结构：`HashMap<EventKind, broadcast::Sender<Arc<Event>>>`，由 `RwLock`
/// 保护。发布者 [`EventBus::publish`] 会在对应 kind 桶存在时把事件发给所有订阅者；
/// 无订阅者时 `Sender::send` 返回的 `SendError` 会被静默吞掉（`NoReceivers` 对齐
/// Requirements 9.2 的「空订阅者为正常状态」语义）。
pub struct EventBus {
    channels: RwLock<HashMap<EventKind, broadcast::Sender<Arc<Event>>>>,
    capacity: usize,
}

impl EventBus {
    #[must_use]
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CHANNEL_CAPACITY)
    }

    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            channels: RwLock::new(HashMap::new()),
            capacity,
        }
    }

    /// 发布一条事件。当该 `kind` 无订阅者时为 no-op（吞掉 `SendError::NoReceivers`）。
    pub fn publish(&self, event: Event) {
        let kind = event.kind.clone();
        let guard = self
            .channels
            .read()
            .unwrap_or_else(PoisonError::into_inner);
        if let Some(sender) = guard.get(&kind) {
            // SendError 只在无订阅者时出现，对齐 9.2 的「静默丢弃」语义
            let _ = sender.send(Arc::new(event));
        }
    }

    /// 返回某类事件当前的订阅者数量（监控 / 测试用途）。
    #[must_use]
    pub fn receiver_count(&self, kind: &EventKind) -> usize {
        let guard = self
            .channels
            .read()
            .unwrap_or_else(PoisonError::into_inner);
        guard.get(kind).map_or(0, broadcast::Sender::receiver_count)
    }

    /// 获取 broadcast channel 容量（单桶）。
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// 取或建对应 `kind` 的 broadcast sender，并返回新 receiver。
    /// 供 7.4 `subscribe` 内部使用（由 handler 模块消费）。
    #[allow(dead_code)] // 7.4 handler 模块接入后成为活跃 API
    pub(crate) fn subscribe_channel(&self, kind: EventKind) -> broadcast::Receiver<Arc<Event>> {
        let capacity = self.capacity;
        let mut guard = self
            .channels
            .write()
            .unwrap_or_else(PoisonError::into_inner);
        let sender = guard
            .entry(kind)
            .or_insert_with(|| broadcast::channel::<Arc<Event>>(capacity).0);
        sender.subscribe()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{EventBus, DEFAULT_CHANNEL_CAPACITY};
    use crate::event::{Event, EventKind};

    #[test]
    fn new_uses_default_capacity() {
        let bus = EventBus::new();
        assert_eq!(bus.capacity(), DEFAULT_CHANNEL_CAPACITY);
    }

    #[test]
    fn with_capacity_overrides_default() {
        let bus = EventBus::with_capacity(8);
        assert_eq!(bus.capacity(), 8);
    }

    #[test]
    fn publish_without_subscribers_is_noop() {
        let bus = EventBus::new();
        bus.publish(Event::new(EventKind::ContentCreated));
        assert_eq!(bus.receiver_count(&EventKind::ContentCreated), 0);
    }

    #[tokio::test]
    async fn subscribe_channel_creates_bucket_and_delivers_event() {
        let bus = EventBus::new();
        let mut rx = bus.subscribe_channel(EventKind::UserCreated);
        assert_eq!(bus.receiver_count(&EventKind::UserCreated), 1);

        bus.publish(Event::new(EventKind::UserCreated).with_actor("u1"));
        let arrived = rx.recv().await.unwrap();
        assert_eq!(arrived.kind, EventKind::UserCreated);
        assert_eq!(arrived.actor_id.as_deref(), Some("u1"));
    }
}
