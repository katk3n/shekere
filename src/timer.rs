use std::time::Instant;

pub struct Timer {
    start: Instant,
}

impl Default for Timer {
    fn default() -> Self {
        Self::new()
    }
}

impl Timer {
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    #[cfg(test)]
    pub fn with_start(start: Instant) -> Self {
        Self { start }
    }

    pub fn get_duration(&self) -> f32 {
        let elapsed = Instant::now() - self.start;
        elapsed.as_secs_f32()
    }

    #[cfg(test)]
    pub fn get_duration_from(&self, now: Instant) -> f32 {
        let elapsed = now - self.start;
        elapsed.as_secs_f32()
    }

    #[cfg(test)]
    pub fn reset(&mut self) {
        self.start = Instant::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_timer_new() {
        let timer = Timer::new();
        let duration = timer.get_duration();
        assert!(duration >= 0.0);
        assert!(duration < 0.1);
    }

    #[test]
    fn test_timer_with_start() {
        let start = Instant::now();
        let timer = Timer::with_start(start);
        assert_eq!(timer.start, start);
    }

    #[test]
    fn test_get_duration_from() {
        let start = Instant::now();
        let timer = Timer::with_start(start);
        let future = start + Duration::from_millis(100);
        let duration = timer.get_duration_from(future);
        assert!((duration - 0.1).abs() < 0.01);
    }

    #[test]
    fn test_reset() {
        let mut timer = Timer::new();
        std::thread::sleep(Duration::from_millis(10));
        timer.reset();
        let duration = timer.get_duration();
        assert!(duration < 0.01);
    }
}
