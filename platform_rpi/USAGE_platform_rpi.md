# 📘 使用教學：platform_rpi (PID + FailSafe + Bench 模式 + 自適應)

## 1. 基本功能
這個 binary (`platform_rpi`) 實作了一個最小控制迴圈：
- **PID 控制器**（Kp, Ki, Kd 可調）。
- **FailSafe**：距離過近時觸發停車。
- **Telemetry 記錄**：把每次迴圈的輸出、量測速度、狀態等寫到 CSV。
- **Bench 模式**（模擬）：用一階系統模型 (τ, gain) 將馬達輸出轉換成模擬的「量測速度」。
- **自適應模式**：根據誤差大小動態調整輸出增益，改善跟隨精度。

---

## 2. 主要參數說明
### 一般控制
- `--kp, --ki, --kd`：PID 係數。
- `--hz`：控制迴圈頻率 (Hz)。
- `--seconds`：總執行時間 (秒)。
- `-v`：期望速度 (m/s)，或搭配 `--v-profile` 使用。

### Profile
- `--v-profile const`：常值速度（預設）。
- `--v-profile step --step-at 1.5`：1.5 秒時從 0 → v。
- `--v-profile sin --sin-amp 0.3 --sin-freq 0.2 --sin-bias 0.4`：正弦速度曲線。

### Bench 模式
- `--bench`：啟用 bench 模擬。
- `--bench-tau 0.8`：模擬的一階系統時間常數 τ。
- `--bench-gain 0.6`：模擬的輸出增益 (u → v)。

### 自適應模式
- `--adaptive`：啟用自適應輸出增益。
- `--e-small 0.02`：小誤差閾值（誤差 ≤ e_small → 使用最小增益）。
- `--e-large 0.20`：大誤差閾值（誤差 ≥ e_large → 使用最大增益）。
- `--gain-min 0.6`：最小增益。
- `--gain-max 1.2`：最大增益。  
➡ 中間區間會用線性內插。

---

## 3. 常見使用方式

### (A) 常值速度，無 bench
```bash
cargo run -p platform_rpi -- -v 0.8 --seconds 8   --kp 0.4 --ki 0.05 --kd 0.04   --csv runs/const.csv --quiet
```

### (B) Bench 模擬
```bash
cargo run -p platform_rpi -- --bench --bench-tau 0.8 --bench-gain 0.6   -v 0.6 --seconds 8   --kp 0.4 --ki 0.05 --kd 0.04   --csv runs/bench.csv --quiet
```

### (C) Bench + 自適應
```bash
cargo run -p platform_rpi -- --bench --bench-tau 0.8 --bench-gain 0.6   -v 0.6 --seconds 8   --kp 0.4 --ki 0.05 --kd 0.04   --adaptive --e-small 0.02 --e-large 0.20 --gain-min 0.6 --gain-max 1.2   --csv runs/adapt.csv --quiet
```

---

## 4. 結果分析

1. 每次執行會輸出一個 **CSV 檔**，包含：
   ```
   t,dt,desired_v,left,right,distance,state,meas_left,meas_right
   ```
   - `desired_v`：期望速度。
   - `left,right`：實際輸出指令（已套用自適應）。
   - `meas_left,meas_right`：bench 模式下的模擬量測速度。

2. 可用分析腳本比較：
   - `scripts/analyze_compare.py` → 疊圖 + KPI。
   - `scripts/run_pid_sweep.py` → 一次掃多組 PID，顯示 RMS 誤差表。

---

## 5. 建議流程

1. **先跑 Bench baseline**（無 adaptive）。  
2. **再跑 Bench + adaptive**，比較 RMS 誤差是否下降。  
3. 若要大量測試，直接用 `run_pid_sweep.py`，在 `EXTRA_FLAGS` 加 `--adaptive` 即可。  
