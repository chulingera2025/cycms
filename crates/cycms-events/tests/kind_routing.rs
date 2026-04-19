//! 按 `EventKind` 路由：订阅 `A` 的 handler 不应收到 `B`。

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use cycms_events::{Event, EventBus, EventHandler, EventKind};

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
    for _ in 0..100 {
        if counter.load(Ordering::SeqCst) >= target {
            return;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    panic!("counter did not reach {target}");
}

#[tokio::test]
async fn handler_only_receives_subscribed_kind() {
    let bus = EventBus::new();
    let content_count = Arc::new(AtomicU64::new(0));
    let user_count = Arc::new(AtomicU64::new(0));

    let _c = bus.subscribe(
        EventKind::ContentCreated,
        Arc::new(Counter {
            name: "content".to_owned(),
            count: Arc::clone(&content_count),
        }),
    );
    let _u = bus.subscribe(
        EventKind::UserCreated,
        Arc::new(Counter {
            name: "user".to_owned(),
            count: Arc::clone(&user_count),
        }),
    );

    bus.publish(Event::new(EventKind::ContentCreated));
    bus.publish(Event::new(EventKind::ContentCreated));
    bus.publish(Event::new(EventKind::UserCreated));

    wait_for(&content_count, 2).await;
    wait_for(&user_count, 1).await;

    // 交叉验证：另一方不应有变化
    assert_eq!(content_count.load(Ordering::SeqCst), 2);
    assert_eq!(user_count.load(Ordering::SeqCst), 1);
}
