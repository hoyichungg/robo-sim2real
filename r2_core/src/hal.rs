pub trait Motor {
    fn set_wheel_speeds(&mut self, left_mps: f32, right_mps: f32) -> Result<(), String>;
}

pub trait DistanceSensor {
    fn distance_m(&mut self) -> Result<f32, String>;
}

pub trait Clock {
    fn now_s(&self) -> f64;
    fn sleep_ms(&self, ms: u64);
}

pub trait Telemetry {
    fn record(&mut self, ts_s: f64, key: &str, value: &str);
}
