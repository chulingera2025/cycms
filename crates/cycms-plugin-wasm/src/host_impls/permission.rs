//! `permission` host 组：代理到 `PermissionEngine::check_permission`。
//!
//! v0.1 不向 guest 暴露 `owner_id`（scope=own 权限点永远不命中）；完全信任模型下
//! 插件如需 owner 语义，应自行判定后再以 scope=all 权限点询问运行时。

use crate::bindings::cycms::plugin::permission::Host;
use crate::host::HostState;

impl Host for HostState {
    async fn check(
        &mut self,
        user_id: String,
        domain: String,
        resource_name: String,
        action: String,
    ) -> wasmtime::Result<Result<bool, String>> {
        let roles = match self.auth.users().fetch_roles(&user_id).await {
            Ok(r) => r,
            Err(e) => return Ok(Err(format!("permission.check: fetch_roles: {e}"))),
        };
        let code = format!("{domain}.{resource_name}.{action}");
        match self
            .permissions
            .check_permission(&user_id, &roles, &code, None)
            .await
        {
            Ok(allowed) => Ok(Ok(allowed)),
            Err(e) => Ok(Err(format!("permission.check: {e}"))),
        }
    }
}
