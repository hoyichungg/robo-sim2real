# Scripts Quick Recipes

常用資料產生、繪圖與分析指令彙整。下列指令皆以專案根目錄為工作目錄。

## 產生 CSV（Platform RPi／bench + mock）

```bash
cargo run -p platform_rpi -- \
  --bench --bench-tau 0.8 --bench-gain 0.6 \
  -v 0.6 --hz 50 --seconds 6 \
  --adaptive --e-small 0.02 --e-large 0.20 --gain-min 0.6 --gain-max 1.2 \
  --v-profile sin --sin-amp 0.3 --sin-freq 0.2 --sin-bias 0.4 \
  --csv run/sample_platform.csv --quiet
```

備註：`--v-profile` 請使用小寫（`const|step|sin`）。

## 產生 CSV（Sim2D 桌面模擬）

```bash
cargo run -p sim2d -- \
  --hz 100 -v 0.6 \
  --v-profile sin --sin-amp 0.3 --sin-freq 0.2 --sin-bias 0.4 \
  --safety-margin-ratio 0.1 \
  --obstacle 300,0 --obstacle 350,120 --obstacle 420,-100
# 預設輸出：run/sim.csv（可用 --csv 指定）
```

- `--safety-margin-ratio`：感測距離扣除「車長 × 比例」做保守緩衝（預設 0.1）。
- `--obstacle X,Y`：可重複提供多個障礙座標（像素）。未提供時有 3 個預設位置。

## 繪圖（單檔）

```bash
# 指定平台輸出 CSV
python scripts/plot_run.py --csv run/sample_platform.csv
# 指定模擬輸出 CSV
python scripts/plot_run.py --csv run/sim.csv
# 若省略 --csv，會依序尋找：run.csv → run/telemetry.csv → run/sim.csv
```

- 參數：`--out my_plot.png` 可自訂圖檔名稱。

## 單檔分析（KPI + 圖）

```bash
# 指定 CSV
python scripts/analyze_run.py --csv run/sample_platform.csv
# 或省略 --csv，由腳本自動尋找常見路徑
python scripts/analyze_run.py
```

- 腳本會計算 FailSafe 觸發前的 RMS 誤差、平均 |誤差|、平均自適應增益，並輸出對應圖檔。
- 若 CSV 含 `meas_left/meas_right`（platform bench），會優先使用它們；否則退回 `left/right` 平均。

## 複數 Profile 對比（Const / Sin / Step）

1) 先產生三個 CSV（以 platform_rpi 為例）：

```bash
cargo run -p platform_rpi -- --bench --bench-tau 0.8 --bench-gain 0.6 -v 0.6 --seconds 8 --v-profile const --csv out_const.csv --quiet
cargo run -p platform_rpi -- --bench --bench-tau 0.8 --bench-gain 0.6 -v 0.6 --seconds 8 --v-profile sin   --sin-amp 0.3 --sin-freq 0.2 --sin-bias 0.4 --csv out_sin.csv --quiet
cargo run -p platform_rpi -- --bench --bench-tau 0.8 --bench-gain 0.6 -v 0.6 --seconds 8 --v-profile step  --step-at 1.5 --csv out_step.csv --quiet
```

2) 執行對比分析與疊圖：

```bash
# 會自動尋找 out_*.csv 或 run/out_*.csv
python scripts/analyze_compare.py
# 或明確指定路徑
python scripts/analyze_compare.py --const run/out_const.csv --sin run/out_sin.csv --step run/out_step.csv --out compare.png
```

- 會列出三條曲線的 RMS 誤差（FailSafe 前）與 Step 測試的 KPI（上升時間、超調%、穩定時間、穩態誤差）。

## Step 專用 KPI（analyze_step.py）

此腳本預設讀取 `out_step.csv`，請先產出或改檔名：

```bash
# 產出 Step CSV（platform_rpi 範例）
cargo run -p platform_rpi -- \
  --bench --bench-tau 0.8 --bench-gain 0.6 \
  -v 0.6 --seconds 8 --v-profile step --step-at 1.5 \
  --csv out_step.csv --quiet

# 計算 KPI（上升時間/超調/穩定時間/穩態誤差），只統計 FailSafe 前
python scripts/analyze_step.py
```

若你用其他檔名，請開啟 `scripts/analyze_step.py` 修改 `CSV_FILE`。

## 多組掃描 A/B 比較（ab_compare.py）

比較 baseline（未開 adaptive）與 adaptive 的掃描結果（RMS 誤差 vs PID 組），並畫出兩邊最佳組的疊圖。

1) 先產生 baseline 與 adaptive 的 sweep（會輸出 summary 到 `run/base` 與 `run/adapt`）：

```bash
# baseline（未開 adaptive）
python scripts/run_pid_sweep.py \
  --extra "--bench --bench-tau 0.8 --bench-gain 0.6 -v 0.6" \
  --out-dir run \
  --plot base_overlay.png \
  --summary base_summary.csv

# adaptive（開啟 --adaptive）
python scripts/run_pid_sweep.py \
  --adaptive \
  --extra "--bench --bench-tau 0.8 --bench-gain 0.6 -v 0.6" \
  --out-dir run \
  --plot adapt_overlay.png \
  --summary adapt_summary.csv
```

2) 執行 A/B 比較（預設讀 `run/base/base_summary.csv` 與 `run/adapt/adapt_summary.csv`）：

```bash
python scripts/ab_compare.py
```

輸出：
- 顯示改善名單（RMS 降幅由大到小）
- 繪製 baseline/adaptive 的最佳組疊圖

## 三檔 Telemetry 疊圖（plot_telemetry.py）

此腳本會讀取 `out_const.csv`、`out_sin.csv`、`out_step.csv` 並輸出 `compare.png`（與 `analyze_compare.py` 類似，偏向純繪圖）。

```bash
python scripts/plot_telemetry.py
```

## PID 參數掃描（grid sweep）

```bash
python scripts/run_pid_sweep.py \
  --adaptive \
  --extra "--bench --bench-tau 0.8 --bench-gain 0.6 -v 0.6" \
  --out-dir run \
  --plot sweep_compare.png \
  --summary sweep_summary.csv
```

- 會以 `cargo run -p platform_rpi -- ...` 自動掃描 Kp/Ki/Kd 組合，輸出總表與疊圖。
- 參數 `--kp/--ki/--kd` 可覆蓋預設範圍（格式 `start,stop,step`）。

## CSV 欄位（統一）

CSV 表頭統一為：

```
t,dt,desired_v,left,right,distance,state,meas_left,meas_right,err,adapt_gain
```

- sim2d 會將 `meas_left/meas_right` 填 `NaN`（平台 bench 為實值）。
- 分析腳本會自動處理 `meas_*` 缺失或全為 `NaN` 的情況。

---

若你想把這份 Quick Recipes 從專案根的 README 中移除或只保留連結，請告訴我，我可以同步調整根 README。
