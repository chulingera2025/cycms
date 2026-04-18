use std::sync::Arc;

use cycms_config::AuthConfig;
use cycms_core::Result;
use cycms_db::DatabasePool;
use serde::{Deserialize, Serialize};

use crate::error::AuthError;
use crate::password::{hash_password, verify_password};
use crate::revoked::RevokedTokenRepository;
use crate::seed::CreateUserInput;
use crate::token::{JwtCodec, TokenPair};
use crate::user::{NewUserRow, User, UserRepository};

/// 登录请求 DTO，字段固定为 username + password（v0.1 不支持 email 登录）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// 认证引擎：组合 `UserRepository` / `RevokedTokenRepository` / `JwtCodec` 提供登录与
/// token 校验能力，后续 5.5–5.7 的 refresh / admin / middleware 亦在此扩展。
pub struct AuthEngine {
    users: UserRepository,
    revoked: RevokedTokenRepository,
    jwt: JwtCodec,
    #[allow(dead_code)] // 任务 5.6 create_user 将使用
    argon2_cfg: cycms_config::Argon2Config,
    dummy_phc: String,
    db: Arc<DatabasePool>,
}

impl AuthEngine {
    /// 构造引擎并预热一份 dummy Argon2 哈希，用于登录路径的时间侧信道防御。
    ///
    /// # Errors
    /// Argon2 参数非法导致 dummy 哈希失败时返回 [`cycms_core::Error::Internal`]。
    pub fn new(db: Arc<DatabasePool>, cfg: AuthConfig) -> Result<Self> {
        let dummy_phc = hash_password(DUMMY_PASSWORD, &cfg.argon2)?;
        let users = UserRepository::new(Arc::clone(&db));
        let revoked = RevokedTokenRepository::new(Arc::clone(&db));
        let jwt = JwtCodec::new(
            &cfg.jwt_secret,
            cfg.access_token_ttl_secs,
            cfg.refresh_token_ttl_secs,
        );
        Ok(Self {
            users,
            revoked,
            jwt,
            argon2_cfg: cfg.argon2,
            dummy_phc,
            db,
        })
    }

    /// 返回内部 [`UserRepository`]（供任务 6 权限引擎与 CLI 使用）。
    #[must_use]
    pub fn users(&self) -> &UserRepository {
        &self.users
    }

    /// 返回内部 [`RevokedTokenRepository`]。
    #[must_use]
    pub fn revoked_tokens(&self) -> &RevokedTokenRepository {
        &self.revoked
    }

    /// 返回底层 [`DatabasePool`] 引用，后续任务若需共享同一池可从这里取。
    #[must_use]
    pub fn db(&self) -> &Arc<DatabasePool> {
        &self.db
    }

    /// 登录：验证凭证 → 颁发 `access/refresh` token 对。
    ///
    /// 登录失败不泄露"用户不存在"与"密码错误"之间的区别，统一返回
    /// [`cycms_core::Error::Unauthorized`]，并对不存在/禁用路径额外执行一次 dummy
    /// Argon2 校验以抵御时间侧信道。
    ///
    /// # Errors
    /// - 凭证不符、用户不存在、账户禁用 → [`cycms_core::Error::Unauthorized`]
    /// - DB / JWT 故障 → [`cycms_core::Error::Internal`]
    pub async fn login(&self, req: LoginRequest) -> Result<TokenPair> {
        match self.users.find_by_username(&req.username).await? {
            Some(user) if user.is_active => {
                if verify_password(&req.password, &user.password_hash)? {
                    let roles = self.users.fetch_roles(&user.id).await?;
                    let issued = self.jwt.issue_pair(&user.id, roles)?;
                    Ok(issued.pair)
                } else {
                    Err(AuthError::InvalidCredentials.into())
                }
            }
            _ => {
                // 无论用户不存在还是被禁用，都消耗一次 Argon2 verify 以统一响应时间
                let _ignore = verify_password(&req.password, &self.dummy_phc);
                Err(AuthError::InvalidCredentials.into())
            }
        }
    }

    /// 校验 access token 并返回 claims；同时检查 jti 是否在黑名单。
    ///
    /// # Errors
    /// - 解码失败 / 类型错配 / 已吊销 → [`cycms_core::Error::Unauthorized`]
    /// - DB 故障 → [`cycms_core::Error::Internal`]
    pub async fn verify_access(&self, token: &str) -> Result<crate::claims::AuthClaims> {
        let claims = self.jwt.decode(token, crate::claims::TokenType::Access)?;
        if self.revoked.is_revoked(&claims.jti).await? {
            return Err(AuthError::TokenRevoked.into());
        }
        Ok(claims)
    }

    /// 使用 refresh token 轮换 token 对：颁发新的 access+refresh，
    /// 并把旧 refresh 的 jti 写入 `revoked_tokens` 以阻止重放。
    ///
    /// 当旧 refresh 已经被吊销（即第二次复用）时直接 Unauthorized；
    /// 完整账号下线（`token_version` 方案）推迟到后续版本实现。
    ///
    /// # Errors
    /// - 解码失败 / 类型错配 / 已吊销 / 用户不存在或被禁用 → [`cycms_core::Error::Unauthorized`]
    /// - DB / JWT 故障 → [`cycms_core::Error::Internal`]
    pub async fn refresh(&self, refresh_token: &str) -> Result<TokenPair> {
        let claims = self
            .jwt
            .decode(refresh_token, crate::claims::TokenType::Refresh)?;

        if self.revoked.is_revoked(&claims.jti).await? {
            return Err(AuthError::TokenRevoked.into());
        }

        let user = self
            .users
            .find_by_id(&claims.sub)
            .await?
            .ok_or(AuthError::InvalidCredentials)?;
        if !user.is_active {
            return Err(AuthError::AccountDisabled.into());
        }

        let roles = self.users.fetch_roles(&user.id).await?;
        let issued = self.jwt.issue_pair(&user.id, roles)?;

        let old_exp = chrono::DateTime::<chrono::Utc>::from_timestamp(claims.exp, 0)
            .unwrap_or_else(chrono::Utc::now);
        self.revoked.revoke(&claims.jti, old_exp, "rotated").await?;

        Ok(issued.pair)
    }

    /// 创建新用户，执行 [`CreateUserInput::validate`] 后使用 Argon2id 哈希密码。
    ///
    /// # Errors
    /// - 字段校验 / 密码策略失败 → [`cycms_core::Error::ValidationError`]
    /// - username / email 冲突 → [`cycms_core::Error::Conflict`]
    /// - 其他 DB 或哈希错误 → [`cycms_core::Error::Internal`]
    pub async fn create_user(&self, input: CreateUserInput) -> Result<User> {
        input.validate()?;
        let phc = hash_password(&input.password, &self.argon2_cfg)?;
        self.users
            .create(NewUserRow {
                username: input.username,
                email: input.email,
                password_hash: phc,
                is_active: true,
            })
            .await
    }

    /// 当且仅当系统尚无任何用户时创建初始管理员。此方法**不**绑定角色，角色绑定
    /// 由调用方（任务 17 CLI `cycms seed admin`）在 `super_admin` 种子数据就绪后执行。
    ///
    /// # Errors
    /// - 已存在任意用户 → [`cycms_core::Error::Conflict`]
    /// - 输入校验 / 密码策略 / DB 错误 → 对应分类映射
    pub async fn setup_admin(&self, input: CreateUserInput) -> Result<User> {
        if self.users.count().await? > 0 {
            return Err(AuthError::AdminAlreadyExists.into());
        }
        self.create_user(input).await
    }
}

/// Dummy 明文，用于 [`AuthEngine::new`] 预热时间常数哈希。
const DUMMY_PASSWORD: &str = "__cycms_auth_dummy_password__";
