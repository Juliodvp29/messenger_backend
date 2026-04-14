use chrono::{DateTime, Utc};
use domain::user::entity::User;
use domain::user::repository::UserRepository;
use domain::user::value_objects::{Email, PhoneNumber, UserId, Username};
use shared::error::{DomainError, DomainResult};
use sqlx::PgPool;
use uuid::Uuid;

pub struct PostgresUserRepository {
    pool: PgPool,
}

#[derive(Debug, Clone)]
pub struct UserSessionRecord {
    pub id: Uuid,
    pub device_name: String,
    pub device_type: String,
    pub ip_address: Option<String>,
    pub last_active_at: Option<DateTime<Utc>>,
}

impl PostgresUserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn upsert_session(
        &self,
        user_id: Uuid,
        device_id: &str,
        device_name: &str,
        device_type: &str,
        push_token: Option<&str>,
        expires_at: DateTime<Utc>,
    ) -> DomainResult<Uuid> {
        let session_id = Uuid::new_v4();

        let record = sqlx::query_as::<_, (Uuid,)>(
            r#"
            INSERT INTO user_sessions (id, user_id, device_id, device_name, device_type, push_token, expires_at)
            VALUES ($1, $2, $3, $4, $5::device_type, $6, $7)
            ON CONFLICT (user_id, device_id) DO UPDATE
            SET
                id = EXCLUDED.id,
                device_name = EXCLUDED.device_name,
                device_type = EXCLUDED.device_type,
                push_token = EXCLUDED.push_token,
                expires_at = EXCLUDED.expires_at,
                last_active_at = NOW()
            RETURNING id
            "#,
        )
        .bind(session_id)
        .bind(user_id)
        .bind(device_id)
        .bind(device_name)
        .bind(device_type)
        .bind(push_token)
        .bind(expires_at)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(record.0)
    }

    pub async fn list_sessions(&self, user_id: Uuid) -> DomainResult<Vec<UserSessionRecord>> {
        let records =
            sqlx::query_as::<_, (Uuid, String, String, Option<String>, Option<DateTime<Utc>>)>(
                r#"
            SELECT
                id,
                device_name,
                device_type::text,
                ip_address::text,
                last_active_at
            FROM user_sessions
            WHERE user_id = $1
            ORDER BY last_active_at DESC
            "#,
            )
            .bind(user_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(records
            .into_iter()
            .map(
                |(id, device_name, device_type, ip_address, last_active_at)| UserSessionRecord {
                    id,
                    device_name,
                    device_type,
                    ip_address,
                    last_active_at,
                },
            )
            .collect())
    }

    pub async fn delete_session(&self, user_id: Uuid, session_id: Uuid) -> DomainResult<bool> {
        let result = sqlx::query("DELETE FROM user_sessions WHERE id = $1 AND user_id = $2")
            .bind(session_id)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn is_session_valid(&self, user_id: Uuid, session_id: Uuid) -> DomainResult<bool> {
        let exists: Option<i32> = sqlx::query_scalar(
            r#"
            SELECT 1 FROM user_sessions 
            WHERE id = $1 AND user_id = $2 AND expires_at > NOW()
            "#,
        )
        .bind(session_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(exists.is_some())
    }
}

#[derive(sqlx::FromRow)]
struct UserRecord {
    id: Uuid,
    username: Option<String>,
    phone: String,
    email: Option<String>,
    status_text: Option<String>,
    two_fa_enabled: bool,
    two_fa_secret: Option<String>,
    is_active: bool,
    last_seen_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    deleted_at: Option<DateTime<Utc>>,
}

impl UserRepository for PostgresUserRepository {
    async fn create(&self, user: &User) -> DomainResult<()> {
        sqlx::query!(
            r#"
            INSERT INTO users (id, username, phone, email, status_text, two_fa_enabled, two_fa_secret, is_active, last_seen_at, created_at, updated_at, deleted_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            "#,
            user.id.0,
            user.username.as_ref().map(|u| u.as_str()),
            user.phone.as_str(),
            user.email.as_ref().map(|e| e.as_str()),
            user.status_text,
            user.two_fa_enabled,
            user.two_fa_secret,
            user.is_active,
            user.last_seen_at,
            user.created_at,
            user.updated_at,
            user.deleted_at
        )
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn find_by_id(&self, id: &UserId) -> DomainResult<Option<User>> {
        let record = sqlx::query_as!(
            UserRecord,
            r#"
            SELECT id, username, phone, email, status_text, two_fa_enabled, two_fa_secret, is_active, last_seen_at, created_at, updated_at, deleted_at 
            FROM users 
            WHERE id = $1 AND deleted_at IS NULL
            "#,
            id.0
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        record.map(map_record_to_user).transpose()
    }

    async fn find_by_phone(&self, phone: &PhoneNumber) -> DomainResult<Option<User>> {
        let record = sqlx::query_as!(
            UserRecord,
            r#"
            SELECT id, username, phone, email, status_text, two_fa_enabled, two_fa_secret, is_active, last_seen_at, created_at, updated_at, deleted_at 
            FROM users 
            WHERE phone = $1 AND deleted_at IS NULL
            "#,
            phone.as_str()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        record.map(map_record_to_user).transpose()
    }

    async fn find_by_username(&self, username: &Username) -> DomainResult<Option<User>> {
        let record = sqlx::query_as!(
            UserRecord,
            r#"
            SELECT id, username, phone, email, status_text, two_fa_enabled, two_fa_secret, is_active, last_seen_at, created_at, updated_at, deleted_at 
            FROM users 
            WHERE username = $1 AND deleted_at IS NULL
            "#,
            username.as_str()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        record.map(map_record_to_user).transpose()
    }

    async fn update(&self, user: &User) -> DomainResult<()> {
        sqlx::query!(
            r#"
            UPDATE users SET 
                username = $2,
                phone = $3,
                email = $4,
                status_text = $5,
                two_fa_enabled = $6,
                two_fa_secret = $7,
                is_active = $8,
                last_seen_at = $9,
                updated_at = NOW()
            WHERE id = $1 AND deleted_at IS NULL
            "#,
            user.id.0,
            user.username.as_ref().map(|u| u.as_str()),
            user.phone.as_str(),
            user.email.as_ref().map(|e| e.as_str()),
            user.status_text,
            user.two_fa_enabled,
            user.two_fa_secret,
            user.is_active,
            user.last_seen_at
        )
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn delete_soft(&self, id: &UserId) -> DomainResult<()> {
        sqlx::query!(
            r#"
            UPDATE users SET deleted_at = NOW() WHERE id = $1
            "#,
            id.0
        )
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn update_last_seen(&self, user_id: &UserId, timestamp: DateTime<Utc>) -> DomainResult<()> {
        sqlx::query(
            r#"
            UPDATE users SET last_seen_at = $2 WHERE id = $1 AND deleted_at IS NULL
            "#,
        )
        .bind(user_id.0)
        .bind(timestamp)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(())
    }
}

fn map_record_to_user(rec: UserRecord) -> DomainResult<User> {
    Ok(User {
        id: UserId(rec.id),
        username: rec.username.map(Username::new).transpose()?,
        phone: PhoneNumber::new(rec.phone)?,
        email: rec.email.map(Email::new).transpose()?,
        status_text: rec.status_text.unwrap_or_default(),
        two_fa_enabled: rec.two_fa_enabled,
        two_fa_secret: rec.two_fa_secret,
        is_active: rec.is_active,
        last_seen_at: rec.last_seen_at,
        created_at: rec.created_at,
        updated_at: rec.updated_at,
        deleted_at: rec.deleted_at,
    })
}
