#!/usr/bin/env python3
import os
import pandas as pd
import numpy as np
import matplotlib.pyplot as plt

# 尋找 telemetry.csv（腳本在根或 scripts/ 都可）
ROOT = os.path.dirname(os.path.abspath(__file__))
CSV = os.path.join(ROOT, "telemetry.csv")
if not os.path.exists(CSV):
    CSV = os.path.join(ROOT, "..", "telemetry.csv")
if not os.path.exists(CSV):
    raise FileNotFoundError("找不到 telemetry.csv，請把它放在專案根目錄，或把腳本放到專案根。")

df = pd.read_csv(CSV)

# 檢查欄位
cols = ["t", "dt", "desired_v", "left", "right", "distance", "state"]
missing = [c for c in cols if c not in df.columns]
if missing:
    raise ValueError(f"CSV 缺少欄位: {missing}\n目前欄位: {df.columns.tolist()}")

# 時間軸：若 t 幾乎不變，改用 dt 累積
t = df["t"].to_numpy(dtype=float)
if len(t) == 0 or np.nanmax(t) - np.nanmin(t) < 1e-6:
    t = np.cumsum(df["dt"].to_numpy(dtype=float))
    if len(t):
        t -= t[0]

# 取資料
v_des = df["desired_v"].to_numpy(dtype=float)
v_l = df["left"].to_numpy(dtype=float)
v_r = df["right"].to_numpy(dtype=float)
dist = df["distance"].to_numpy(dtype=float)
state = df["state"].astype(str).to_list()

# 找出非 Run 區段（連續片段）
def non_run_spans(t_arr, state_list):
    spans = []
    if len(t_arr) < 2:
        return spans
    in_span = False
    start = None
    for i in range(len(t_arr) - 1):
        non_run = (state_list[i] != "Run")
        if non_run and not in_span:
            in_span = True
            start = t_arr[i]
        if in_span and not non_run:
            in_span = False
            spans.append((start, t_arr[i]))
    if in_span:
        spans.append((start, t_arr[-1]))
    return spans

spans = non_run_spans(t, state)

# 畫圖
fig, (ax1, ax2) = plt.subplots(2, 1, figsize=(12, 7), sharex=True)

# 上：速度 + FailSafe 陰影
ax1.plot(t, v_des, linestyle="--", label="desired_v")
ax1.plot(t, v_l, label="left")
ax1.plot(t, v_r, label="right")
for s, e in spans:
    ax1.axvspan(s, e, color="red", alpha=0.15, linewidth=0)
ax1.set_ylabel("Velocity (m/s)")
ax1.set_title("Velocity tracking")
ax1.grid(True)
ax1.legend(loc="upper right")

# 下：距離 + FailSafe 陰影
ax2.plot(t, dist, label="distance (m)")
for s, e in spans:
    ax2.axvspan(s, e, color="red", alpha=0.18, linewidth=0)
ax2.set_xlabel("Time (s)")
ax2.set_ylabel("Distance (m)")
ax2.set_title("Distance & safety state (red = EmergencyBrake/SafeStop)")
ax2.grid(True)
ax2.legend(loc="upper right")

plt.tight_layout()

# 存檔到和 CSV 同一層
out_png = os.path.join(os.path.dirname(CSV), "telemetry.png")
plt.savefig(out_png, dpi=150)
print(f"Saved figure to: {out_png}")

plt.show()