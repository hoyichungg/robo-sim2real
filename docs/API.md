# API 參考指南

本文件彙整專案中對外可用的型別與函式（重點在 HAL、控制與平台工具），並附上簡短使用範例。若需更完整的設計背景，請先閱讀 `docs/ARCHITECTURE.md`。

## HAL（硬體抽象層）
檔案：`r2_core/src/hal.rs`

- `trait Motor`
  - `fn set_wheel_speeds(&mut self, left_mps: f32, right_mps: f32) -> Result<(), String>`
  - 說明：設定左右輪線速度命令（m/s）。

- `trait DistanceSensor`
  - `fn distance_m(&mut self) -> Result<f32, String>`
  - 說明：回傳與前方障礙物的距離（m）。錯誤/NaN/負值將被 FailSafe 視為危險。

- `trait Clock`
  - `fn now_s(&self) -> f64`、`fn sleep_ms(&self, ms: u64)`
  - 說明：時間來源抽象（目前平台以標準庫替代）。

- `trait Telemetry`
  - `fn record(&mut self, ts_s: f64, key: &str, value: &str)`
  - 說明：最小遙測輸出介面；`r2_core/src/replay.rs` 有 stdout 範例。

範例：自訂硬體馬達/感測器（簡化示意）
```rust
use r2_core::hal::{Motor, DistanceSensor};

pub struct MyMotor { /* pwm, gpio, ... */ }
impl Motor for MyMotor {
    fn set_wheel_speeds(&mut self, left: f32, right: f32) -> Result<(), String> {
        // 轉換為 PWM / RPM，寫入驅動
        Ok(())
    }
}

pub struct MyTof { /* i2c, addr, ... */ }
impl DistanceSensor for MyTof {
    fn distance_m(&mut self) -> Result<f32, String> {
        // 讀 I2C，回傳公尺
        Ok(0.42)
    }
}
```

## Drivers

檔案：`drivers/src/factory.rs`

- `type MotorHandle = Box<dyn Motor>`
- `type DistanceHandle = Box<dyn DistanceSensor>`
- `struct DriverHandles { pub motor: MotorHandle, pub distance: DistanceHandle }`
- `struct DriverFactory`
  - `fn create_motor(cfg: &MotorBackend) -> Result<MotorHandle, String>`
  - `fn create_distance(cfg: &DistanceBackend) -> Result<DistanceHandle, String>`
  - `fn create_all(cfg: &DriverConfig) -> Result<DriverHandles, String>`

說明：根據 `r2_core::config::drivers::DriverConfig`（包含 `MotorBackend`、`DistanceBackend`）建立對應的 HAL 實作。若未啟用 `drivers` crate 的 `rpi` feature，Raspberry Pi 後端會自動回退到 stub 型別。

範例：
```rust
use drivers::factory::{DriverFactory, DriverHandles};
use r2_core::config::drivers::{DriverConfig, MotorBackend, DistanceBackend};

let cfg = DriverConfig {
    motor: MotorBackend::Mock,
    distance: DistanceBackend::Mock,
};
let DriverHandles { mut motor, mut distance } = DriverFactory::create_all(&cfg)?;
motor.set_wheel_speeds(0.5, 0.5)?;
let d = distance.distance_m()?;
```

---

## 控制

### PID
檔案：`r2_core/src/control/pid.rs`

- `struct Pid { kp, ki, kd, i, prev_e, out_min, out_max, i_min, i_max }`
- `fn new(kp: f32, ki: f32, kd: f32) -> Self`
- `fn with_output_limits(self, out_min: f32, out_max: f32) -> Self`
- `fn with_integral_limits(self, i_min: f32, i_max: f32) -> Self`
- `fn reset(&mut self)`
- `fn step(&mut self, target: f32, measured: f32, dt: f32) -> f32`

範例：
```rust
use r2_core::control::pid::Pid;
let mut pid = Pid::new(0.8, 0.05, 0.04)
    .with_output_limits(-1.0, 1.0)
    .with_integral_limits(-0.5, 0.5);
let u = pid.step(0.6, 0.3, 0.01); // 目標 0.6、量測 0.3、dt=10ms
```

### FailSafe（安全保護）
檔案：`r2_core/src/control/safety.rs`

- `enum SafetyState { Run, EmergencyBrake, SafeStop }`
- `struct FailSafe { threshold_m, hysteresis_m, state }`
- `fn new(threshold_m: f32, hysteresis_m: f32) -> Self`
- `fn state(&self) -> SafetyState`
- `fn update(&mut self, distance: Result<f32, ()>) -> SafetyState`
- `fn update_opt(&mut self, distance: Option<f32>) -> SafetyState`
- `fn clamp_speed(&self, v_cmd: f32) -> f32`
- `fn reset(&mut self, current_distance_m: Option<f32>)`

行為摘要：
- 錯誤/NaN/負值 或 距離 ≤ threshold → `EmergencyBrake`
- threshold < 距離 ≤ threshold + hysteresis：若先前為急停則進入 `SafeStop`
- 距離 > threshold + hysteresis：自動解除回到 `Run`

範例：
```rust
use r2_core::control::safety::{FailSafe, SafetyState};
let mut fs = FailSafe::new(0.25, 0.05);
assert_eq!(fs.update(Ok(0.20)), SafetyState::EmergencyBrake);
assert_eq!(fs.clamp_speed(0.6), 0.0);
assert_eq!(fs.update(Ok(0.28)), SafetyState::SafeStop); // 尚未超過 hysteresis
assert_eq!(fs.update(Ok(0.31)), SafetyState::Run);      // 自動解除
```

### Controller（組合控制器）
檔案：`r2_core/src/control/controller.rs`

- `struct DifferentialKinematics { wheel_base_m }`
  - `fn to_wheel_speeds(&self, v_mps: f32, w_rps: f32) -> (f32, f32)`
- `struct Controller { pid_v, kin, safety, v_meas }`
  - `fn new(pid_v: Pid, kin: DifferentialKinematics, safety: FailSafe) -> Self`
  - `fn tick(&mut self, desired_v: f32, dt_s: f32, distance_m: Result<f32, ()>) -> ((f32, f32), SafetyState)`

範例：
```rust
use r2_core::control::controller::{Controller, DifferentialKinematics};
use r2_core::control::pid::Pid;
use r2_core::control::safety::FailSafe;

let pid = Pid::new(1.0, 0.5, 0.05)
    .with_output_limits(-1.0, 1.0)
    .with_integral_limits(-0.5, 0.5);
let kin = DifferentialKinematics { wheel_base_m: 0.22 };
let safety = FailSafe::new(0.25, 0.05);
let mut ctrl = Controller::new(pid, kin, safety);

let ((l, r), state) = ctrl.tick(0.6, 0.02, Ok(0.8));
```

## 平台工具（platform_rpi）

### 速度曲線與遙測
檔案：`platform_rpi/src/profile.rs`

- `enum VProfile { Const, Step, Sin }`
- `struct ProfileParams { step_at, sin_amp, sin_freq, sin_bias }`
- `fn desired_v(profile: Option<VProfile>, params: ProfileParams, desired_v_const: f32, t: f32) -> f32`
- `struct Telemetry { t, dt, desired_v, left, right, distance, state, meas_left, meas_right, err, adapt_gain }`

範例：
```rust
use platform_rpi::profile::{VProfile, ProfileParams, desired_v};
let params = ProfileParams { step_at: 1.0, sin_amp: 0.3, sin_freq: 0.2, sin_bias: 0.4 };
let v = desired_v(Some(VProfile::Sin), params, 0.6, 2.5);
```

### 自適應輸出增益
檔案：`r2_core/src/control/adaptive.rs`

- `fn map_gain(abs_e: f32, e_small: f32, e_large: f32, g_min: f32, g_max: f32) -> f32`

說明：把誤差絕對值線性映射到輸出增益，兩端夾限。`platform_rpi::adaptive` 直接 re-export 此函式，`sim2d` 也使用相同邏輯。

範例：
```rust
use r2_core::control::adaptive::map_gain;
let g = map_gain(0.05, 0.02, 0.20, 0.6, 1.2); // => 約 0.7~0.8 之間
```

## 模擬（sim2d）要點

雖然 Bevy 系統屬於內部流程，但以下型別與函式最常被外部引用：
- `components.rs`：`Car`、`Obstacle`、`Velocity { v, omega }`、`Heading(Vec2)`。
- `resources.rs`：`SimClock { t, dt }`、`DistanceSense(f32)`、`TelemetryWriter::new(path)` / `write(...)`（輸出與平台共用的 CSV header）。
- `config.rs`：
  - `struct Cli { pub config: Option<PathBuf>, pub overrides: OverrideArgs }`
  - `struct SimSettings`：`fn into_settings(self) -> Result<SimSettings, String>`、`fn to_runtime(&self) -> RuntimeCfg`、`fn csv_path(&self) -> &Path`
  - `struct RuntimeCfg`（Bevy `Resource`）：控制參數、plant、障礙、CSV 路徑等。
  - `desired_speed(cfg: &RuntimeCfg, t: f32) -> f32`：呼叫 `r2_core::profile::desired_v`，支援 const/step/sin。

Config 流程：
1. `Cli::parse()` 讀取 `--config <file>` 與 CLI override。
2. `Cli::into_settings()` 會載入 TOML（支援陣列/物件/字串格式的 obstacles）並套用 CLI 覆蓋，處理相對路徑。
3. `SimSettings::to_runtime()` 轉為 `RuntimeCfg`，再注入 Bevy App。

控制步驟（`control.rs`）：以 `Pid` 產生 `u`，可選擇 `map_gain` 套用自適應增益後，再由 `FailSafe.clamp_speed(u)` 做最終裁切，寫回 `Velocity.v`。

## 小貼士
- FailSafe 對無效距離（Err/NaN/負值）視為危險 → 立即急停，且距離超過 `threshold + hysteresis` 會自動解除。
- `DriverFactory` 會根據 feature 自動選擇實機或 stub；啟用 `rpi` feature 後才會連結 `rppal`。
- `Controller.tick(...)` 目前角速度給 0；若要擴充轉向，可改 tick 簽名或新增角速控制器。
- `sim2d` 的 `--config` 支援相對路徑，皆以設定檔所在目錄為基準。
