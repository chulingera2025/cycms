//! `auth` host 组：代理到 `AuthEngine::verify_access` 与 `UserRepository::find_by_id`。
//!
//! `verify-access-token` 成功时返回 `user-id`；`get-user` 返回清洗过的 User JSON，
//! 剔除 `password_hash` 等敏感字段。完全信任模型下插件仍不需要直接访问密码哈希。

use serde_json::json;

use crate::bindings::cycms::plugin::auth::Host;
use crate::host::HostState;

impl Host for HostState {
    async fn verify_access_token(
        &mut self,
        token: String,
    ) -> wasmtime::Result<Result<String, String>> {
        match self.auth.verify_access(&token).await {
            Ok(claims) => Ok(Ok(claims.sub)),
            Err(e) => Ok(Err(format!("auth.verify_access_token: {e}"))),
        }
    }

    async fn get_user(&mut self, user_id: String) -> wasmtime::Result<Result<String, String>> {
        match self.auth.users().find_by_id(&user_id).await {
            Ok(Some(user)) => {
                let roles = self
                    .auth
                    .users()
                    .fetch_roles(&user.id)
                    .await
                    .unwrap_or_default();
                let sanitized = json!({
                    "id": user.id,
                    "username": user.username,
                    "email": user.email,
                    "is_active": user.is_active,
                    "roles": roles,
                    "created_at": user.created_at,
                    "updated_at": user.updated_at,
                });
                Ok(Ok(sanitized.to_string()))
            }
            Ok(None) => Ok(Err(format!("auth.get_user: user {user_id} not found"))),
            Err(e) => Ok(Err(format!("auth.get_user: {e}"))),
        }
    }
}
