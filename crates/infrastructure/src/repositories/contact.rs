use chrono::{DateTime, Utc};
use domain::contact::entity::Contact;
use domain::contact::repository::ContactRepository;
use domain::contact::value_objects::{ContactId, ContactPhoneNumber};
use domain::user::value_objects::{PhoneNumber, UserId};
use shared::error::{DomainError, DomainResult};
use sqlx::PgPool;

pub struct PostgresContactRepository {
    pool: PgPool,
}

impl PostgresContactRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct ContactRecord {
    id: Uuid,
    owner_id: Uuid,
    contact_id: Option<Uuid>,
    phone: String,
    nickname: Option<String>,
    is_favorite: bool,
    created_at: DateTime<Utc>,
}

impl ContactRepository for PostgresContactRepository {
    async fn create(&self, contact: &Contact) -> DomainResult<()> {
        sqlx::query!(
            r#"
            INSERT INTO contacts (id, owner_id, contact_id, phone, nickname, is_favorite, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
            contact.id.0,
            contact.owner_id.0,
            contact.contact_id.as_ref().map(|c| c.0),
            contact.phone.as_str(),
            contact.nickname,
            contact.is_favorite,
            contact.created_at
        )
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn find_by_id(&self, id: &ContactId) -> DomainResult<Option<Contact>> {
        let record = sqlx::query_as!(
            ContactRecord,
            r#"
            SELECT id, owner_id, contact_id, phone, nickname, is_favorite, created_at
            FROM contacts
            WHERE id = $1
            "#,
            id.0
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        record.map(map_record_to_contact).transpose()
    }

    async fn find_by_owner_and_phone(&self, owner_id: &UserId, phone: &ContactPhoneNumber) -> DomainResult<Option<Contact>> {
        let record = sqlx::query_as!(
            ContactRecord,
            r#"
            SELECT id, owner_id, contact_id, phone, nickname, is_favorite, created_at
            FROM contacts
            WHERE owner_id = $1 AND phone = $2
            "#,
            owner_id.0,
            phone.as_str()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        record.map(map_record_to_contact).transpose()
    }

    async fn find_all_by_owner(&self, owner_id: &UserId) -> DomainResult<Vec<Contact>> {
        let records = sqlx::query_as!(
            ContactRecord,
            r#"
            SELECT id, owner_id, contact_id, phone, nickname, is_favorite, created_at
            FROM contacts
            WHERE owner_id = $1
            ORDER BY nickname ASC, phone ASC
            "#,
            owner_id.0
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        records.into_iter().map(map_record_to_contact).collect()
    }

    async fn find_favorites(&self, owner_id: &UserId) -> DomainResult<Vec<Contact>> {
        let records = sqlx::query_as!(
            ContactRecord,
            r#"
            SELECT id, owner_id, contact_id, phone, nickname, is_favorite, created_at
            FROM contacts
            WHERE owner_id = $1 AND is_favorite = TRUE
            ORDER BY nickname ASC, phone ASC
            "#,
            owner_id.0
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        records.into_iter().map(map_record_to_contact).collect()
    }

    async fn update(&self, contact: &Contact) -> DomainResult<()> {
        sqlx::query!(
            r#"
            UPDATE contacts SET 
                contact_id = $2,
                phone = $3,
                nickname = $4,
                is_favorite = $5
            WHERE id = $1
            "#,
            contact.id.0,
            contact.contact_id.as_ref().map(|c| c.0),
            contact.phone.as_str(),
            contact.nickname,
            contact.is_favorite
        )
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn delete(&self, id: &ContactId) -> DomainResult<()> {
        sqlx::query!(
            r#"
            DELETE FROM contacts WHERE id = $1
            "#,
            id.0
        )
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn delete_by_owner(&self, owner_id: &UserId) -> DomainResult<()> {
        sqlx::query!(
            r#"
            DELETE FROM contacts WHERE owner_id = $1
            "#,
            owner_id.0
        )
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(())
    }
}

fn map_record_to_contact(rec: ContactRecord) -> DomainResult<Contact> {
    Ok(Contact {
        id: ContactId(rec.id),
        owner_id: UserId(rec.owner_id),
        contact_id: rec.contact_id.map(UserId),
        phone: PhoneNumber::new(rec.phone)?,
        nickname: rec.nickname,
        is_favorite: rec.is_favorite,
        created_at: rec.created_at,
    })
}

use uuid::Uuid;