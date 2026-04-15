use redis::aio::ConnectionManager;
use redis::cmd;

pub struct RateLimiter {
    redis: ConnectionManager,
}

impl RateLimiter {
    pub fn new(redis: ConnectionManager) -> Self {
        Self { redis }
    }

    pub async fn check_rate_limit(
        &self,
        key: &str,
        limit: u64,
        window_seconds: u64,
    ) -> Result<RateLimitResult, redis::RedisError> {
        let mut conn = self.redis.clone();
        let full_key = format!("rate:{}", key);

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| {
                redis::RedisError::from((
                    redis::ErrorKind::IoError,
                    "Failed to get current timestamp",
                    e.to_string(),
                ))
            })?
            .as_millis() as u64;

        let lua_script = r#"
            local key = KEYS[1]
            local now = tonumber(ARGV[1])
            local window = tonumber(ARGV[2])
            local limit = tonumber(ARGV[3])
            local window_seconds = tonumber(ARGV[4])

            redis.call('ZREMRANGEBYSCORE', key, 0, now - window)

            local count = redis.call('ZCARD', key)

            if count < limit then
                redis.call('ZADD', key, now, now .. '-' .. math.random())
                redis.call('EXPIRE', key, window_seconds + 1)
                return {count + 1, 1}
            else
                return {count, 0}
            end
        "#;

        let window_ms = window_seconds * 1000;

        let result: (u64, u64) = cmd("EVAL")
            .arg(lua_script)
            .arg(1)
            .arg(&full_key)
            .arg(now)
            .arg(window_ms)
            .arg(limit)
            .arg(window_seconds)
            .query_async(&mut conn)
            .await?;

        let reset_timestamp = (now / 1000) + window_seconds;

        Ok(RateLimitResult {
            allowed: result.1 == 1,
            current_count: result.0,
            limit,
            reset_timestamp,
        })
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RateLimitResult {
    pub allowed: bool,
    pub current_count: u64,
    pub limit: u64,
    pub reset_timestamp: u64,
}
