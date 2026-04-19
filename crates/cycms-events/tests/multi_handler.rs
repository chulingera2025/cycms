//! 多订阅者广播：同一 `EventKind` 上有多个 handler 时，每个都应各自收到事件。

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
async fn three_handlers_each_receive_all_events() {
    let bus = EventBus::new();
    let a = Arc::new(AtomicU64::new(0));
    let b = Arc::new(AtomicU64::new(0));
    let c = Arc::new(AtomicU64::new(0));

    let _h1 = bus.subscribe(
        EventKind::UserCreated,
        Arc::new(Counter {
            name: "a".to_owned(),
            count: Arc::clone(&a),
        }),
    );
    let _h2 = bus.subscribe(
        EventKind::UserCreated,
        Arc::new(Counter {
            name: "b".to_owned(),
            count: Arc::clone(&b),
        }),
    );
    let _h3 = bus.subscribe(
        EventKind::UserCreated,
        Arc::new(Counter {
            name: "c".to_owned(),
            count: Arc::clone(&c),
        }),
    );
    assert_eq!(bus.receiver_count(&EventKind::UserCreated), 3);

    for _ in 0..4 {
        bus.publish(Event::new(EventKind::UserCreated));
    }

    wait_for(&a, 4).await;
    wait_for(&b, 4).await;
    wait_for(&c, 4).await;
}
