use domain::keys::{KeyRepository, OneTimePrekey, SignedPrekey, UserKeys};
use shared::error::DomainError;
use sqlx::PgPool;
use uuid::Uuid;

pub struct PostgresKeyRepository {
    pool: PgPool,
}

impl PostgresKeyRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

impl KeyRepository for PostgresKeyRepository {
    async fn upsert_keys(
        &self,
        user_id: Uuid,
        identity_key: &str,
        signed_prekey: &SignedPrekey,
    ) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            INSERT INTO user_keys (user_id, identity_key, signed_prekey, signed_prekey_id, signed_prekey_sig)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (user_id) DO UPDATE SET
                identity_key = EXCLUDED.identity_key,
                signed_prekey = EXCLUDED.signed_prekey,
                signed_prekey_id = EXCLUDED.signed_prekey_id,
                signed_prekey_sig = EXCLUDED.signed_prekey_sig,
                updated_at = NOW()
            "#,
            user_id,
            identity_key,
            signed_prekey.key,
            signed_prekey.id,
            signed_prekey.signature
        )
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn add_one_time_prekeys(
        &self,
        user_id: Uuid,
        prekeys: Vec<OneTimePrekey>,
    ) -> Result<(), DomainError> {
        for prekey in prekeys {
            sqlx::query!(
                r#"
                INSERT INTO one_time_prekeys (user_id, key_id, public_key)
                VALUES ($1, $2, $3)
                ON CONFLICT (user_id, key_id) DO NOTHING
                "#,
                user_id,
                prekey.id,
                prekey.key
            )
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Internal(e.to_string()))?;
        }

        Ok(())
    }

    async fn get_keys(&self, user_id: Uuid) -> Result<Option<UserKeys>, DomainError> {
        let row = sqlx::query_as!(
            DbUserKeys,
            r#"
            SELECT 
                user_id,
                identity_key,
                signed_prekey,
                signed_prekey_id,
                signed_prekey_sig,
                prekey_count
            FROM user_keys
            WHERE user_id = $1
            "#,
            user_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(row.map(|r| UserKeys {
            user_id: r.user_id,
            identity_key: r.identity_key,
            signed_prekey: SignedPrekey {
                id: r.signed_prekey_id,
                key: r.signed_prekey,
                signature: r.signed_prekey_sig,
            },
            signed_prekey_id: r.signed_prekey_id,
            prekey_count: r.prekey_count,
        }))
    }

    async fn get_prekey_count(&self, user_id: Uuid) -> Result<i32, DomainError> {
        let count = sqlx::query_scalar!(
            r#"
            SELECT prekey_count FROM user_keys WHERE user_id = $1
            "#,
            user_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(count.unwrap_or(0))
    }

    async fn is_blocked(&self, user_id: Uuid, target_user_id: Uuid) -> Result<bool, DomainError> {
        let blocked = sqlx::query_scalar!(
            r#"
            SELECT 1 FROM user_blocks 
            WHERE blocker_id = $1 AND blocked_id = $2
            "#,
            user_id,
            target_user_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(blocked.is_some())
    }

    async fn get_public_key_bundle(
        &self,
        target_user_id: Uuid,
    ) -> Result<Option<(String, String, i32, String, Option<i32>, Option<String>)>, DomainError>
    {
        let row = sqlx::query_as(
            r#"
            SELECT * FROM get_user_public_keys($1)
            "#,
        )
        .bind(target_user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(row.map(|r: (String, String, i32, String, Option<i32>, Option<String>)| r))
    }
}

#[derive(sqlx::FromRow)]
struct DbUserKeys {
    user_id: Uuid,
    identity_key: String,
    signed_prekey: String,
    signed_prekey_id: i32,
    signed_prekey_sig: String,
    prekey_count: i32,
}
