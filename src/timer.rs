use std::time::Instant;

pub struct Timer {
    start: Instant,
}

impl Timer {
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    pub fn get_duration(&self) -> f32 {
        let elapsed = Instant::now() - self.start;
        return elapsed.as_secs_f32();
    }
}
