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

    /// Raspberry Pi 兩路 PWM 馬達（使用硬體 PWM 腳位）
    pub struct RpiPwmMotor {
        left: Pwm,
        right: Pwm,
        max_mps: f32,
    }

    impl RpiPwmMotor {
        /// 建立兩路 PWM：
        /// - `left_ch`/`right_ch`: PWM 通道（對應特定 GPIO，請見 rppal 文件）
        /// - `freq_hz`: PWM 頻率（例如 1000.0）
        /// - `max_mps`: 速度轉 duty 的比例上限（|v|>=max_mps 時 duty=100%）
        pub fn new(left_ch: Channel, right_ch: Channel, freq_hz: f64, max_mps: f32) -> Result<Self, String> {
            let left = Pwm::with_frequency(left_ch, freq_hz, 0.0, Polarity::Normal, true)
                .map_err(|e| format!("left pwm: {e}"))?;
            let right = Pwm::with_frequency(right_ch, freq_hz, 0.0, Polarity::Normal, true)
                .map_err(|e| format!("right pwm: {e}"))?;
            Ok(Self { left, right, max_mps: max_mps.max(1e-6) })
        }

        fn v_to_duty(&self, v: f32) -> f64 {
            let r = (v / self.max_mps).clamp(-1.0, 1.0) as f64;
            r.abs() // 單邊正轉：先忽略方向，方向可用 H 橋或相反腳實作
        }
    }

    impl Motor for RpiPwmMotor {
        fn set_wheel_speeds(&mut self, left_mps: f32, right_mps: f32) -> Result<(), String> {
            let dl = self.v_to_duty(left_mps);
            let dr = self.v_to_duty(right_mps);
            self.left.set_duty_cycle(dl).map_err(|e| e.to_string())?;
            self.right.set_duty_cycle(dr).map_err(|e| e.to_string())?;
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
            let mut trig = gpio.get(trig_gpio).map_err(|e| e.to_string())?.into_output();
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
    pub use RpiPwmMotor as RpiMotor;
}

// 方便外部使用：feature 啟用時導出實機型別名稱，否則僅導出 stub
#[cfg(feature = "rpi")]
pub use real::{RpiDistance, RpiMotor};
