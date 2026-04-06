use std::time::Instant;

use dashmap::DashMap;

/// トークンバケット方式のレート制限
pub struct RateLimiter {
    /// ユーザーID -> (残りトークン数, 最終補充時刻)
    buckets: DashMap<u64, (u32, Instant)>,
    /// 1分あたりの最大リクエスト数
    max_tokens: u32,
    /// クールダウン（秒）
    cooldown_secs: u64,
}

impl RateLimiter {
    pub fn new(max_tokens_per_minute: u32, cooldown_secs: u64) -> Self {
        Self {
            buckets: DashMap::new(),
            max_tokens: max_tokens_per_minute,
            cooldown_secs,
        }
    }

    /// リクエストが許可されるかチェックし、許可なら true を返す
    pub fn check_and_consume(&self, user_id: u64) -> bool {
        let now = Instant::now();

        let mut entry = self
            .buckets
            .entry(user_id)
            .or_insert((self.max_tokens, now));
        let (tokens, last_refill) = entry.value_mut();

        // 経過時間に基づきトークンを補充
        let elapsed = now.duration_since(*last_refill).as_secs();
        if elapsed >= self.cooldown_secs {
            let refill = (elapsed / self.cooldown_secs) as u32;
            *tokens = (*tokens + refill).min(self.max_tokens);
            *last_refill = now;
        }

        if *tokens > 0 {
            *tokens -= 1;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_requests_within_limit() {
        let limiter = RateLimiter::new(3, 1);
        assert!(limiter.check_and_consume(1));
        assert!(limiter.check_and_consume(1));
        assert!(limiter.check_and_consume(1));
        assert!(!limiter.check_and_consume(1));
    }

    #[test]
    fn independent_per_user() {
        let limiter = RateLimiter::new(1, 1);
        assert!(limiter.check_and_consume(1));
        assert!(limiter.check_and_consume(2));
        assert!(!limiter.check_and_consume(1));
        assert!(!limiter.check_and_consume(2));
    }
}
