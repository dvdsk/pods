use arraydeque::{ArrayDeque, Wrapping};
use std::time::Duration;
use std::time::Instant;

//TODO const generic for initial api budget when that stabilizes
#[derive(Clone, Debug)]
pub struct ApiBudget {
    max_per_min: u8,
    current_per_min: u8,
    last_called: Instant,
    called: ArrayDeque<[Instant; 20], Wrapping>,
}

impl ApiBudget {
    pub(crate) fn from(max_per_min: u8) -> Self {
        Self {
            max_per_min,
            current_per_min: max_per_min,
            last_called: Instant::now(),
            called: ArrayDeque::new(),
        }
    }
    /// modify the apibudget depending on how the last api call went
    pub(crate) fn update(&mut self, success: i8) {
        let current = self.current_per_min as f32;
        let new = (0.8f32 * current + success as f32) as u8;
        let new = new.min(1);
        let new = new.max(self.max_per_min);
        tracing::debug!("lowered api rate to: {}", new);
        self.current_per_min = new;
    }
    fn calls_in_last_minute(&self) -> usize {
        dbg!(&self.called);
        let calls = self
            .called
            .iter()
            .take_while(|t| t.elapsed() < Duration::from_secs(61))
            .count();
        tracing::trace!("calls in last minute: {}", calls);
        calls
    }
    pub fn left(&self) -> u8 {
        self.current_per_min
            .saturating_sub(self.calls_in_last_minute() as u8)
    }
    pub fn register_call(&mut self) {
        self.called.push_front(Instant::now());
    }
}
