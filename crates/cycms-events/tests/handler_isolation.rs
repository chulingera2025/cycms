//! Handler 失败隔离：handler A 总是返回 Err、handler B 正常——B 必须继续收到事件。
//! 对齐 Requirements 9.2「单个处理器失败不影响其他处理器」。

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use cycms_events::{Event, EventBus, EventHandler, EventKind};

struct OkCounter {
    name: String,
    count: Arc<AtomicU64>,
}

#[async_trait]
impl EventHandler for OkCounter {
    fn name(&self) -> &str {
        &self.name
    }

    async fn handle(&self, _event: Arc<Event>) -> cycms_core::Result<()> {
        self.count.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

struct ErrCounter {
    name: String,
    count: Arc<AtomicU64>,
}

#[async_trait]
impl EventHandler for ErrCounter {
    fn name(&self) -> &str {
        &self.name
    }

    async fn handle(&self, _event: Arc<Event>) -> cycms_core::Result<()> {
        self.count.fetch_add(1, Ordering::SeqCst);
        Err(cycms_core::Error::Internal {
            message: "boom".to_owned(),
            source: None,
        })
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
async fn failing_handler_does_not_block_healthy_handler() {
    let bus = EventBus::new();
    let ok_count = Arc::new(AtomicU64::new(0));
    let err_count = Arc::new(AtomicU64::new(0));

    let _h_ok = bus.subscribe(
        EventKind::ContentPublished,
        Arc::new(OkCounter {
            name: "ok".to_owned(),
            count: Arc::clone(&ok_count),
        }),
    );
    let _h_err = bus.subscribe(
        EventKind::ContentPublished,
        Arc::new(ErrCounter {
            name: "err".to_owned(),
            count: Arc::clone(&err_count),
        }),
    );

    for _ in 0..3 {
        bus.publish(Event::new(EventKind::ContentPublished));
    }

    wait_for(&ok_count, 3).await;
    wait_for(&err_count, 3).await;
}
