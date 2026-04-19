use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::broadcast::error::RecvError;
use tokio::task::AbortHandle;
use tracing::warn;
use uuid::Uuid;

use crate::bus::EventBus;
use crate::event::{Event, EventKind};

/// 订阅者提供的事件处理器。
///
/// handler 以 `Arc<dyn EventHandler>` 保存在后台 task 中被重复调用，因此要求
/// `Send + Sync + 'static`。返回的错误会被 `EventBus` 记录到 tracing，但不阻断
/// 其他订阅者（对齐 Requirements 9.2）。
#[async_trait]
pub trait EventHandler: Send + Sync + 'static {
    /// 处理器名称，供 tracing / 错误上下文使用。
    fn name(&self) -> &str;

    /// 处理单条事件。返回 Err 时不会传播到发布者，仅被 `EventBus` 记录。
    async fn handle(&self, event: Arc<Event>) -> cycms_core::Result<()>;
}

/// 订阅 ID，每次 `subscribe` 自动生成。
pub type SubscriptionId = Uuid;

/// 订阅句柄。Drop **不** 触发取消订阅（后台 task 继续运行），必须显式调用
/// [`SubscriptionHandle::unsubscribe`] 或 [`EventBus::unsubscribe`] 才会停止。
pub struct SubscriptionHandle {
    id: SubscriptionId,
    kind: EventKind,
    abort: AbortHandle,
}

impl SubscriptionHandle {
    #[must_use]
    pub fn id(&self) -> SubscriptionId {
        self.id
    }

    #[must_use]
    pub fn kind(&self) -> &EventKind {
        &self.kind
    }

    /// 显式解除订阅：abort 后台 task，receiver 随之 drop。
    pub fn unsubscribe(self) {
        self.abort.abort();
    }
}

impl EventBus {
    /// 注册 handler 订阅指定 `kind` 的事件。
    ///
    /// 内部启动一个 tokio task 持有 broadcast receiver，对每条事件调用
    /// `handler.handle()`。任务生命周期由返回的 [`SubscriptionHandle`] 控制；
    /// 句柄被丢弃时 task 仍会持续运行，需显式调用 `unsubscribe`。
    pub fn subscribe(
        &self,
        kind: EventKind,
        handler: Arc<dyn EventHandler>,
    ) -> SubscriptionHandle {
        let id = Uuid::new_v4();
        let mut rx = self.subscribe_channel(kind.clone());
        let handler_name = handler.name().to_owned();

        let task = tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(event) => {
                        if let Err(err) = handler.handle(event).await {
                            warn!(
                                handler = %handler_name,
                                error = %err,
                                "event handler returned error"
                            );
                        }
                    }
                    Err(RecvError::Closed) => break,
                    Err(RecvError::Lagged(skipped)) => {
                        warn!(
                            handler = %handler_name,
                            skipped,
                            "event handler lagged; old events dropped"
                        );
                    }
                }
            }
        });

        SubscriptionHandle {
            id,
            kind,
            abort: task.abort_handle(),
        }
    }

    /// 等效于 `handle.unsubscribe()`；提供对称 API。
    pub fn unsubscribe(&self, handle: SubscriptionHandle) {
        handle.unsubscribe();
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::Duration;

    use super::{EventHandler, SubscriptionHandle};
    use crate::bus::EventBus;
    use crate::event::{Event, EventKind};
    use async_trait::async_trait;
    use std::sync::Arc;

    struct Counter {
        name: String,
        count: Arc<AtomicU64>,
    }

    #[async_trait]
    impl EventHandler for Counter {
        fn name(&self) -> &str {
            &self.name
        }

        async fn handle(&self, _event: Arc<Event>) -> cycms_core::Result<()> {
            self.count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    async fn wait_for(counter: &Arc<AtomicU64>, target: u64) {
        for _ in 0..50 {
            if counter.load(Ordering::SeqCst) >= target {
                return;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        panic!("counter did not reach {target}");
    }

    #[tokio::test]
    async fn subscribe_dispatches_event_to_handler() {
        let bus = EventBus::new();
        let count = Arc::new(AtomicU64::new(0));
        let handler = Arc::new(Counter {
            name: "counter".to_owned(),
            count: Arc::clone(&count),
        });

        let handle: SubscriptionHandle = bus.subscribe(EventKind::UserCreated, handler);
        assert_eq!(handle.kind(), &EventKind::UserCreated);

        bus.publish(Event::new(EventKind::UserCreated));
        wait_for(&count, 1).await;

        bus.unsubscribe(handle);
    }

    #[tokio::test]
    async fn unsubscribe_stops_further_dispatch() {
        let bus = EventBus::new();
        let count = Arc::new(AtomicU64::new(0));
        let handler = Arc::new(Counter {
            name: "counter".to_owned(),
            count: Arc::clone(&count),
        });

        let handle = bus.subscribe(EventKind::UserDeleted, handler);
        bus.publish(Event::new(EventKind::UserDeleted));
        wait_for(&count, 1).await;

        bus.unsubscribe(handle);
        tokio::time::sleep(Duration::from_millis(10)).await;

        bus.publish(Event::new(EventKind::UserDeleted));
        tokio::time::sleep(Duration::from_millis(20)).await;

        assert_eq!(
            count.load(Ordering::SeqCst),
            1,
            "unsubscribed handler must not be called again"
        );
    }
}
