use cycms_core::Error;
use thiserror::Error;

/// cycms-events 内部错误。统一通过 `From<EventError> for cycms_core::Error` 映射到
/// [`Error::Internal`]，供调用方用 `?` 透传到统一错误响应。
///
/// `EventBus` 的发布 / 订阅接口在 v0.1 下大多为 infallible（`NoReceivers` 会被吞掉
/// 而非抛错），因此本枚举主要被插件 handler 与未来的持久化/远程事件扩展使用。
#[derive(Debug, Error)]
pub enum EventError {
    /// 事件总线已关闭或处于不可用状态。
    #[error("event bus is closed")]
    BusClosed,

    /// `EventHandler::handle` 返回错误的消息包装，供 tracing / 上层日志使用。
    #[error("event handler `{handler}` failed: {message}")]
    HandlerFailed { handler: String, message: String },

    /// 事件分发过程中遇到非预期错误（例如 broadcast 发送异常且并非 `NoReceivers`）。
    #[error("event dispatch error: {0}")]
    Dispatch(String),
}

impl From<EventError> for Error {
    fn from(err: EventError) -> Self {
        let message = err.to_string();
        Self::Internal {
            message,
            source: Some(Box::new(err)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::EventError;
    use cycms_core::Error;

    #[test]
    fn event_error_maps_to_internal() {
        let err: Error = EventError::BusClosed.into();
        assert!(matches!(err, Error::Internal { .. }));
        assert!(err.to_string().contains("event bus is closed"));
    }

    #[test]
    fn handler_failed_preserves_context() {
        let err = EventError::HandlerFailed {
            handler: "audit".to_owned(),
            message: "db offline".to_owned(),
        };
        let rendered = err.to_string();
        assert!(rendered.contains("audit"));
        assert!(rendered.contains("db offline"));
    }
}
