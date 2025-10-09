# robo-sim2real 架構說明

本文件說明專案目標、模組切分、控制資料流、擴充點（API 開口）與開發/執行指引，協助你在「實體（Raspberry Pi）」與「2D 視覺模擬」之間共用核心控制邏輯。

## 目標
- 在單一核心庫上共用 PID 控制與安全保護（FailSafe）。
- 以 HAL（硬體抽象層）隔離驅動層，使真硬體與模擬環境可互換。
- 以簡單 CLI/CSV/腳本完成迴圈測試、掃參與分析繪圖。

## 專案結構
- `r2_core/`：核心庫（控制、HAL、模型/遙測骨架）
- `drivers/`：對 HAL 的實作（mock、Raspberry Pi feature-gated 驅動、自動 fallback stub）
- `platform_rpi/`：實體平台/命令列執行（含 bench 一階模型、自適應增益、CSV）
- `sim2d/`：Bevy 2D 視覺模擬（感測 → 控制 → 物理 → 紀錄 的固定步進管線）
- `tests/`：針對 FailSafe 與 PID 的單元測試
- `scripts/`：CSV 分析與繪圖（單次、A/B 與 sweep）
- `configs/`：範例與自訂 TOML 設定檔（可在 CLI 以 `--config` 載入）

## 核心觀念

### HAL（硬體抽象層）
檔案：`r2_core/src/hal.rs`
- `trait Motor { set_wheel_speeds(left_mps, right_mps) }`：馬達命令入口。
- `trait DistanceSensor { distance_m() }`：距離量測（公尺）。
- `trait Clock { now_s(), sleep_ms(ms) }`：時間抽象（目前平台多用標準庫）。
- `trait Telemetry { record(ts_s, key, value) }`：最小化遙測介面（`replay.rs` 內有 stdout 版）。

這些 trait 是本專案最重要的 API 開口。任何新硬體只要實作它們即可融入控制流程。

### 控制（PID / FailSafe / Controller）
檔案：
- `r2_core/src/control/pid.rs`：`Pid::step(target, measured, dt)` 實作 P/I/D，含積分/輸出夾限。
- `r2_core/src/control/safety.rs`：`FailSafe` 狀態機（Run / EmergencyBrake / SafeStop），`update(...)` 依距離/錯誤更新狀態，`clamp_speed(v)` 做最終零速裁切；距離超過 `threshold + hysteresis` 時會自動回到 Run，如需人工解除仍可呼叫 `reset(...)`。
- `r2_core/src/control/controller.rs`：`Controller::tick(desired_v, dt, distance)` 組合 PID + FailSafe + 差速運動學（先只用線速，角速保留）。內部用簡單低通模擬量測慣性。

注意：`r2_core/src/control/telemetry.rs` 內定義的 `TelemetrySample` 與 HAL 的 `trait Telemetry` 是不同概念；前者描述一次樣本的欄位，後者提供輸出介面。

### 自適應輸出增益（Adaptive Gain）
- `r2_core/src/control/adaptive.rs`：提供 `GainScheduler` trait、`AdaptiveGainScheduler`、`FixedGainScheduler`，以及 `scheduler_from_params(...)` 幫手，可根據設定建構 Box 化排程器；底層仍透過 `map_gain(...)` 做線性插值。
- `config::control::AdaptiveConfig::build_scheduler()`：從設定直接建立調度器；模擬與平台都改為呼叫排程器而非自己判斷 `enabled`。

## 平台與資料流

### 平台（Raspberry Pi/命令列）
檔案：`platform_rpi/src/main.rs`
- CLI 旗標（目標速度、PID、頻率、秒數、FailSafe 門檻/回滯、CSV、速度曲線、bench、一階參數、自適應參數），這些控制相關旗標會被彙整成 `ControlOverrides`，套用到 `ControlConfig::default()` 後給 `Controller` 使用，確保模擬與平台共用同一組合併邏輯。
- 迴圈每步：
  1) 以 `runtime.profile.sample(...)` 計算目標速度（Const / Step / Sin）。
  2) 讀距離 `DistanceSensor::distance_m()` → `FailSafe.update(...)`。
  3) `Controller.tick(...)` 得到未自適應的 `(left,right)` 與 `SafetyState`。
  4) 若 `--bench`：用一階模型（`tau`, `gain`）把命令轉成「量測速度」估測誤差。
  5) 若 `--adaptive`：透過 `GainScheduler::update(err)` 調整輸出增益並夾限到 [-1,1]。
  6) 呼叫 `Motor::set_wheel_speeds(...)`（預設使用 `drivers::mock::MockMotor`）。
  7) 累積遙測列，最後輸出 CSV（`run/telemetry.csv` 或自訂路徑）。

- CSV header 與 `sim2d` 完全對齊：`t,dt,desired_v,left,right,distance,state,meas_left,meas_right,err,adapt_gain`，方便共用分析腳本。

- 速度曲線與遙測：
- `platform_rpi/src/profile.rs`：CLI 專用的 `VProfile` enum 會映射到 `r2_core::profile::VProfile`；`ProfileExecutor::sample(...)` 封裝 Const/Step/Sin 計算。
- `r2_core::control::telemetry::TelemetrySample` 描述 CSV 欄位（含 bench mode 量測值與自適應增益），可透過 `TelemetrySink` 寫往任意輸出（平台與模擬皆使用 CSV 版）。

### 2D 模擬（Bevy）
檔案：`sim2d/src/*.rs`
- 啟動：`main.rs` 解析 `Cli`（支援 `--config <file>`），透過 `SimSettings` 合併 TOML 與 CLI override，失敗時會印出錯誤並結束。成功後插入 `SimClock`、`DistanceSense`、`TelemetryWriter`、`RuntimeCfg`，再設定 FixedUpdate 管線。
- 固定步進順序（`SimStep` SystemSet）：
  1) `sensing::sense_distance`（SimStep::Sense）：Ray-AABB 測距，考慮安全裕度。
  2) `control::control_step`（SimStep::Control）：`Pid` → `GainScheduler` → `FailSafe.clamp_speed`，更新 `Velocity.cmd` 並寫入遙測。
  3) `physics::integrate_kinematics`（SimStep::Physics）：一階模型濾命令並積分到世界座標。
  4) `logging::flush_telemetry`（SimStep::Logging）：定期 flush CSV writer。

組件與資源：
- `components.rs`：`Car`、`Obstacle`、`Velocity { cmd, meas, omega }`、`Heading(Vec2)`。
- `resources.rs`：`SimClock{t,dt}`、`DistanceSense(f32)`、`TelemetryWriter`（預設寫出與平台共用的 CSV header）。
- `config.rs`：
  - `Cli` 帶有 `--config` 與 `OverrideArgs`，提供所有控制/plant/自適應/障礙/CSV 參數。
  - `SimSettings` 會載入 TOML (`FileOverrides`) 並套用 CLI；`RuntimeConfig` 預先整合 `LoopConfig`、`ProfileConfig`、`PlantConfig`，並提供 `apply_overrides(...)` 共用邏輯。
  - `RuntimeCfg` 作為 Bevy `Resource`，內含 `runtime: RuntimeConfig`；`desired_speed(...)` 以 `runtime.profile.sample(...)` 取得目標速度。

## 驅動（Drivers）
- `drivers/src/factory.rs`：`DriverFactory` 內建 `DeviceBuilder` 註冊表並回傳 `DriverSet`；預設對 motor/distance 設定 builder，使用者也可呼叫 `register(...)` 新增裝置，並透過 `DriverSet::extras` 存放附加 handle（例如 bench 參數）。
- `drivers/src/mock.rs`：
  - `MockMotor`：以 `tracing::debug!` 記錄左右馬達命令，可透過 subscriber 控制輸出，避免高頻噪訊；平台執行可用 `--quiet` 抑制 stdout。
  - `MockSensor`：提供可重複的距離函數（預設逐步靠近），方便驗證 FailSafe。
- `drivers/src/rpi.rs`：
  - Feature = `rpi` 時匯入 `rppal`，提供 `RpiMotor`（硬體 PWM + 選配方向腳）與 `RpiDistance`（HC-SR04 超音波）。
  - 無該 feature 時，公開同名 stub 型別以保持 API 相容。

啟用方式：`cargo build -p drivers --features rpi`（workspace 其他 crate 亦可透過 `drivers/` 的 feature 啟動）。

## 測試
- `tests/safety_failsafe.rs`：覆蓋錯誤/NaN/負值急停、閾值觸發與 SafeStop、回滯解除條件等。
- `tests/control_pid.rs`：`Pid` 在一階系統上的收斂性（含輸出/積分夾限）。

## 腳本與分析
- `scripts/plot_run.py`：讀單一 `run.csv` 繪製輸出 vs 期望與距離，輸出 `run.png`。
- `scripts/run_pid_sweep.py`：以 `cargo run -p platform_rpi -- ...` 掃描 Kp/Ki/Kd，計算 RMS 誤差/平均自適應增益，輸出總表與疊圖。
- 共用工具：`scripts/common/telemetry.py` 封裝 CSV 讀取與行為提取，`analyze_run.py`、`analyze_compare.py`、`ab_compare.py`、`run_pid_sweep.py` 皆共用。

## 執行範例
- 平台（mock + bench + 自適應 + CSV）

  ```bash
  cargo run -p platform_rpi -- \
    --bench --bench-tau 0.8 --bench-gain 0.6 \
    -v 0.6 --hz 100 --seconds 8 \
    --adaptive --e-small 0.02 --e-large 0.20 --gain-min 0.6 --gain-max 1.2 \
    --v-profile Sin --sin-amp 0.3 --sin-freq 0.2 --sin-bias 0.4 \
    --csv run/telemetry.csv
  ```

- 2D 模擬（固定步進，視窗可視化）

  ```bash
  cargo run -p sim2d -- --config configs/sim2d.example.toml
  # 可覆寫：cargo run -p sim2d -- --config configs/sim2d.example.toml --hz 200 --obstacle 420,-50
  ```

- 參數掃描（以平台二進位跑多組 PID）

  ```bash
  python scripts/run_pid_sweep.py --extra "--bench --bench-tau 0.8 --bench-gain 0.6 -v 0.6"
  ```

## 擴充點與實作指北
- 新硬體（馬達/距離）：在 `drivers/` 新增型別並實作 `Motor` 與 `DistanceSensor`，並於 `DriverFactory` 登錄，執行端即可取得對應 handle。
- 新遙測輸出：實作 `hal::Telemetry`，在平台/模擬迴圈中呼叫 `record(...)` 或使用現有 CSV writer。
- 新控制器：在 `r2_core::control` 新增模組與型別，於平台/模擬接上即可（保留 `FailSafe` 最終裁切）。
- 新速度曲線：擴充 `r2_core::profile::VProfile` / `desired_v`，兩平臺會自動同步；再補 CLI 旗標即可。
- 物理/感測：在 `sim2d` 擴充 `physics.rs`、`sensing.rs` 或新增系統/資源。

## 已知限制 / 待辦
- `sim2d/src/logging.rs` 的 `flush_telemetry` 仍為空；若要輸出 CSV，可直接用 `TelemetryWriter` 或擴充該系統。
- `platform_rpi` 目前在 `main.rs` 仍直接使用 mock drivers；若要切換到 DriverFactory / 實機 driver，需額外串接。
- Raspberry Pi 實機驅動仰賴 `rppal`，未在 CI 裡自動測試，佈署前請在硬體上驗證。
- FailSafe 策略為線距一維判斷，尚未考慮多感測器或角度資訊。

---

若要把本文件拆分成更細的 `API.md` 或加入更完整的時序圖、KPI 定義，請提出需求我再補上。
