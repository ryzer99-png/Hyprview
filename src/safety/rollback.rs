use crate::models::config::DisplayConfiguration;
use std::time::{Duration, Instant};

#[derive(Debug, PartialEq, Clone)]
pub enum SafetyState {
    Committed(DisplayConfiguration),
    PendingConfirmation {
        previous: DisplayConfiguration,
        candidate: DisplayConfiguration,
        deadline: Instant,
    },
    Reverted(DisplayConfiguration),
}

pub struct RollbackGuard {
    pub state: SafetyState,
    pub timeout: Duration,
}

impl RollbackGuard {
    pub fn new(current: DisplayConfiguration) -> Self {
        Self {
            state: SafetyState::Committed(current),
            timeout: Duration::from_secs(15),
        }
    }

    pub fn with_timeout(current: DisplayConfiguration, timeout: Duration) -> Self {
        Self {
            state: SafetyState::Committed(current),
            timeout,
        }
    }

    /// Transitions to PendingConfirmation state with 15s deadline
    pub fn apply_candidate(&mut self, candidate: DisplayConfiguration) {
        let prev = match &self.state {
            SafetyState::Committed(c) => c.clone(),
            SafetyState::PendingConfirmation { previous, .. } => previous.clone(),
            SafetyState::Reverted(c) => c.clone(),
        };

        self.state = SafetyState::PendingConfirmation {
            previous: prev,
            candidate,
            deadline: Instant::now() + self.timeout,
        };
    }

    /// User confirms changes within the window; commits the candidate configuration
    pub fn confirm(&mut self) -> Option<DisplayConfiguration> {
        if let SafetyState::PendingConfirmation { candidate, .. } = &self.state {
            let committed = candidate.clone();
            self.state = SafetyState::Committed(committed.clone());
            Some(committed)
        } else {
            None
        }
    }

    /// Returns remaining seconds until rollback triggers
    pub fn remaining_seconds(&self) -> u64 {
        if let SafetyState::PendingConfirmation { deadline, .. } = &self.state {
            let now = Instant::now();
            if *deadline > now {
                (*deadline - now).as_secs() + 1
            } else {
                0
            }
        } else {
            0
        }
    }

    /// Checks if deadline has expired
    pub fn is_expired(&self) -> bool {
        if let SafetyState::PendingConfirmation { deadline, .. } = &self.state {
            Instant::now() >= *deadline
        } else {
            false
        }
    }

    /// Reverts to previous configuration
    pub fn rollback(&mut self) -> Option<DisplayConfiguration> {
        if let SafetyState::PendingConfirmation { previous, .. } = &self.state {
            let revert_to = previous.clone();
            self.state = SafetyState::Reverted(revert_to.clone());
            Some(revert_to)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::geometry::{Point, Transform};
    use crate::models::monitor::Monitor;
    use crate::models::resolution::Mode;

    fn test_config(x: i32) -> DisplayConfiguration {
        DisplayConfiguration::new(vec![Monitor {
            id: 0,
            name: "eDP-1".to_string(),
            description: "Screen".to_string(),
            make: "".to_string(),
            model: "".to_string(),
            serial: "".to_string(),
            enabled: true,
            is_primary: true,
            active_mode: Mode::new(1920, 1080, 60.0),
            position: Point::new(x, 0),
            scale: 1.0,
            transform: Transform::Normal,
            vrr: false,
            available_modes: vec![],
            mirror_of: None,
        }])

    }

    #[test]
    fn test_rollback_guard_flow_confirm() {
        let original = test_config(0);
        let candidate = test_config(1920);

        let mut guard = RollbackGuard::new(original.clone());
        guard.apply_candidate(candidate.clone());

        assert!(matches!(guard.state, SafetyState::PendingConfirmation { .. }));
        assert!(guard.remaining_seconds() > 0);
        assert!(!guard.is_expired());

        let confirmed = guard.confirm().expect("should confirm");
        assert_eq!(confirmed, candidate);
        assert_eq!(guard.state, SafetyState::Committed(candidate));
    }

    #[test]
    fn test_rollback_guard_timeout_revert() {
        let original = test_config(0);
        let candidate = test_config(1920);

        let mut guard = RollbackGuard::with_timeout(original.clone(), Duration::from_millis(50));
        guard.apply_candidate(candidate);

        std::thread::sleep(Duration::from_millis(60));
        assert!(guard.is_expired());

        let reverted = guard.rollback().expect("should revert");
        assert_eq!(reverted, original);
        assert_eq!(guard.state, SafetyState::Reverted(original));
    }
}
