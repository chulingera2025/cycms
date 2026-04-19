use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use uuid::Uuid;

/// 内建事件类型（对应 Requirements 9.1）与插件自定义类型（`Custom`）的统一枚举。
///
/// 内建 variant 与它们对应的规范字符串由 [`EventKind::as_str`] 给出，格式固定为
/// `domain.action` 两段。插件可通过 [`EventKind::Custom`] 注册任意字符串，命名建议
/// 遵循 `plugin_name.action` 以降低冲突概率。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EventKind {
    ContentCreated,
    ContentUpdated,
    ContentDeleted,
    ContentPublished,
    ContentUnpublished,
    UserCreated,
    UserUpdated,
    UserDeleted,
    MediaUploaded,
    MediaDeleted,
    PluginInstalled,
    PluginEnabled,
    PluginDisabled,
    PluginUninstalled,
    /// 插件或业务自定义事件键，如 `"newsletter.subscribed"`。
    Custom(String),
}

impl EventKind {
    /// 返回事件类型的规范字符串表示。
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::ContentCreated => "content.created",
            Self::ContentUpdated => "content.updated",
            Self::ContentDeleted => "content.deleted",
            Self::ContentPublished => "content.published",
            Self::ContentUnpublished => "content.unpublished",
            Self::UserCreated => "user.created",
            Self::UserUpdated => "user.updated",
            Self::UserDeleted => "user.deleted",
            Self::MediaUploaded => "media.uploaded",
            Self::MediaDeleted => "media.deleted",
            Self::PluginInstalled => "plugin.installed",
            Self::PluginEnabled => "plugin.enabled",
            Self::PluginDisabled => "plugin.disabled",
            Self::PluginUninstalled => "plugin.uninstalled",
            Self::Custom(s) => s.as_str(),
        }
    }

    /// 判断该 `EventKind` 是否为系统内建类型（非 `Custom`）。
    #[must_use]
    pub fn is_builtin(&self) -> bool {
        !matches!(self, Self::Custom(_))
    }
}

impl fmt::Display for EventKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for EventKind {
    fn from(s: &str) -> Self {
        match s {
            "content.created" => Self::ContentCreated,
            "content.updated" => Self::ContentUpdated,
            "content.deleted" => Self::ContentDeleted,
            "content.published" => Self::ContentPublished,
            "content.unpublished" => Self::ContentUnpublished,
            "user.created" => Self::UserCreated,
            "user.updated" => Self::UserUpdated,
            "user.deleted" => Self::UserDeleted,
            "media.uploaded" => Self::MediaUploaded,
            "media.deleted" => Self::MediaDeleted,
            "plugin.installed" => Self::PluginInstalled,
            "plugin.enabled" => Self::PluginEnabled,
            "plugin.disabled" => Self::PluginDisabled,
            "plugin.uninstalled" => Self::PluginUninstalled,
            other => Self::Custom(other.to_owned()),
        }
    }
}

impl From<String> for EventKind {
    fn from(s: String) -> Self {
        Self::from(s.as_str())
    }
}

impl Serialize for EventKind {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for EventKind {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer).map(Self::from)
    }
}

/// 事件载荷。`kind` 决定 `payload` 的 schema，发布者保证一致性；EventBus 不做
/// schema 校验，只作为透明传输。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: Uuid,
    pub kind: EventKind,
    pub timestamp: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor_id: Option<String>,
    #[serde(default)]
    pub payload: serde_json::Value,
}

impl Event {
    /// 构造一条新事件（自动生成 id、当前时间戳、空 payload、匿名触发者）。
    #[must_use]
    pub fn new(kind: EventKind) -> Self {
        Self {
            id: Uuid::new_v4(),
            kind,
            timestamp: Utc::now(),
            actor_id: None,
            payload: serde_json::Value::Null,
        }
    }

    /// 附加触发者 ID，通常来自 `AuthClaims::sub`。
    #[must_use]
    pub fn with_actor(mut self, actor_id: impl Into<String>) -> Self {
        self.actor_id = Some(actor_id.into());
        self
    }

    /// 附加 JSON payload；调用方负责与 `kind` 对应的 schema 保持一致。
    #[must_use]
    pub fn with_payload(mut self, payload: serde_json::Value) -> Self {
        self.payload = payload;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::{Event, EventKind};
    use serde_json::json;

    const BUILTIN_CODES: &[&str] = &[
        "content.created",
        "content.updated",
        "content.deleted",
        "content.published",
        "content.unpublished",
        "user.created",
        "user.updated",
        "user.deleted",
        "media.uploaded",
        "media.deleted",
        "plugin.installed",
        "plugin.enabled",
        "plugin.disabled",
        "plugin.uninstalled",
    ];

    #[test]
    fn builtin_codes_round_trip_through_string() {
        for raw in BUILTIN_CODES {
            let kind: EventKind = (*raw).into();
            assert!(kind.is_builtin(), "{raw} should be recognised as builtin");
            assert_eq!(kind.as_str(), *raw);
            assert_eq!(format!("{kind}"), *raw);
        }
    }

    #[test]
    fn custom_kind_preserves_original_string() {
        let kind: EventKind = "newsletter.subscribed".into();
        assert!(!kind.is_builtin());
        assert_eq!(kind.as_str(), "newsletter.subscribed");
    }

    #[test]
    fn builtin_kind_serializes_as_dotted_string() {
        let kind = EventKind::ContentCreated;
        let json = serde_json::to_string(&kind).unwrap();
        assert_eq!(json, "\"content.created\"");

        let back: EventKind = serde_json::from_str(&json).unwrap();
        assert_eq!(back, EventKind::ContentCreated);
    }

    #[test]
    fn custom_kind_survives_round_trip() {
        let original = EventKind::Custom("plugin.my.thing".to_owned());
        let json = serde_json::to_string(&original).unwrap();
        let back: EventKind = serde_json::from_str(&json).unwrap();
        assert_eq!(back, original);
    }

    #[test]
    fn event_builder_sets_optional_fields() {
        let event = Event::new(EventKind::UserCreated)
            .with_actor("user-1")
            .with_payload(json!({"username": "alice"}));

        assert_eq!(event.kind, EventKind::UserCreated);
        assert_eq!(event.actor_id.as_deref(), Some("user-1"));
        assert_eq!(event.payload["username"], "alice");
    }

    #[test]
    fn event_serializes_with_expected_shape() {
        let event = Event::new(EventKind::MediaUploaded)
            .with_actor("admin")
            .with_payload(json!({"size": 1024}));

        let value = serde_json::to_value(&event).unwrap();
        assert_eq!(value["kind"], "media.uploaded");
        assert_eq!(value["actor_id"], "admin");
        assert_eq!(value["payload"]["size"], 1024);
    }
}
