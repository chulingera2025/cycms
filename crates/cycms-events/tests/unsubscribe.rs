//! 取消订阅：多订阅者场景下 `unsubscribe` 只终止目标 handler，其他 handler 继续收到。

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
async fn unsubscribe_leaves_other_handlers_intact() {
    let bus = EventBus::new();
    let a = Arc::new(AtomicU64::new(0));
    let b = Arc::new(AtomicU64::new(0));

    let h_a = bus.subscribe(
        EventKind::MediaUploaded,
        Arc::new(Counter {
            name: "a".to_owned(),
            count: Arc::clone(&a),
        }),
    );
    let _h_b = bus.subscribe(
        EventKind::MediaUploaded,
        Arc::new(Counter {
            name: "b".to_owned(),
            count: Arc::clone(&b),
        }),
    );

    bus.publish(Event::new(EventKind::MediaUploaded));
    wait_for(&a, 1).await;
    wait_for(&b, 1).await;

    bus.unsubscribe(h_a);
    tokio::time::sleep(Duration::from_millis(20)).await;

    bus.publish(Event::new(EventKind::MediaUploaded));
    wait_for(&b, 2).await;

    assert_eq!(
        a.load(Ordering::SeqCst),
        1,
        "unsubscribed handler must not be called"
    );
}
