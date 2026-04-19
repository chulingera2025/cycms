//! cycms-settings —— 系统与插件设置持久化层（任务 8）。
//!
//! 覆盖 Requirements 15.2 / 15.3：
//! - 键值以 `(namespace, key) -> JSON value` 形态存入 `settings` 表；
//! - 插件可注册 `plugin_settings_schemas`，供管理后台生成设置表单；
//! - 对外暴露 [`SettingsManager`]，封装三方言差异。
//!
//! 模块结构：
//! - [`error`]：crate 内部错误枚举与跨 crate 映射；
//! - [`model`]：`SettingEntry` / `PluginSchema` 数据模型；
//! - [`repository`]：`settings` / `plugin_settings_schemas` 表的三方言 CRUD；
//! - [`schema`]：注册时的 schema 格式校验；
//! - [`service`]：`SettingsManager` 门面。

mod error;
mod model;
mod repository;
mod schema;
mod service;

pub use error::SettingsError;
pub use repository::{PluginSchemaRepository, SettingsRepository};
pub use service::SettingsManager;
