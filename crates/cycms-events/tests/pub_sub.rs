//! 基础发布/订阅：单 handler 订阅单 kind，`publish` 后 handler 被调用。

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
async fn publish_reaches_single_subscriber() {
    let bus = EventBus::new();
    let count = Arc::new(AtomicU64::new(0));
    let handler = Arc::new(Counter {
        name: "c".to_owned(),
        count: Arc::clone(&count),
    });

    let handle = bus.subscribe(EventKind::ContentCreated, handler);
    assert_eq!(bus.receiver_count(&EventKind::ContentCreated), 1);

    bus.publish(Event::new(EventKind::ContentCreated).with_actor("u1"));
    wait_for(&count, 1).await;

    bus.unsubscribe(handle);
}

#[tokio::test]
async fn multiple_events_accumulate_on_same_handler() {
    let bus = EventBus::new();
    let count = Arc::new(AtomicU64::new(0));
    let handler = Arc::new(Counter {
        name: "c".to_owned(),
        count: Arc::clone(&count),
    });

    let _h = bus.subscribe(EventKind::ContentUpdated, handler);
    for _ in 0..5 {
        bus.publish(Event::new(EventKind::ContentUpdated));
    }
    wait_for(&count, 5).await;
}
