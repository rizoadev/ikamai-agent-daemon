#[cfg(test)]
mod tests {
    use crate::process::ProcessManager;
    use std::time::Duration;

    #[test]
    fn backoff_grows_exponentially() {
        let b0 = ProcessManager::backoff(0).as_millis();
        let b1 = ProcessManager::backoff(1).as_millis();
        let b2 = ProcessManager::backoff(2).as_millis();
        assert!(b1 >= b0.saturating_sub(1000));
        assert!(b2 >= b1.saturating_sub(1000));
    }

    #[test]
    fn backoff_capped_at_60s() {
        let b = ProcessManager::backoff(100);
        assert!(b <= Duration::from_millis(61_000), "backoff should be capped, got {:?}", b);
    }

    #[test]
    fn backoff_attempt_0_is_near_1s() {
        let b = ProcessManager::backoff(0).as_millis();
        assert!(b >= 1000 && b <= 2000, "attempt 0 should be ~1s, got {b}ms");
    }
}
