use chrono::{DateTime, Utc};
use domain::user::entity::User;
use domain::user::repository::UserRepository;
use domain::user::value_objects::{Email, PasswordHash, PhoneNumber, UserId, Username};
use shared::error::{DomainError, DomainResult};
use sqlx::PgPool;
use uuid::Uuid;

pub struct PostgresUserRepository {
    pool: PgPool,
}

impl PostgresUserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct UserRecord {
    id: Uuid,
    username: Option<String>,
    phone: String,
    email: Option<String>,
    password_hash: String,
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
            INSERT INTO users (id, username, phone, email, password_hash, status_text, two_fa_enabled, two_fa_secret, is_active, last_seen_at, created_at, updated_at, deleted_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            "#,
            user.id.0,
            user.username.as_ref().map(|u| u.as_str()),
            user.phone.as_str(),
            user.email.as_ref().map(|e| e.as_str()),
            user.password_hash.0,
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
            SELECT id, username, phone, email, password_hash, status_text, two_fa_enabled, two_fa_secret, is_active, last_seen_at, created_at, updated_at, deleted_at 
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
            SELECT id, username, phone, email, password_hash, status_text, two_fa_enabled, two_fa_secret, is_active, last_seen_at, created_at, updated_at, deleted_at 
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
            SELECT id, username, phone, email, password_hash, status_text, two_fa_enabled, two_fa_secret, is_active, last_seen_at, created_at, updated_at, deleted_at 
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
                password_hash = $5,
                status_text = $6,
                two_fa_enabled = $7,
                two_fa_secret = $8,
                is_active = $9,
                last_seen_at = $10,
                updated_at = NOW()
            WHERE id = $1 AND deleted_at IS NULL
            "#,
            user.id.0,
            user.username.as_ref().map(|u| u.as_str()),
            user.phone.as_str(),
            user.email.as_ref().map(|e| e.as_str()),
            user.password_hash.0,
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
}

fn map_record_to_user(rec: UserRecord) -> DomainResult<User> {
    Ok(User {
        id: UserId(rec.id),
        username: rec.username.map(|u| Username::new(u)).transpose()?,
        phone: PhoneNumber::new(rec.phone)?,
        email: rec.email.map(|e| Email::new(e)).transpose()?,
        password_hash: PasswordHash(rec.password_hash),
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
