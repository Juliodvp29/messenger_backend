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

#[cfg(test)]
mod tests {
    use super::*;
    use domain::user::entity::User;
    use sqlx::PgPool;

    #[tokio::test]
    async fn test_user_repository_crud() {
        // En un entorno real usaríamos una BD de test o transacciones.
        // Aquí conectamos a la de Docker si está disponible.
        let db_url = "postgresql://messenger:messenger_secret@localhost:5434/messenger_dev";
        let pool = PgPool::connect(db_url).await.expect("Failed to connect to test DB");

        let repo = PostgresUserRepository::new(pool);
        let user_id = UserId(Uuid::new_v4());
        let phone = PhoneNumber::new("+51999888777".to_string()).unwrap();

        let user = User {
            id: user_id.clone(),
            username: Some(Username::new("testuser".to_string()).unwrap()),
            phone: phone.clone(),
            email: Some(Email::new("test@example.com".to_string()).unwrap()),
            password_hash: PasswordHash("hashed_pass".to_string()),
            status_text: "Hello".to_string(),
            two_fa_enabled: false,
            two_fa_secret: None,
            is_active: true,
            last_seen_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: None,
        };

        // 1. Create
        repo.create(&user).await.expect("Failed to create user");

        // 2. Find by ID
        let found = repo.find_by_id(&user_id).await.expect("Failed to find user");
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.phone.as_str(), "+51999888777");

        // 3. Find by Phone
        let found_phone = repo.find_by_phone(&phone).await.expect("Failed to find by phone");
        assert!(found_phone.is_some());

        // 4. Update
        let mut updated_user = found;
        updated_user.status_text = "Updated".to_string();
        repo.update(&updated_user).await.expect("Failed to update user");

        let found_updated = repo.find_by_id(&user_id).await.expect("Failed to fetch updated");
        assert_eq!(found_updated.unwrap().status_text, "Updated");

        // 5. Delete
        repo.delete_soft(&user_id).await.expect("Failed to delete user");
        let found_deleted = repo.find_by_id(&user_id).await.expect("Failed to check deletion");
        assert!(found_deleted.is_none());
    }
}
