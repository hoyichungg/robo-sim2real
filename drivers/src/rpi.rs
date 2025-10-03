use r2_core::hal::{DistanceSensor, Motor};

// =============================================================
// 非 RPi（或未開 feature=rpi）時：提供可編譯的 stub 實作
// =============================================================
#[cfg(not(feature = "rpi"))]
pub struct RpiMotorStub;
#[cfg(not(feature = "rpi"))]
impl Motor for RpiMotorStub {
    fn set_wheel_speeds(&mut self, _left: f32, _right: f32) -> Result<(), String> {
        Ok(())
    }
}

#[cfg(not(feature = "rpi"))]
pub struct RpiDistanceStub;
#[cfg(not(feature = "rpi"))]
impl DistanceSensor for RpiDistanceStub {
    fn distance_m(&mut self) -> Result<f32, String> {
        Ok(0.8)
    }
}

// =============================================================
// RPi 實機版（需要 --features drivers/rpi）
// - 馬達：硬體 PWM（兩路），將 m/s 依 max_mps 線性映射到 duty cycle
// - 距離：HC-SR04 超音波，GPIO 量測 echo pulse 寬度
// =============================================================
#[cfg(feature = "rpi")]
mod real {
    use super::*;
    use rppal::gpio::{Gpio, InputPin, Level, OutputPin};
    use rppal::pwm::{Channel, Polarity, Pwm};
    use std::thread;
    use std::time::{Duration, Instant};

    use r2_core::config::drivers::RpiMotorConfig;

    /// Raspberry Pi 兩路 PWM 馬達（支援方向腳）
    pub struct RpiMotor {
        left_pwm: Pwm,
        right_pwm: Pwm,
        left_dir: Option<(OutputPin, OutputPin)>,
        right_dir: Option<(OutputPin, OutputPin)>,
        max_mps: f32,
    }

    impl RpiMotor {
        pub fn new(cfg: RpiMotorConfig) -> Result<Self, String> {
            let left_ch = channel_from_index(cfg.left_pwm)?;
            let right_ch = channel_from_index(cfg.right_pwm)?;

            let left_pwm =
                Pwm::with_frequency(left_ch, cfg.pwm_freq_hz, 0.0, Polarity::Normal, true)
                    .map_err(|e| format!("left pwm: {e}"))?;
            let right_pwm =
                Pwm::with_frequency(right_ch, cfg.pwm_freq_hz, 0.0, Polarity::Normal, true)
                    .map_err(|e| format!("right pwm: {e}"))?;

            let left_dir = create_dir_pair(cfg.left_dir)?;
            let right_dir = create_dir_pair(cfg.right_dir)?;

            Ok(Self {
                left_pwm,
                right_pwm,
                left_dir,
                right_dir,
                max_mps: cfg.max_mps.max(1e-6),
            })
        }

        fn v_to_duty(&self, v: f32) -> f64 {
            (v / self.max_mps).clamp(-1.0, 1.0).abs() as f64
        }
    }

    impl Motor for RpiMotor {
        fn set_wheel_speeds(&mut self, left_mps: f32, right_mps: f32) -> Result<(), String> {
            let left_cmd = left_mps.clamp(-self.max_mps, self.max_mps);
            let right_cmd = right_mps.clamp(-self.max_mps, self.max_mps);

            if left_cmd < 0.0 && self.left_dir.is_none() {
                return Err(
                    "left motor configured without direction pins; received negative speed".into(),
                );
            }
            if right_cmd < 0.0 && self.right_dir.is_none() {
                return Err(
                    "right motor configured without direction pins; received negative speed".into(),
                );
            }

            if let Some((ref mut in1, ref mut in2)) = self.left_dir.as_mut() {
                set_direction(in1, in2, left_cmd);
            }
            if let Some((ref mut in1, ref mut in2)) = self.right_dir.as_mut() {
                set_direction(in1, in2, right_cmd);
            }

            let dl = self.v_to_duty(left_cmd);
            let dr = self.v_to_duty(right_cmd);
            self.left_pwm
                .set_duty_cycle(dl)
                .map_err(|e| e.to_string())?;
            self.right_pwm
                .set_duty_cycle(dr)
                .map_err(|e| e.to_string())?;
            Ok(())
        }
    }

    /// HC-SR04 超音波距離感測器
    pub struct Hcsr04 {
        trig: OutputPin,
        echo: InputPin,
    }

    impl Hcsr04 {
        /// `trig_gpio` / `echo_gpio`: BCM GPIO 編號（非實體 pin 序號）
        pub fn new(trig_gpio: u8, echo_gpio: u8) -> Result<Self, String> {
            let gpio = Gpio::new().map_err(|e| e.to_string())?;
            let mut trig = gpio
                .get(trig_gpio)
                .map_err(|e| e.to_string())?
                .into_output();
            trig.set_low();
            let echo = gpio.get(echo_gpio).map_err(|e| e.to_string())?.into_input();
            Ok(Self { trig, echo })
        }

        fn pulse_us(us: u64) -> Duration {
            Duration::from_micros(us)
        }
    }

    impl DistanceSensor for Hcsr04 {
        fn distance_m(&mut self) -> Result<f32, String> {
            // 觸發 10us 高脈衝
            self.trig.set_low();
            thread::sleep(Self::pulse_us(2));
            self.trig.set_high();
            thread::sleep(Self::pulse_us(10));
            self.trig.set_low();

            // 等待 echo 拉高（最多 25ms）
            let timeout = Duration::from_millis(25);
            let t0 = Instant::now();
            while self.echo.read() == Level::Low {
                if t0.elapsed() > timeout {
                    return Err("echo timeout (rise)".into());
                }
            }
            let start = Instant::now();

            // 等待 echo 下降
            while self.echo.read() == Level::High {
                if start.elapsed() > timeout {
                    return Err("echo timeout (fall)".into());
                }
            }
            let dur = start.elapsed();

            // 距離 = 時間 * 音速 / 2
            let secs = dur.as_secs_f32();
            let d = secs * 343.0 / 2.0;
            Ok(d)
        }
    }

    // 公開型別
    pub use Hcsr04 as RpiDistance;
    pub use RpiMotor;

    fn channel_from_index(idx: u8) -> Result<Channel, String> {
        match idx {
            0 => Ok(Channel::Pwm0),
            1 => Ok(Channel::Pwm1),
            other => Err(format!("unsupported PWM channel index: {other}")),
        }
    }

    fn create_dir_pair(pins: Option<(u8, u8)>) -> Result<Option<(OutputPin, OutputPin)>, String> {
        if let Some((pin_a, pin_b)) = pins {
            let mut in1 = gpio_output(pin_a)?;
            let mut in2 = gpio_output(pin_b)?;
            in1.set_low();
            in2.set_low();
            Ok(Some((in1, in2)))
        } else {
            Ok(None)
        }
    }

    fn gpio_output(pin: u8) -> Result<OutputPin, String> {
        Gpio::new()
            .map_err(|e| e.to_string())?
            .get(pin)
            .map_err(|e| e.to_string())?
            .into_output()
    }

    fn set_direction(in1: &mut OutputPin, in2: &mut OutputPin, v: f32) {
        const EPS: f32 = 1e-4;
        if v > EPS {
            in1.set_high();
            in2.set_low();
        } else if v < -EPS {
            in1.set_low();
            in2.set_high();
        } else {
            in1.set_low();
            in2.set_low();
        }
    }
}

// 方便外部使用：feature 啟用時導出實機型別名稱，否則僅導出 stub
#[cfg(not(feature = "rpi"))]
pub use self::{RpiDistanceStub as RpiDistance, RpiMotorStub as RpiMotor};
#[cfg(feature = "rpi")]
pub use real::{RpiDistance, RpiMotor};
