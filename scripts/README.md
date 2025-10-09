# Scripts Toolkit

這裡整理專案內常用的命令腳本與分析流程。所有腳本預設在專案根目錄執行，並共享 `scripts/common/telemetry.py` 裡的 CSV 解析工具。

---

## 📁 目錄概覽

- `common/telemetry.py`：核心工具，提供 `resolve_csv_path`、`load_dataframe`、`telemetry_vectors` 等函式，負責讀取/解析統一的 telemetry CSV 欄位。
- `plot_telemetry.py`：讀取多組 CSV（預設 `out_const.csv/out_sin.csv/out_step.csv`）並繪製輸出/目標疊圖。
- `analyze_run.py`：針對單一 CSV 計算 FailSafe 前的 RMS、平均誤差、平均自適應增益並產生圖表。
- `analyze_compare.py`：比較 Const/Sin/Step 三種 profile 的差異，列出 RMS 與 Step KPI（上升時間、超調、穩定時間、穩態誤差）。
- `ab_compare.py`：讀取 baseline/adaptive 的 sweep summary，列出改善名單並輸出最佳曲線疊圖。
- `run_pid_sweep.py`：以 `cargo run -p platform_rpi` 掃描多組 PID 組合，輸出總表與疊圖、個別 CSV。

---

## 🧪 產生測試資料

### Platform RPi（bench + mock）
```bash
cargo run -p platform_rpi -- \
  --bench --bench-tau 0.8 --bench-gain 0.6 \
  -v 0.6 --hz 50 --seconds 6 \
  --adaptive --e-small 0.02 --e-large 0.20 --gain-min 0.6 --gain-max 1.2 \
  --v-profile sin --sin-amp 0.3 --sin-freq 0.2 --sin-bias 0.4 \
  --csv run/sample_platform.csv --quiet
```

### Sim2D 模擬
```bash
cargo run -p sim2d -- \
  --hz 100 -v 0.6 \
  --v-profile sin --sin-amp 0.3 --sin-freq 0.2 --sin-bias 0.4 \
  --safety-margin-ratio 0.1 \
  --obstacle 300,0 --obstacle 350,120 --obstacle 420,-100
# 預設輸出 run/sim.csv，可用 --csv 指定
```

---

## 📈 單檔分析與繪圖

### `plot_telemetry.py`
```bash
python scripts/plot_telemetry.py --csv run/sample_platform.csv
python scripts/plot_telemetry.py --csv run/sim.csv
# 若省略 --csv，會依序尋找：run.csv → run/telemetry.csv → run/sim.csv
```

### `analyze_run.py`
```bash
python scripts/analyze_run.py --csv run/sample_platform.csv
# 或省略 --csv，腳本會用 common.telemetry.resolve_csv_path() 尋找常見路徑
```
- 計算 FailSafe 前 RMS、平均 |誤差|、平均 adapt_gain，並輸出圖表。
- `meas_left/meas_right` 存在時會優先使用；否則 fallback 到 `left/right` 平均。

### `analyze_compare.py`
```bash
# 會自動尋找 out_const/out_sin/out_step（支援放在 run/ 子目錄）
python scripts/analyze_compare.py
# 或明確指定路徑
python scripts/analyze_compare.py --const run/out_const.csv --sin run/out_sin.csv --step run/out_step.csv --out compare.png
```
- 列出 Const/Sin/Step 的 RMS（FailSafe 前）與 Step KPI（上升時間、超調%、穩定時間、穩態誤差）。

### `plot_telemetry.py`（多檔疊圖）
```bash
python scripts/plot_telemetry.py
```
僅繪圖、不計算 KPI；預設輸出 `compare.png`。

---

## 🔁 PID 掃描與 A/B 比較

### `run_pid_sweep.py`
```bash
python scripts/run_pid_sweep.py \
  --adaptive \
  --extra "--bench --bench-tau 0.8 --bench-gain 0.6 -v 0.6" \
  --out-dir run \
  --plot sweep_compare.png \
  --summary sweep_summary.csv
```
- 掃描預設或指定的 Kp/Ki/Kd 範圍（格式 `--kp start,stop,step`）。
- 產出個別 CSV、總表與疊圖；總表欄位含 RMS、Mean|Err|、MeanAdaptGain、FailSafe 時間。

### `ab_compare.py`
```bash
python scripts/ab_compare.py
```
- 讀取 `run/base/base_summary.csv` 與 `run/adapt/adapt_summary.csv`（由 `run_pid_sweep.py` 輸出）。
- 顯示 RMS 改善排行並輸出最佳組疊圖 `run/ab_best_overlay.png`。

---

## 🧩 Telemetry CSV 統一格式

所有腳本都假設 CSV 表頭為：
```
t,dt,desired_v,left,right,distance,state,meas_left,meas_right,err,adapt_gain
```
- `sim2d` 若無量測，`meas_left/meas_right` 會是 `NaN`；`common.telemetry.measured_velocity()` 會自動 fallback 到 `left/right` 平均。
- 若 CSV 缺少 `err` 或 `adapt_gain` 欄位，解析器會回傳預設值（例如全 1.0 的 adapt_gain）。

---

## 🧭 小提醒

- `scripts/common/telemetry.py` 可以直接 import（例如 `from scripts.common import telemetry_vectors`）。
- 若要新增腳本，建議共用 `telemetry_vectors` 來維持欄位解析一致性。
- 所有腳本皆可接受絕對或相對路徑；若路徑不存在會提示錯誤。
