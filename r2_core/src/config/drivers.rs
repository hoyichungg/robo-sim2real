#[derive(Debug, Clone)]
pub enum MotorBackend {
    Mock,
    Bench(BenchMotorConfig),
    Rpi(RpiMotorConfig),
}

impl Default for MotorBackend {
    fn default() -> Self {
        MotorBackend::Mock
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BenchMotorConfig {
    pub tau: f32,
    pub plant_gain: f32,
}

impl Default for BenchMotorConfig {
    fn default() -> Self {
        Self {
            tau: 0.8,
            plant_gain: 0.6,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RpiMotorConfig {
    pub left_pwm: u8,
    pub right_pwm: u8,
    pub left_dir: Option<(u8, u8)>,
    pub right_dir: Option<(u8, u8)>,
    pub max_mps: f32,
    pub pwm_freq_hz: f64,
}

impl Default for RpiMotorConfig {
    fn default() -> Self {
        Self {
            left_pwm: 18,
            right_pwm: 19,
            left_dir: None,
            right_dir: None,
            max_mps: 1.0,
            pwm_freq_hz: 1000.0,
        }
    }
}

#[derive(Debug, Clone)]
pub enum DistanceBackend {
    Mock,
    RpiHcsr04 { trig_gpio: u8, echo_gpio: u8 },
}

impl Default for DistanceBackend {
    fn default() -> Self {
        DistanceBackend::Mock
    }
}

#[derive(Debug, Clone, Default)]
pub struct DriverConfig {
    pub motor: MotorBackend,
    pub distance: DistanceBackend,
}
