use redis::AsyncCommands;
use redis::aio::ConnectionManager;

#[derive(Clone)]
pub struct OtpService {
    pub redis: ConnectionManager,
    otp_ttl: u64,
}

impl OtpService {
    pub fn new(redis: ConnectionManager, otp_ttl: u64) -> Self {
        Self { redis, otp_ttl }
    }

    pub fn generate() -> String {
        let code = rand::random::<u32>() % 1_000_000;
        format!("{:06}", code)
    }

    pub async fn store_register_otp(
        &self,
        phone: &str,
        code: &str,
    ) -> Result<(), redis::RedisError> {
        let mut con = self.redis.clone();
        let key = format!("otp:register:{}", phone);
        let _: () = con.set_ex(key, code, self.otp_ttl).await?;
        Ok(())
    }

    pub async fn store_login_otp(&self, phone: &str, code: &str) -> Result<(), redis::RedisError> {
        let mut con = self.redis.clone();
        let key = format!("otp:login:{}", phone);
        let _: () = con.set_ex(key, code, self.otp_ttl).await?;
        Ok(())
    }

    pub async fn store_recover_otp(
        &self,
        phone: &str,
        code: &str,
    ) -> Result<(), redis::RedisError> {
        let mut con = self.redis.clone();
        let key = format!("otp:recover:{}", phone);
        let _: () = con.set_ex(key, code, self.otp_ttl).await?;
        Ok(())
    }

    pub async fn verify_register_otp(
        &self,
        phone: &str,
        code: &str,
    ) -> Result<bool, redis::RedisError> {
        let mut con = self.redis.clone();
        let key = format!("otp:register:{}", phone);
        let stored: Option<String> = con.get(&key).await?;

        if let Some(stored_code) = stored
            && stored_code == code
        {
            let _: usize = con.del(&key).await?;
            return Ok(true);
        }
        Ok(false)
    }

    pub async fn verify_login_otp(
        &self,
        phone: &str,
        code: &str,
    ) -> Result<bool, redis::RedisError> {
        let mut con = self.redis.clone();
        let key = format!("otp:login:{}", phone);
        let stored: Option<String> = con.get(&key).await?;

        if let Some(stored_code) = stored
            && stored_code == code
        {
            let _: usize = con.del(&key).await?;
            return Ok(true);
        }
        Ok(false)
    }

    pub async fn verify_recover_otp(
        &self,
        phone: &str,
        code: &str,
    ) -> Result<bool, redis::RedisError> {
        let mut con = self.redis.clone();
        let key = format!("otp:recover:{}", phone);
        let stored: Option<String> = con.get(&key).await?;

        if let Some(stored_code) = stored
            && stored_code == code
        {
            let _: usize = con.del(&key).await?;
            return Ok(true);
        }
        Ok(false)
    }

    pub async fn store_two_fa_setup_otp(
        &self,
        user_id: &str,
        code: &str,
    ) -> Result<(), redis::RedisError> {
        let mut con = self.redis.clone();
        let key = format!("otp:2fa:setup:{}", user_id);
        let _: () = con.set_ex(key, code, self.otp_ttl).await?;
        Ok(())
    }

    pub async fn verify_two_fa_setup_otp(
        &self,
        user_id: &str,
        code: &str,
    ) -> Result<bool, redis::RedisError> {
        let mut con = self.redis.clone();
        let key = format!("otp:2fa:setup:{}", user_id);
        let stored: Option<String> = con.get(&key).await?;

        if let Some(stored_code) = stored
            && stored_code == code
        {
            let _: usize = con.del(&key).await?;
            return Ok(true);
        }
        Ok(false)
    }

    pub async fn store_two_fa_login_otp(
        &self,
        user_id: &str,
        code: &str,
    ) -> Result<(), redis::RedisError> {
        let mut con = self.redis.clone();
        let key = format!("otp:2fa:login:{}", user_id);
        let _: () = con.set_ex(key, code, self.otp_ttl).await?;
        Ok(())
    }

    pub async fn verify_two_fa_login_otp(
        &self,
        user_id: &str,
        code: &str,
    ) -> Result<bool, redis::RedisError> {
        let mut con = self.redis.clone();
        let key = format!("otp:2fa:login:{}", user_id);
        let stored: Option<String> = con.get(&key).await?;

        if let Some(stored_code) = stored
            && stored_code == code
        {
            let _: usize = con.del(&key).await?;
            return Ok(true);
        }
        Ok(false)
    }

    pub async fn check_rate_limit(
        &self,
        key: &str,
        limit: u64,
        window: u64,
    ) -> Result<bool, redis::RedisError> {
        let mut con = self.redis.clone();
        let rate_key = format!("rate:{}", key);

        let lua_script = r#"
            local count = redis.call('INCR', KEYS[1])
            if count == 1 then
                redis.call('EXPIRE', KEYS[1], ARGV[1])
            end
            return count
        "#;

        let count: u64 = redis::Script::new(lua_script)
            .key(&rate_key)
            .arg(window)
            .invoke_async(&mut con)
            .await?;

        Ok(count <= limit)
    }
}
