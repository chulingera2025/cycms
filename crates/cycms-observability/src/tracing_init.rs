use cycms_config::{LogFormat, ObservabilityConfig};
use cycms_core::{Error, Result};
use tracing::Dispatch;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt;
use tracing_subscriber::fmt::writer::MakeWriter;
use tracing_subscriber::layer::SubscriberExt;

/// 根据配置初始化全局 tracing dispatcher。
///
/// # Errors
/// 当日志级别过滤器无效时返回错误。
pub fn init_tracing(config: &ObservabilityConfig) -> Result<()> {
    let dispatch = build_dispatch(config, std::io::stdout)?;
    let _ = tracing::dispatcher::set_global_default(dispatch);
    Ok(())
}

fn build_dispatch<W>(config: &ObservabilityConfig, writer: W) -> Result<Dispatch>
where
    W: for<'writer> MakeWriter<'writer> + Send + Sync + 'static,
{
    let level = config.level.trim();
    let filter = EnvFilter::try_new(level).map_err(|source| Error::BadRequest {
        message: format!("invalid observability level/filter: {level}"),
        source: Some(Box::new(source)),
    })?;

    let dispatch = match config.format {
        LogFormat::Json => Dispatch::new(
            tracing_subscriber::registry().with(filter).with(
                fmt::layer()
                    .json()
                    .flatten_event(true)
                    .with_current_span(true)
                    .with_span_list(true)
                    .with_target(true)
                    .with_thread_ids(true)
                    .with_thread_names(true)
                    .with_writer(writer),
            ),
        ),
        LogFormat::Pretty => Dispatch::new(
            tracing_subscriber::registry().with(filter).with(
                fmt::layer()
                    .pretty()
                    .with_target(true)
                    .with_thread_ids(true)
                    .with_thread_names(true)
                    .with_writer(writer),
            ),
        ),
    };

    Ok(dispatch)
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::io::Write;
    use std::sync::{Arc, Mutex};

    use super::build_dispatch;
    use cycms_config::{LogFormat, ObservabilityConfig};
    use tracing::info;
    use tracing_subscriber::fmt::writer::MakeWriter;

    #[derive(Clone, Default)]
    struct SharedWriter {
        buffer: Arc<Mutex<Vec<u8>>>,
    }

    struct VecWriter {
        buffer: Arc<Mutex<Vec<u8>>>,
    }

    impl<'writer> MakeWriter<'writer> for SharedWriter {
        type Writer = VecWriter;

        fn make_writer(&'writer self) -> Self::Writer {
            VecWriter {
                buffer: Arc::clone(&self.buffer),
            }
        }
    }

    impl Write for VecWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.buffer.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    fn render_log(config: &ObservabilityConfig) -> String {
        let writer = SharedWriter::default();
        let dispatch = build_dispatch(config, writer.clone()).unwrap();

        tracing::dispatcher::with_default(&dispatch, || {
            info!(answer = 42, component = "test", "hello observability");
        });

        String::from_utf8(writer.buffer.lock().unwrap().clone()).unwrap()
    }

    #[test]
    fn json_format_emits_structured_output() {
        let output = render_log(&ObservabilityConfig {
            format: LogFormat::Json,
            level: "info".to_owned(),
            audit_enabled: true,
        });

        assert!(output.contains("\"message\":\"hello observability\""));
        assert!(output.contains("\"answer\":42"));
        assert!(output.contains("\"component\":\"test\""));
    }

    #[test]
    fn pretty_format_emits_human_readable_output() {
        let output = render_log(&ObservabilityConfig {
            format: LogFormat::Pretty,
            level: "info".to_owned(),
            audit_enabled: true,
        });

        assert!(output.contains("hello observability"));
        assert!(output.contains("INFO"));
        assert!(output.contains("component"));
    }

    #[test]
    fn invalid_filter_returns_bad_request() {
        let error = build_dispatch(
            &ObservabilityConfig {
                format: LogFormat::Pretty,
                level: "info,[broken".to_owned(),
                audit_enabled: true,
            },
            SharedWriter::default(),
        )
        .unwrap_err();

        assert!(matches!(error, cycms_core::Error::BadRequest { .. }));
    }
}
