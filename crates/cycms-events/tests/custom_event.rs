//! 插件自定义事件：`EventKind::Custom(..)` 可被订阅与发布，不与内建 kind 冲突。

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
async fn custom_event_can_be_subscribed_and_published() {
    let bus = EventBus::new();
    let count = Arc::new(AtomicU64::new(0));
    let custom = EventKind::Custom("newsletter.subscribed".to_owned());

    let _h = bus.subscribe(
        custom.clone(),
        Arc::new(Counter {
            name: "newsletter".to_owned(),
            count: Arc::clone(&count),
        }),
    );

    bus.publish(Event::new(custom.clone()));
    bus.publish(Event::new(custom));
    wait_for(&count, 2).await;
}

#[tokio::test]
async fn custom_event_kind_does_not_leak_into_builtin() {
    let bus = EventBus::new();
    let builtin_count = Arc::new(AtomicU64::new(0));
    let custom_count = Arc::new(AtomicU64::new(0));

    let _b = bus.subscribe(
        EventKind::PluginInstalled,
        Arc::new(Counter {
            name: "builtin".to_owned(),
            count: Arc::clone(&builtin_count),
        }),
    );
    let _c = bus.subscribe(
        EventKind::Custom("plugin.installed.custom".to_owned()),
        Arc::new(Counter {
            name: "custom".to_owned(),
            count: Arc::clone(&custom_count),
        }),
    );

    bus.publish(Event::new(EventKind::PluginInstalled));
    wait_for(&builtin_count, 1).await;

    bus.publish(Event::new(EventKind::Custom(
        "plugin.installed.custom".to_owned(),
    )));
    wait_for(&custom_count, 1).await;

    assert_eq!(builtin_count.load(Ordering::SeqCst), 1);
    assert_eq!(custom_count.load(Ordering::SeqCst), 1);
}
