//! 异步认证与权限模块（V3.0.0）
//!
//! 使用 sqlx 异步查询，tokio::sync 异步锁

use sqlx::{Executor, Row, Sqlite, SqlitePool};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::{Mutex, RwLock};

use crate::errors::{AppError, Result};

type PermissionCacheKey = (String, String, String);

static PERMISSION_CACHE: RwLock<Option<HashMap<PermissionCacheKey, PermissionAction>>> =
    RwLock::const_new(None);

pub async fn invalidate_permission_cache() {
    let mut cache = PERMISSION_CACHE.write().await;
    *cache = None;
    tracing::info!("[权限缓存] 已清除");
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Admin,
    Frontdesk,
    Maintenance,
    Finance,
}

impl Role {
    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "admin" => Ok(Role::Admin),
            "frontdesk" => Ok(Role::Frontdesk),
            "maintenance" => Ok(Role::Maintenance),
            "finance" => Ok(Role::Finance),
            _ => Err(AppError::InvalidInput(format!("未知角色: {}", s))),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Role::Admin => "admin",
            Role::Frontdesk => "frontdesk",
            Role::Maintenance => "maintenance",
            Role::Finance => "finance",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PermissionAction {
    Read,
    Write,
    Manage,
    None,
}

impl PermissionAction {
    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "read" => Ok(PermissionAction::Read),
            "write" => Ok(PermissionAction::Write),
            "manage" => Ok(PermissionAction::Manage),
            "none" => Ok(PermissionAction::None),
            _ => Err(AppError::InvalidInput(format!("未知权限动作: {}", s))),
        }
    }

    pub fn satisfies(&self, required: &PermissionAction) -> bool {
        match (self, required) {
            (PermissionAction::None, _) => false,
            (PermissionAction::Manage, _) => true,
            (PermissionAction::Write, PermissionAction::Read) => true,
            (PermissionAction::Write, PermissionAction::Write) => true,
            (PermissionAction::Write, PermissionAction::Manage) => false,
            (PermissionAction::Read, PermissionAction::Read) => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub password_hash: String,
    pub role: String,
    pub display_name: Option<String>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
    pub success: bool,
    pub token: Option<String>,
    pub user: Option<UserInfo>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserInfo {
    pub id: i64,
    pub username: String,
    pub role: String,
    pub display_name: Option<String>,
}

static TOKEN_STORE: std::sync::LazyLock<Mutex<std::collections::HashMap<String, User>>> =
    std::sync::LazyLock::new(|| Mutex::new(std::collections::HashMap::new()));

fn generate_token(user: &User) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{:x}-{}", timestamp, user.id)
}

fn verify_password(password: &str, hashed: &str) -> Result<bool> {
    bcrypt::verify(password, hashed).map_err(|e| {
        tracing::warn!("密码验证失败: {}", e);
        AppError::Authentication("密码验证失败".into())
    })
}

pub fn hash_password(password: &str) -> Result<String> {
    bcrypt::hash(password, bcrypt::DEFAULT_COST).map_err(|e| {
        tracing::error!("密码哈希生成失败: {}", e);
        AppError::Business("密码哈希生成失败".into())
    })
}

pub async fn login(pool: &SqlitePool, username: &str, password: &str) -> Result<LoginResponse> {
    let row = sqlx::query(
        "SELECT id, username, password_hash, role, display_name, is_active FROM users WHERE username = ? AND is_active = 1",
    )
    .bind(username)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Database(format!("login 查询失败: {}", e)))?;

    let row = match row {
        Some(r) => r,
        None => {
            return Ok(LoginResponse {
                success: false,
                token: None,
                user: None,
                message: Some("用户名或密码错误".into()),
            });
        }
    };

    let user = User {
        id: row.get("id"),
        username: row.get("username"),
        password_hash: row.get("password_hash"),
        role: row.get("role"),
        display_name: row.get("display_name"),
        is_active: row.get::<i32, _>("is_active") != 0,
    };

    let password_valid = verify_password(password, &user.password_hash)
        .map_err(|_| AppError::Authentication("用户名或密码错误".into()))?;

    if !password_valid {
        return Ok(LoginResponse {
            success: false,
            token: None,
            user: None,
            message: Some("用户名或密码错误".into()),
        });
    }

    let token = generate_token(&user);
    let user_clone = user.clone();
    TOKEN_STORE.lock().await.insert(token.clone(), user_clone);

    sqlx::query("UPDATE users SET last_login_at = datetime('now') WHERE id = ?")
        .bind(user.id)
        .execute(pool)
        .await
        .ok();

    Ok(LoginResponse {
        success: true,
        token: Some(token),
        user: Some(UserInfo {
            id: user.id,
            username: user.username,
            role: user.role.clone(),
            display_name: user.display_name,
        }),
        message: None,
    })
}

pub async fn get_user_by_token(token: &str) -> Option<User> {
    let store = TOKEN_STORE.lock().await;
    store.get(token).and_then(|u| {
        if u.is_active {
            Some(u.clone())
        } else {
            None
        }
    })
}

pub async fn logout(token: &str) {
    TOKEN_STORE.lock().await.remove(token);
}

#[derive(Debug, Clone)]
pub struct PermissionCheck {
    pub role: String,
    pub resource: String,
    pub permission: String,
    pub required_action: PermissionAction,
}

pub async fn check_permission<'e, E>(
    executor: E,
    role: &str,
    resource: &str,
    permission: &str,
    required_action: &PermissionAction,
) -> Result<bool>
where
    E: Executor<'e, Database = Sqlite> + Send + 'e,
{
    let cache_key = (role.to_string(), resource.to_string(), permission.to_string());

    {
        let cache = PERMISSION_CACHE.read().await;
        if let Some(ref map) = *cache {
            if let Some(cached_action) = map.get(&cache_key) {
                return Ok(cached_action.satisfies(required_action));
            }
        }
    }

    let row = sqlx::query(
        "SELECT action FROM permissions WHERE role = ? AND resource = ? AND permission = ?",
    )
    .bind(role)
    .bind(resource)
    .bind(permission)
    .fetch_optional(executor)
    .await
    .map_err(|e| AppError::Database(format!("check_permission 查询失败: {}", e)))?;

    let user_action = match row {
        Some(r) => {
            let action: String = r.get("action");
            PermissionAction::from_str(&action)?
        }
        None => PermissionAction::None,
    };

    {
        let mut cache = PERMISSION_CACHE.write().await;
        if cache.is_none() {
            *cache = Some(HashMap::new());
        }
        if let Some(ref mut map) = *cache {
            map.insert(cache_key, user_action.clone());
        }
    }

    Ok(user_action.satisfies(required_action))
}

#[macro_export]
macro_rules! require_permission_async {
    ($executor:expr, $role:expr, $resource:expr, $permission:expr, $action:expr) => {{
        use crate::auth_async::check_permission;
        use crate::errors::{AppError, Result};

        let required = $action;
        match check_permission($executor, $role, $resource, $permission, &required).await {
            Ok(true) => {}
            Ok(false) => {
                return Err(AppError::PermissionDenied(format!(
                    "权限不足: role={}, resource={}, permission={}, action={:?}",
                    $role, $resource, $permission, required
                )));
            }
            Err(e) => return Err(e),
        }
    }};
}

#[macro_export]
macro_rules! require_admin_async {
    ($token:expr) => {{
        use crate::auth_async::get_user_by_token;
        use crate::errors::{AppError, app_error_to_json_string};

        let user = match get_user_by_token(&$token).await {
            Some(u) => u,
            None => return Err(app_error_to_json_string(AppError::Authentication("请先登录".into()))),
        };
        if user.role != "admin" {
            return Err(app_error_to_json_string(AppError::PermissionDenied("仅管理员可执行此操作".into())));
        }
        user
    }};
}

#[macro_export]
macro_rules! require_login_async {
    ($token:expr) => {{
        use crate::auth_async::get_user_by_token;
        use crate::errors::{AppError, app_error_to_json_string};

        match get_user_by_token(&$token).await {
            Some(u) => u,
            None => return Err(app_error_to_json_string(AppError::Authentication("请先登录".into()))),
        }
    }};
}

pub async fn list_users(pool: &SqlitePool) -> Result<Vec<User>> {
    let rows = sqlx::query(
        "SELECT id, username, password_hash, role, display_name, is_active FROM users ORDER BY id",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Database(format!("list_users 查询失败: {}", e)))?;

    Ok(rows
        .iter()
        .map(|r| User {
            id: r.get("id"),
            username: r.get("username"),
            password_hash: r.get("password_hash"),
            role: r.get("role"),
            display_name: r.get("display_name"),
            is_active: r.get::<i32, _>("is_active") != 0,
        })
        .collect())
}

pub async fn create_user(
    pool: &SqlitePool,
    username: &str,
    password: &str,
    role: &str,
    display_name: Option<&str>,
) -> Result<i64> {
    let existing: i32 = sqlx::query("SELECT COUNT(*) as cnt FROM users WHERE username = ?")
        .bind(username)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(format!("create_user 查询失败: {}", e)))?
        .get("cnt");

    if existing > 0 {
        return Err(AppError::InvalidInput(format!(
            "用户名 '{}' 已存在",
            username
        )));
    }

    Role::from_str(role)?;

    let password_hash = hash_password(password)?;

    let result = sqlx::query(
        "INSERT INTO users (username, password_hash, role, display_name) VALUES (?, ?, ?, ?)",
    )
    .bind(username)
    .bind(password_hash)
    .bind(role)
    .bind(display_name)
    .execute(pool)
    .await
    .map_err(|e| AppError::Database(format!("create_user 失败: {}", e)))?;

    Ok(result.last_insert_rowid())
}

pub async fn update_user(
    pool: &SqlitePool,
    id: i64,
    role: Option<&str>,
    display_name: Option<&str>,
    is_active: Option<bool>,
) -> Result<bool> {
    if let Some(r) = role {
        Role::from_str(r)?;
    }

    let mut updates = Vec::new();
    if role.is_some() {
        updates.push(format!("role = '{}'", role.unwrap()));
    }
    if display_name.is_some() {
        updates.push(format!("display_name = '{}'", display_name.unwrap()));
    }
    if let Some(a) = is_active {
        updates.push(format!("is_active = {}", if a { 1 } else { 0 }));
    }

    if updates.is_empty() {
        return Ok(false);
    }

    updates.push("updated_at = datetime('now')".to_string());

    let sql = format!("UPDATE users SET {} WHERE id = {}", updates.join(", "), id);

    let affected = sqlx::query(&sql)
        .execute(pool)
        .await
        .map_err(|e| AppError::Database(format!("update_user 失败: {}", e)))?;

    Ok(affected.rows_affected() > 0)
}

pub async fn reset_password(pool: &SqlitePool, id: i64, new_password: &str) -> Result<bool> {
    let password_hash = hash_password(new_password)?;

    let affected = sqlx::query(
        "UPDATE users SET password_hash = ?, updated_at = datetime('now') WHERE id = ?",
    )
    .bind(password_hash)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| AppError::Database(format!("reset_password 失败: {}", e)))?;

    Ok(affected.rows_affected() > 0)
}

pub async fn delete_user(pool: &SqlitePool, id: i64) -> Result<bool> {
    let admin_count: i32 = sqlx::query(
        "SELECT COUNT(*) as cnt FROM users WHERE role = 'admin' AND is_active = 1",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Database(format!("delete_user 查询失败: {}", e)))?
    .get("cnt");

    let target_role: Option<String> = sqlx::query("SELECT role FROM users WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Database(format!("delete_user 查询角色失败: {}", e)))?
        .and_then(|r| r.get::<Option<String>, _>("role"));

    if target_role.as_deref() == Some("admin") && admin_count <= 1 {
        return Err(AppError::Business("无法删除最后一个管理员账户".into()));
    }

    let affected = sqlx::query("UPDATE users SET is_active = 0 WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| AppError::Database(format!("delete_user 失败: {}", e)))?;

    Ok(affected.rows_affected() > 0)
}
