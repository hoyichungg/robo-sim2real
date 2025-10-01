# robo-sim2real 架構說明

本文件說明專案目標、模組切分、控制資料流、擴充點（API 開口）與開發/執行指引，協助你在「實體（Raspberry Pi）」與「2D 視覺模擬」之間共用核心控制邏輯。

## 目標
- 在單一核心庫上共用 PID 控制與安全保護（FailSafe）。
- 以 HAL（硬體抽象層）隔離驅動層，使真硬體與模擬環境可互換。
- 以簡單 CLI/CSV/腳本完成迴圈測試、掃參與分析繪圖。

## 專案結構
- `r2_core/`：核心庫（控制、HAL、模型/遙測骨架）
- `drivers/`：對 HAL 的實作（mock 與 RPi 佔位 stub）
- `platform_rpi/`：實體平台/命令列執行（含 bench 一階模型、自適應增益、CSV）
- `sim2d/`：Bevy 2D 視覺模擬（感測 → 控制 → 物理 → 紀錄 的固定步進管線）
- `tests/`：針對 FailSafe 與 PID 的單元測試
- `scripts/`：CSV 分析與繪圖（單次、A/B 與 sweep）

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
- `r2_core/src/control/safety.rs`：`FailSafe` 狀態機（Run / EmergencyBrake / SafeStop），`update(...)` 依距離/錯誤更新狀態，`clamp_speed(v)` 做最終零速裁切，`reset(...)` 需在安全距離外手動解除（v0 不自動）。
- `r2_core/src/control/controller.rs`：`Controller::tick(desired_v, dt, distance)` 組合 PID + FailSafe + 差速運動學（先只用線速，角速保留）。內部用簡單低通模擬量測慣性。

注意：`r2_core/src/control/telemetry.rs` 內有一個資料結構 `struct Telemetry`，它與 HAL 的 `trait Telemetry` 僅同名不同模組，語意不同（前者為 CSV 欄位暫存，後者為抽象輸出管道）。

### 自適應輸出增益（Adaptive Gain）
- `platform_rpi/src/adaptive.rs`：`map_gain(|e|, e_small..e_large, gain_min..gain_max)` 把誤差絕對值線性映射到輸出增益（區間外夾限）。
- `sim2d/src/control.rs`：也內建一份等效的自適應計算流程（直接寫在系統中）。

## 平台與資料流

### 平台（Raspberry Pi/命令列）
檔案：`platform_rpi/src/main.rs`
- CLI 旗標（目標速度、PID、頻率、秒數、FailSafe 門檻/回滯、CSV、速度曲線、bench、一階參數、自適應參數）。
- 迴圈每步：
  1) 計算目標速度 `profile::desired_v(...)`（Const / Step / Sin）。
  2) 讀距離 `DistanceSensor::distance_m()` → `FailSafe.update(...)`。
  3) `Controller.tick(...)` 得到未自適應的 `(left,right)` 與 `SafetyState`。
  4) 若 `--bench`：用一階模型（`tau`, `gain`）把命令轉成「量測速度」估測誤差。
  5) 若 `--adaptive`：以 `map_gain(...)` 依 |err| 調整輸出增益並夾限到 [-1,1]。
  6) 呼叫 `Motor::set_wheel_speeds(...)`（預設使用 `drivers::mock::MockMotor`）。
  7) 累積遙測列，最後輸出 CSV（`run/telemetry.csv` 或自訂路徑）。

速度曲線與遙測：
- `platform_rpi/src/profile.rs`：`VProfile`（Const/Step/Sin）、`desired_v(...)`、平台版 `Telemetry { t, dt, desired_v, left, right, distance, state, meas_left/right, err, adapt_gain }`。

### 2D 模擬（Bevy）
檔案：`sim2d/src/*.rs`
- 啟動：`main.rs` 建立 App → 插入 `SimClock`、`DistanceSense`、`TelemetryWriter`、`RuntimeCfg` → 設定 FixedUpdate 管線。
- 固定步進順序：
  1) `sensing::sense_distance`：以車頭射線對障礙物 AABB 做簡化 Ray-AABB 估距，寫入 `DistanceSense`。
  2) `control::control_step`：`Pid` →（可選）自適應 → `FailSafe.clamp_speed`，把 u 寫回 `Velocity.v`。
  3) `physics::integrate_kinematics`：用一階系統把命令濾成量測速度並整步到位置（依 `Heading` 前進）。
  4) `logging::flush_telemetry`：預留（可改為使用 `TelemetryWriter` 寫檔）。

組件與資源：
- `components.rs`：`Car`、`Obstacle`、`Velocity{v,omega}`、`Heading(Vec2)`。
- `resources.rs`：`SimClock{t,dt}`、`DistanceSense(f32)`、`TelemetryWriter`（含 CSV header 與 `write(...)`）。
- `config.rs`：`Cli` 與 `RuntimeCfg`（PID、FailSafe、自適應、plant、像素/公尺、CSV），`desired_speed(...)`（目前 const/step）。

## 驅動（Drivers）
- `drivers/src/mock.rs`：
  - `MockMotor`：印出左右馬達命令（便於除錯）。
  - `MockSensor`：隨時間讓距離從 1.0m 線性降到 0.1m（測試 FailSafe 觸發）。
- `drivers/src/rpi.rs`：
  - `RpiMotorStub`、`RpiDistanceStub`：佔位實作，日後接 `rppal`/GPIO/PWM/I2C 等。

欲接真硬體：在此檔實作 `Motor` 與 `DistanceSensor`，然後在 `platform_rpi/src/main.rs` 換成你的型別即可。

## 測試
- `tests/safety_failsafe.rs`：覆蓋錯誤/NaN/負值急停、閾值觸發與 SafeStop、回滯解除條件等。
- `tests/control_pid.rs`：`Pid` 在一階系統上的收斂性（含輸出/積分夾限）。

## 腳本與分析
- `scripts/plot_run.py`：讀單一 `run.csv` 繪製輸出 vs 期望與距離，輸出 `run.png`。
- `scripts/run_pid_sweep.py`：以 `cargo run -p platform_rpi -- ...` 掃描 Kp/Ki/Kd，計算 RMS 誤差/平均自適應增益，輸出總表與疊圖。
- 其它：`analyze_run.py`、`analyze_compare.py`、`ab_compare.py`、`plot_telemetry.py` 提供不同型態的比較與繪圖。

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
  cargo run -p sim2d -- --hz 100 -v 0.6 --threshold 0.25 --hysteresis 0.05 
  ```

- 參數掃描（以平台二進位跑多組 PID）

  ```bash
  python scripts/run_pid_sweep.py --extra "--bench --bench-tau 0.8 --bench-gain 0.6 -v 0.6"
  ```

## 擴充點與實作指北
- 新硬體（馬達/距離）：在 `drivers/` 新增型別並實作 `Motor` 與 `DistanceSensor`，於執行端切換使用。
- 新遙測輸出：實作 `hal::Telemetry`，在平台/模擬迴圈中呼叫 `record(...)` 或使用現有 CSV writer。
- 新控制器：在 `r2_core::control` 新增模組與型別，於平台/模擬接上即可（保留 `FailSafe` 最終裁切）。
- 新速度曲線：擴充 `platform_rpi/src/profile.rs` 或 `sim2d/src/config.rs`（兩邊目前各自維護）。
- 物理/感測：在 `sim2d` 擴充 `physics.rs`、`sensing.rs` 或新增系統/資源。

## 已知限制 / 待辦
- `DifferentialKinematics` 在 `r2_core/src/control/controller.rs` 與 `r2_core/src/model.rs` 各有一份，未整併。
- `sim2d/src/logging.rs` 的 `flush_telemetry` 仍為空；若要輸出 CSV，可直接用 `TelemetryWriter`。
- `drivers/src/rpi.rs` 目前為 stub，需要串接實際 RPi 周邊（GPIO/PWM/I2C 等）。
- FailSafe 已支援「距離 > threshold + hysteresis」自動解除；如需不同策略可再擴充。

---

若要把本文件拆分成更細的 `API.md` 或加入更完整的時序圖、KPI 定義，請提出需求我再補上。
