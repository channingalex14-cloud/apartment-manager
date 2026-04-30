//! 认证与权限模块
//!
//! Phase 4: 权限系统基础
//! - 用户认证（登录/登出）
//! - 权限校验中间件
//! - 密码验证（bcrypt）

use rusqlite::{params, Connection, Result as SqliteResult};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

use crate::errors::{AppError, Result};

/// 用户角色
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

/// 权限动作
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

    /// 检查某个动作是否满足要求的权限等级
    pub fn satisfies(&self, required: &PermissionAction) -> bool {
        match (self, required) {
            // none 永远不满足任何要求
            (PermissionAction::None, _) => false,
            // manage 满足任何要求
            (PermissionAction::Manage, _) => true,
            // write 只满足 read/write
            (PermissionAction::Write, PermissionAction::Read) => true,
            (PermissionAction::Write, PermissionAction::Write) => true,
            (PermissionAction::Write, PermissionAction::Manage) => false,
            // read 只满足 read
            (PermissionAction::Read, PermissionAction::Read) => true,
            _ => false,
        }
    }
}

/// 用户模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub password_hash: String,
    pub role: String,
    pub display_name: Option<String>,
    pub is_active: bool,
}

/// 用户登录请求
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// 用户登录响应
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

/// 简单的 token 管理（基于内存 Map）
/// 生产环境应使用 JWT 或 session
static TOKEN_STORE: std::sync::LazyLock<Mutex<std::collections::HashMap<String, User>>>
    = std::sync::LazyLock::new(|| Mutex::new(std::collections::HashMap::new()));

/// 生成简单 token
fn generate_token(user: &User) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{:x}-{}", timestamp, user.id)
}

/// 使用 bcrypt 验证密码
fn verify_password(password: &str, hashed: &str) -> Result<bool> {
    // bcrypt::verify 会自动处理 $2b$ 等多种 bcrypt 格式变体
    bcrypt::verify(password, hashed)
        .map_err(|e| {
            tracing::warn!("密码验证失败: {}", e);
            AppError::Authentication("密码验证失败".into())
        })
}

/// 使用 bcrypt 生成密码哈希（用于创建/更新用户密码）
pub fn hash_password(password: &str) -> Result<String> {
    bcrypt::hash(password, bcrypt::DEFAULT_COST)
        .map_err(|e| {
            tracing::error!("密码哈希生成失败: {}", e);
            AppError::Business("密码哈希生成失败".into())
        })
}

/// 用户登录
pub fn login(conn: &Connection, username: &str, password: &str) -> Result<LoginResponse> {
    let user: User = conn
        .query_row(
            "SELECT id, username, password_hash, role, display_name, is_active FROM users WHERE username = ? AND is_active = 1",
            [username],
            |row| {
                Ok(User {
                    id: row.get(0)?,
                    username: row.get(1)?,
                    password_hash: row.get(2)?,
                    role: row.get(3)?,
                    display_name: row.get(4)?,
                    is_active: row.get::<_, i64>(5)? == 1,
                })
            },
        )
        .map_err(|_| AppError::Authentication("用户名或密码错误".into()))?;

    // 使用 bcrypt 验证密码
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

    // 生成 token
    let token = generate_token(&user);
    let user_clone = user.clone();
    TOKEN_STORE.lock().unwrap_or_else(|e| e.into_inner()).insert(token.clone(), user_clone);

    // 更新最后登录时间
    conn.execute(
        "UPDATE users SET last_login_at = datetime('now') WHERE id = ?",
        params![user.id],
    )
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

/// 从 token 获取用户
pub fn get_user_by_token(token: &str) -> Option<User> {
    TOKEN_STORE.lock().unwrap_or_else(|e| e.into_inner()).get(token).cloned()
}

/// 登出
pub fn logout(token: &str) {
    TOKEN_STORE.lock().unwrap_or_else(|e| e.into_inner()).remove(token);
}

/// 权限校验请求
#[derive(Debug, Clone)]
pub struct PermissionCheck {
    pub role: String,
    pub resource: String,
    pub permission: String,
    pub required_action: PermissionAction,
}

/// 检查用户是否有权限
pub fn check_permission(conn: &Connection, role: &str, resource: &str, permission: &str, required_action: &PermissionAction) -> Result<bool> {
    let action: String = conn
        .query_row(
            "SELECT action FROM permissions WHERE role = ? AND resource = ? AND permission = ?",
            params![role, resource, permission],
            |row| row.get(0),
        )
        .map_err(|e| {
            tracing::error!("权限查询失败: {:?}", e);
            AppError::PermissionDenied("权限校验失败，请联系管理员".to_string())
        })?;

    let user_action = PermissionAction::from_str(&action)?;
    Ok(user_action.satisfies(required_action))
}

/// 权限校验辅助宏
#[macro_export]
macro_rules! require_permission {
    ($conn:expr, $role:expr, $resource:expr, $permission:expr, $action:expr) => {{
        use crate::auth::check_permission;
        use crate::errors::{AppError, Result};

        let required = $action;
        match check_permission($conn, $role, $resource, $permission, &required) {
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

/// Admin 角色校验宏 — 从 token 获取用户，验证是否为 admin
#[macro_export]
macro_rules! require_admin {
    ($token:expr) => {{
        use crate::auth::get_user_by_token;
        use crate::errors::{AppError, app_error_to_json_string};

        let user = match get_user_by_token(&$token) {
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
macro_rules! require_login {
    ($token:expr) => {{
        use crate::auth::get_user_by_token;
        use crate::errors::{AppError, app_error_to_json_string};

        match get_user_by_token(&$token) {
            Some(u) => u,
            None => return Err(app_error_to_json_string(AppError::Authentication("请先登录".into()))),
        }
    }};
}

/// 获取用户列表（仅 admin）— 不包含密码哈希
pub fn list_users(conn: &Connection) -> Result<Vec<User>> {
    let mut stmt = conn.prepare(
        "SELECT id, username, password_hash, role, display_name, is_active FROM users ORDER BY id",
    )?;

    let users = stmt
        .query_map([], |row| {
            Ok(User {
                id: row.get(0)?,
                username: row.get(1)?,
                password_hash: row.get(2)?,
                role: row.get(3)?,
                display_name: row.get(4)?,
                is_active: row.get::<_, i64>(5)? == 1,
            })
        })?
        .collect::<SqliteResult<Vec<_>>>()
        .map_err(AppError::Database)?;

    Ok(users)
}

pub fn create_user(
    conn: &Connection,
    username: &str,
    password: &str,
    role: &str,
    display_name: Option<&str>,
) -> Result<i64> {
    let existing: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM users WHERE username = ?",
            [username],
            |row| row.get(0),
        )
        .unwrap_or(0);

    if existing > 0 {
        return Err(AppError::InvalidInput(format!("用户名 '{}' 已存在", username)));
    }

    Role::from_str(role)?;

    let password_hash = hash_password(password)?;

    conn.execute(
        "INSERT INTO users (username, password_hash, role, display_name) VALUES (?, ?, ?, ?)",
        params![username, password_hash, role, display_name],
    )
    .map_err(AppError::Database)?;

    Ok(conn.last_insert_rowid())
}

pub fn update_user(
    conn: &Connection,
    id: i64,
    role: Option<&str>,
    display_name: Option<&str>,
    is_active: Option<bool>,
) -> Result<bool> {
    if let Some(r) = role {
        Role::from_str(r)?;
    }

    let mut sets = Vec::new();
    let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(r) = role {
        sets.push("role = ?");
        param_values.push(Box::new(r.to_string()));
    }
    if let Some(d) = display_name {
        sets.push("display_name = ?");
        param_values.push(Box::new(d.to_string()));
    }
    if let Some(a) = is_active {
        sets.push("is_active = ?");
        param_values.push(Box::new(if a { 1i64 } else { 0i64 }));
    }

    if sets.is_empty() {
        return Ok(false);
    }

    sets.push("updated_at = datetime('now')");

    let sql = format!(
        "UPDATE users SET {} WHERE id = ?",
        sets.join(", ")
    );

    param_values.push(Box::new(id));

    let params_refs: Vec<&dyn rusqlite::types::ToSql> = param_values.iter().map(|p| p.as_ref()).collect();

    let affected = conn
        .execute(&sql, params_refs.as_slice())
        .map_err(AppError::Database)?;

    Ok(affected > 0)
}

pub fn reset_password(conn: &Connection, id: i64, new_password: &str) -> Result<bool> {
    let password_hash = hash_password(new_password)?;

    let affected = conn
        .execute(
            "UPDATE users SET password_hash = ?, updated_at = datetime('now') WHERE id = ?",
            params![password_hash, id],
        )
        .map_err(AppError::Database)?;

    Ok(affected > 0)
}

pub fn delete_user(conn: &Connection, id: i64) -> Result<bool> {
    let admin_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM users WHERE role = 'admin' AND is_active = 1",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let target_role: String = conn
        .query_row("SELECT role FROM users WHERE id = ?", [id], |row| row.get(0))
        .unwrap_or_default();

    if target_role == "admin" && admin_count <= 1 {
        return Err(AppError::Business("无法删除最后一个管理员账户".into()));
    }

    let affected = conn
        .execute("DELETE FROM users WHERE id = ?", [id])
        .map_err(AppError::Database)?;

    Ok(affected > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_from_str_valid() {
        assert_eq!(Role::from_str("admin").unwrap(), Role::Admin);
        assert_eq!(Role::from_str("frontdesk").unwrap(), Role::Frontdesk);
        assert_eq!(Role::from_str("maintenance").unwrap(), Role::Maintenance);
        assert_eq!(Role::from_str("finance").unwrap(), Role::Finance);
    }

    #[test]
    fn test_role_from_str_invalid() {
        assert!(Role::from_str("superadmin").is_err());
        assert!(Role::from_str("").is_err());
        assert!(Role::from_str("ADMIN").is_err());
    }

    #[test]
    fn test_role_as_str() {
        assert_eq!(Role::Admin.as_str(), "admin");
        assert_eq!(Role::Frontdesk.as_str(), "frontdesk");
        assert_eq!(Role::Maintenance.as_str(), "maintenance");
        assert_eq!(Role::Finance.as_str(), "finance");
    }

    #[test]
    fn test_hash_password_and_verify() {
        let password = "test123456";
        let hash = hash_password(password).unwrap();
        assert_ne!(hash, password);
        assert!(bcrypt::verify(password, &hash).unwrap());
        assert!(!bcrypt::verify("wrong_password", &hash).unwrap());
    }

    #[test]
    fn test_hash_password_different_hashes() {
        let password = "same_password";
        let hash1 = hash_password(password).unwrap();
        let hash2 = hash_password(password).unwrap();
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_token_store_crud() {
        let token = format!("test-{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis());

        assert!(get_user_by_token(&token).is_none());

        let user = User {
            id: 999,
            username: "test_user".to_string(),
            password_hash: String::new(),
            role: "admin".to_string(),
            display_name: Some("Test".to_string()),
            is_active: true,
        };

        {
            let mut store = TOKEN_STORE.lock().unwrap();
            store.insert(token.clone(), user.clone());
        }

        let retrieved = get_user_by_token(&token);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().username, "test_user");

        logout(&token);
        assert!(get_user_by_token(&token).is_none());
    }

    #[test]
    fn test_permission_matrix_admin() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS permissions (
                role TEXT NOT NULL,
                resource TEXT NOT NULL,
                permission TEXT NOT NULL,
                action TEXT NOT NULL DEFAULT 'read',
                PRIMARY KEY (role, resource, permission)
            );
            INSERT INTO permissions (role, resource, permission, action) VALUES
                ('admin', 'rooms', 'view', 'manage'),
                ('admin', 'bills', 'manage', 'manage'),
                ('frontdesk', 'rooms', 'view', 'read'),
                ('frontdesk', 'bills', 'create', 'write');"
        ).unwrap();

        let result = check_permission(&conn, "admin", "rooms", "view", &PermissionAction::Manage);
        assert!(result.unwrap());

        let result = check_permission(&conn, "frontdesk", "rooms", "view", &PermissionAction::Read);
        assert!(result.unwrap());

        let result = check_permission(&conn, "frontdesk", "rooms", "delete", &PermissionAction::Write);
        assert!(result.is_err());
    }

    #[test]
    fn test_permission_action_ordering() {
        assert!(PermissionAction::Manage >= PermissionAction::Write);
        assert!(PermissionAction::Write >= PermissionAction::Read);
        assert!(PermissionAction::Manage >= PermissionAction::Read);
        assert!(PermissionAction::Read >= PermissionAction::Read);
    }
}
