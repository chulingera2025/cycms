use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use chrono::Utc;
use cycms_config::AdminExtensionsConfig;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminExtensionSecurityState {
    pub csp_enabled: bool,
    pub csp_report_only: bool,
    pub csp_header_name: String,
    pub csp_policy: String,
    pub csp_report_uri: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminExtensionEventRecord {
    pub id: String,
    pub recorded_at: String,
    pub source: String,
    pub level: String,
    pub event_name: String,
    pub message: String,
    pub actor_id: Option<String>,
    pub request_id: Option<String>,
    pub plugin_name: Option<String>,
    pub contribution_id: Option<String>,
    pub contribution_kind: Option<String>,
    pub full_path: Option<String>,
    pub detail: Option<Value>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminExtensionDiagnosticsResponse {
    pub revision: String,
    pub diagnostics: Vec<cycms_plugin_manager::ExtensionDiagnostic>,
    pub recent_events: Vec<AdminExtensionEventRecord>,
    pub security: AdminExtensionSecurityState,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminExtensionClientEventPayload {
    pub source: String,
    pub level: String,
    pub event_name: String,
    pub message: String,
    pub plugin_name: Option<String>,
    pub contribution_id: Option<String>,
    pub contribution_kind: Option<String>,
    pub full_path: Option<String>,
    pub detail: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct AdminExtensionRecordedEvent {
    pub source: String,
    pub level: String,
    pub event_name: String,
    pub message: String,
    pub actor_id: Option<String>,
    pub request_id: Option<String>,
    pub plugin_name: Option<String>,
    pub contribution_id: Option<String>,
    pub contribution_kind: Option<String>,
    pub full_path: Option<String>,
    pub detail: Option<Value>,
}

impl AdminExtensionRecordedEvent {
    #[must_use]
    pub fn client(payload: AdminExtensionClientEventPayload) -> Self {
        Self {
            source: payload.source,
            level: payload.level,
            event_name: payload.event_name,
            message: payload.message,
            actor_id: None,
            request_id: None,
            plugin_name: payload.plugin_name,
            contribution_id: payload.contribution_id,
            contribution_kind: payload.contribution_kind,
            full_path: payload.full_path,
            detail: payload.detail,
        }
    }
}

#[derive(Debug)]
pub struct AdminExtensionEventStore {
    capacity: usize,
    next_id: AtomicU64,
    events: Mutex<VecDeque<AdminExtensionEventRecord>>,
}

impl AdminExtensionEventStore {
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            next_id: AtomicU64::new(1),
            events: Mutex::new(VecDeque::new()),
        }
    }

    pub async fn record(&self, event: AdminExtensionRecordedEvent) -> AdminExtensionEventRecord {
        let now = Utc::now();
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let record = AdminExtensionEventRecord {
            id: format!("admin-ext-event:{id}"),
            recorded_at: now.to_rfc3339(),
            source: event.source,
            level: event.level,
            event_name: event.event_name,
            message: event.message,
            actor_id: event.actor_id,
            request_id: event.request_id,
            plugin_name: event.plugin_name,
            contribution_id: event.contribution_id,
            contribution_kind: event.contribution_kind,
            full_path: event.full_path,
            detail: event.detail,
        };

        let mut events = self.events.lock().await;
        events.push_front(record.clone());
        while events.len() > self.capacity {
            let _ = events.pop_back();
        }

        record
    }

    pub async fn snapshot(&self) -> Vec<AdminExtensionEventRecord> {
        self.events.lock().await.iter().cloned().collect()
    }
}

#[must_use]
pub fn build_admin_extension_security_state(
    config: &AdminExtensionsConfig,
) -> AdminExtensionSecurityState {
    let report_uri =
        (!config.csp_report_uri.trim().is_empty()).then(|| config.csp_report_uri.clone());
    let csp_policy =
        build_admin_extension_csp_policy(config, report_uri.as_deref()).unwrap_or_default();
    AdminExtensionSecurityState {
        csp_enabled: config.csp_enabled,
        csp_report_only: config.csp_report_only,
        csp_header_name: if config.csp_report_only {
            "Content-Security-Policy-Report-Only".to_owned()
        } else {
            "Content-Security-Policy".to_owned()
        },
        csp_policy,
        csp_report_uri: report_uri,
    }
}

#[must_use]
pub fn build_admin_extension_csp_policy(
    config: &AdminExtensionsConfig,
    report_uri: Option<&str>,
) -> Option<String> {
    if !config.csp_enabled {
        return None;
    }

    let mut directives = vec![
        "default-src 'self'".to_owned(),
        "base-uri 'self'".to_owned(),
        "connect-src 'self'".to_owned(),
        "font-src 'self' data:".to_owned(),
        "form-action 'self'".to_owned(),
        "frame-ancestors 'self'".to_owned(),
        "img-src 'self' data: blob:".to_owned(),
        "media-src 'self' blob:".to_owned(),
        "object-src 'none'".to_owned(),
        "script-src 'self'".to_owned(),
        "style-src 'self' 'unsafe-inline'".to_owned(),
        "worker-src 'self' blob:".to_owned(),
    ];
    if let Some(report_uri) = report_uri.filter(|uri| !uri.trim().is_empty()) {
        directives.push(format!("report-uri {report_uri}"));
    }
    Some(directives.join("; "))
}

#[must_use]
pub fn with_request_context(
    mut event: AdminExtensionRecordedEvent,
    actor_id: Option<&str>,
    request_id: Option<&str>,
) -> AdminExtensionRecordedEvent {
    event.actor_id = actor_id.map(ToOwned::to_owned);
    event.request_id = request_id.map(ToOwned::to_owned);
    event
}

#[must_use]
pub fn build_csp_report_event(report: Value) -> AdminExtensionRecordedEvent {
    let document_uri = report
        .get("document-uri")
        .or_else(|| report.get("documentURL"))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let violated_directive = report
        .get("violated-directive")
        .or_else(|| report.get("effectiveDirective"))
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let blocked_uri = report
        .get("blocked-uri")
        .or_else(|| report.get("blockedURL"))
        .and_then(Value::as_str)
        .unwrap_or("unknown");

    AdminExtensionRecordedEvent {
        source: "csp".to_owned(),
        level: "warning".to_owned(),
        event_name: "csp.report".to_owned(),
        message: format!("检测到 CSP 违规：directive={violated_directive}, blocked={blocked_uri}"),
        actor_id: None,
        request_id: None,
        plugin_name: None,
        contribution_id: None,
        contribution_kind: None,
        full_path: document_uri,
        detail: Some(report),
    }
}

#[must_use]
pub fn normalize_csp_report_payload(body: Value) -> Vec<Value> {
    match body {
        Value::Object(mut object) => {
            if let Some(report) = object.remove("csp-report") {
                vec![report]
            } else {
                vec![Value::Object(object)]
            }
        }
        Value::Array(items) => items,
        other => vec![other],
    }
}

pub type SharedAdminExtensionEventStore = Arc<AdminExtensionEventStore>;

#[cfg(test)]
mod tests {
    use super::{
        AdminExtensionClientEventPayload, AdminExtensionEventStore, AdminExtensionRecordedEvent,
        build_admin_extension_csp_policy, build_admin_extension_security_state,
        build_csp_report_event, normalize_csp_report_payload,
    };
    use cycms_config::AdminExtensionsConfig;
    use serde_json::json;

    #[tokio::test]
    async fn event_store_keeps_latest_records_only() {
        let store = AdminExtensionEventStore::new(2);
        for index in 0..3 {
            let _ = store
                .record(AdminExtensionRecordedEvent::client(
                    AdminExtensionClientEventPayload {
                        source: "host".to_owned(),
                        level: "info".to_owned(),
                        event_name: format!("event-{index}"),
                        message: format!("message-{index}"),
                        plugin_name: None,
                        contribution_id: None,
                        contribution_kind: None,
                        full_path: None,
                        detail: None,
                    },
                ))
                .await;
        }

        let snapshot = store.snapshot().await;
        assert_eq!(snapshot.len(), 2);
        assert_eq!(snapshot[0].event_name, "event-2");
        assert_eq!(snapshot[1].event_name, "event-1");
    }

    #[test]
    fn csp_policy_uses_same_origin_defaults() {
        let config = AdminExtensionsConfig::default();
        let policy =
            build_admin_extension_csp_policy(&config, Some("/api/v1/admin/extensions/events"))
                .unwrap();
        assert!(policy.contains("default-src 'self'"));
        assert!(policy.contains("script-src 'self'"));
        assert!(policy.contains("report-uri /api/v1/admin/extensions/events"));
    }

    #[test]
    fn security_state_exposes_header_name_for_report_only() {
        let state = build_admin_extension_security_state(&AdminExtensionsConfig::default());
        assert_eq!(state.csp_header_name, "Content-Security-Policy-Report-Only");
    }

    #[test]
    fn normalizes_csp_report_payloads() {
        let payload = normalize_csp_report_payload(json!({
            "csp-report": { "document-uri": "http://localhost/admin" }
        }));
        assert_eq!(payload.len(), 1);
        let event = build_csp_report_event(payload[0].clone());
        assert_eq!(event.event_name, "csp.report");
        assert_eq!(event.full_path.as_deref(), Some("http://localhost/admin"));
    }
}
