# 控制參數調整指南（TUNING）

本文件提供 sim2d 與 platform_rpi 的 PID/FailSafe/plant 調參流程、建議範圍與常見症狀對應。

## 目標與環境
- 目標：在固定取樣頻率下，讓速度追踐 desired_v，兼顧穩定、反應與安全停車。
- 環境：
  - sim2d：一階 plant（`--tau`, `--plant-gain`），像素/公尺換算 `--px-per-m`。
  - platform_rpi：mock/bench 或真硬體（drivers/rpi 特性）。

## 預設與建議值（sim2d）
- CLI 預設：`--kp 0.6 --ki 0.05 --kd 0.0`（保守穩定，建議起步）。
- 100 Hz 實測可行組：`--kp 0.8 --ki 0.2 --kd 0.0 --tau 0.2 --plant-gain 1.0`。
- 參考範圍（不同 Hz）
  - 50 Hz：`kp 0.6–0.8`、`ki 0.03–0.10`、`kd 0.0–0.005`
  - 100 Hz：`kp 0.6–1.0`、`ki 0.05–0.20`、`kd 0.0–0.010`
  - 200 Hz：`kp 0.4–0.8`、`ki 0.02–0.10`、`kd 0.0–0.020`

## 調參流程（建議步驟）
1) 關掉 D 項：`--kd 0`。先調 P/I，避免微分放大噪音/數值差分造成打頂。
2) 放寬安全限制（驗證控制本體會動）：`--threshold 0 --hysteresis 0 --safety-margin-ratio 0`。
3) 掃 `kp`：逐步增加，直到響應足夠但不顯著震盪。
4) 加 `ki`：消除穩態誤差，小幅增加並觀察是否引入緩慢震盪（過大會累積 windup）。
5) 視需要加極小 `kd`（<= 0.01@100Hz）：僅在超調或快速變化時明顯受益。
6) 調 plant：
   - `tau` 越小反應越快（相當於系統慣性變小）。
   - `plant-gain` 建議 1.0 起步；偏離 1 代表命令→速度的靈敏度縮放。
7) 恢復安全：設定 `--threshold`（如 0.25 m）、`--hysteresis`（如 0.05 m）、`--safety-margin-ratio`（如 0.1）。

## FailSafe 與距離估測
- 門檻：`--threshold`（觸發急停）、`--hysteresis`（解除回滯）。
- 安全緩衝：`--safety-margin-ratio`（車長 × 比例，從量測距離扣除）。
- sim2d 感測：
  - 車頭射線起點在「車頭前半車長」。
  - 掃描所有 `Obstacle`（AABB），取最近交點；無命中回 +∞。
  - 可用 gizmos 觀察射線（橘/命中高亮）、命中點（黃）、障礙 AABB（多色）。

## 常見症狀與對應
- 原地抖動、輸出在 ±1 來回跳：
  - 減小 `kp`，將 `kd` 設為 0，或降低 `ki`。
  - 檢查是否已達輸出上限（`Pid::with_output_limits`）。
- 反應太慢：
  - 增加 `kp` 或 `ki`；降低 `tau`；檢查 `plant-gain` 是否過小。
- 超調大：
  - 降低 `kp` 或加極小 `kd`；適度提高 `tau`。
- 一進場就急停：
  - 放寬 `threshold` 或將 `safety-margin-ratio` 調低/0，確認距離量測是否合理。

## 產生 CSV 與分析
- 產生 sim2d CSV：
```bash
cargo run -p sim2d -- --hz 100 -v 0.6 \
  --kp 0.8 --ki 0.2 --kd 0.0 --tau 0.2 --plant-gain 1.0 \
  --safety-margin-ratio 0.1 --csv run/sim.csv
```
- 分析 KPI/繪圖：
```bash
python scripts/plot_run.py --csv run/sim.csv
python scripts/analyze_run.py --csv run/sim.csv
```
- Profile 對比（platform_rpi 範例）：
```bash
python scripts/analyze_compare.py \
  --const run/out_const.csv --sin run/out_sin.csv --step run/out_step.csv
```

## 自動掃參（platform_rpi）
- 使用 `scripts/run_pid_sweep.py`：
```bash
python scripts/run_pid_sweep.py \
  --extra "--bench --bench-tau 0.8 --bench-gain 0.6 -v 0.6" \
  --out-dir run --plot sweep_compare.png --summary sweep_summary.csv
```
- 比較 baseline/adaptive：`scripts/ab_compare.py`

