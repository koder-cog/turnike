use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug)]
pub struct PreemptionTracker {
    max_attempts: usize,
    window: Duration,
    state_file: Option<PathBuf>,
    history: Vec<u64>,
}

impl Default for PreemptionTracker {
    fn default() -> Self {
        let state_path = std::env::var("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| std::env::temp_dir())
            .join("turnike_preempt.state");
        Self::with_storage(2, Duration::from_secs(600), Some(state_path))
    }
}

impl PreemptionTracker {
    pub fn new(max_attempts: usize, window: Duration) -> Self {
        Self::with_storage(max_attempts, window, None)
    }

    pub fn with_storage(
        max_attempts: usize,
        window: Duration,
        state_file: Option<PathBuf>,
    ) -> Self {
        let mut history = Vec::new();
        if let Some(ref path) = state_file
            && let Ok(content) = fs::read_to_string(path)
        {
            history = content
                .split_whitespace()
                .filter_map(|s| s.parse::<u64>().ok())
                .collect();
        }

        let mut tracker = Self {
            max_attempts,
            window,
            state_file,
            history,
        };
        tracker.prune();
        tracker
    }

    fn current_epoch() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    fn prune(&mut self) {
        let now = Self::current_epoch();
        let win_secs = self.window.as_secs();
        self.history.retain(|&ts| now.saturating_sub(ts) < win_secs);
        self.sync_storage();
    }

    fn sync_storage(&self) {
        if let Some(ref path) = self.state_file {
            let data = self
                .history
                .iter()
                .map(|ts| ts.to_string())
                .collect::<Vec<_>>()
                .join(" ");
            let _ = fs::write(path, data);
        }
    }

    pub fn try_acquire(&mut self) -> bool {
        self.prune();
        if self.history.len() >= self.max_attempts {
            return false;
        }

        self.history.push(Self::current_epoch());
        self.sync_storage();
        true
    }

    pub fn current_count(&mut self) -> usize {
        self.prune();
        self.history.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preemption_limit_enforced() {
        let mut tracker = PreemptionTracker::new(2, Duration::from_secs(60));
        assert!(tracker.try_acquire());
        assert_eq!(tracker.current_count(), 1);

        assert!(tracker.try_acquire());
        assert_eq!(tracker.current_count(), 2);

        assert!(!tracker.try_acquire());
        assert_eq!(tracker.current_count(), 2);
    }

    #[test]
    fn test_preemption_state_file_persistence() {
        let temp_dir = std::env::temp_dir();
        let state_file =
            temp_dir.join(format!("test_turnike_preempt_{}.state", std::process::id()));
        let _ = fs::remove_file(&state_file);

        {
            let mut t1 = PreemptionTracker::with_storage(
                2,
                Duration::from_secs(60),
                Some(state_file.clone()),
            );
            assert!(t1.try_acquire());
            assert_eq!(t1.current_count(), 1);
        }

        {
            let mut t2 = PreemptionTracker::with_storage(
                2,
                Duration::from_secs(60),
                Some(state_file.clone()),
            );
            assert_eq!(t2.current_count(), 1);
            assert!(t2.try_acquire());
            assert!(!t2.try_acquire());
        }

        let _ = fs::remove_file(&state_file);
    }
}
