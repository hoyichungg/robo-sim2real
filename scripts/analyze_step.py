#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Step 響應 KPI 計算（支援 FailSafe）
- 讀取 out_step.csv
- 自動偵測階躍開始時刻
- 若觸發安全制動（state != "Run"），只在觸發以前的資料計算 KPI
- 使用線性內插估算 10%/90% 交越時間，避免取樣造成 0.000s 的誤判
"""

import os
import numpy as np
import pandas as pd

# === 檔名（可改） ===
CSV_FILE = "out_step.csv"

if not os.path.exists(CSV_FILE):
    raise FileNotFoundError(f"找不到 {CSV_FILE}")

# === 讀取 CSV ===
df = pd.read_csv(CSV_FILE)

# 基本欄位
t = df["t"].to_numpy(dtype=float)
v_des = df["desired_v"].to_numpy(dtype=float)
v_out = ((df["left"] + df["right"]) / 2.0).to_numpy(dtype=float)

# 安全狀態（若無此欄位就當全部都是 Run）
state = df["state"].astype(str).to_numpy() if "state" in df.columns else np.array(["Run"] * len(df))

# === 辅助函式：線性內插找 y(t) 穿越 level 的時刻 ===
def first_cross_time(t, y, level, direction="up"):
    """
    取得第一次穿越 level 的時間（線性內插）
    direction:
      - "up": 從低於 level 到高於（或等於）level 的第一次交越
      - "down": 反向
    回傳 float 或 np.nan
    """
    for i in range(len(y) - 1):
        y0, y1 = y[i], y[i + 1]
        if direction == "up":
            if (y0 < level) and (y1 >= level):
                # 線性內插 t = t0 + (level - y0) / (y1 - y0) * (t1 - t0)
                if y1 == y0:
                    return t[i + 1]
                return t[i] + (level - y0) / (y1 - y0) * (t[i + 1] - t[i])
        else:
            if (y0 > level) and (y1 <= level):
                if y1 == y0:
                    return t[i + 1]
                return t[i] + (level - y0) / (y1 - y0) * (t[i + 1] - t[i])
    return np.nan

# === 偵測階躍開始時刻（desired_v 的變化點） ===
# 用「相鄰差值」找出第一次大於 5% 目標值的跳變
v_target = float(np.max(v_des))  # 階躍目標值
if v_target <= 1e-9:
    raise ValueError("偵測到目標速度為 0，這不是有效的 step 測試。")

dv = np.diff(v_des)
step_idx_candidates = np.where(np.abs(dv) >= 0.05 * v_target)[0]
if len(step_idx_candidates) == 0:
    # 若抓不到跳變，嘗試用「第一個達到 10% 目標」當作步階開始
    step_start_time = first_cross_time(t, v_des, 0.1 * v_target, direction="up")
else:
    step_idx = int(step_idx_candidates[0])
    step_start_time = t[step_idx + 1]  # 跳變出現於區間 [idx, idx+1]，用右端點當作開始

# === 取出從階躍開始到安全觸發前的資料 ===
# 找第一個 state != "Run" 的時間，即安全觸發點
non_run = np.where(state != "Run")[0]
if len(non_run) > 0:
    brake_time = t[int(non_run[0])]
else:
    brake_time = np.inf  # 沒有觸發

# 只取 [step_start_time, brake_time) 的資料來計算控制性能 KPI
valid_mask = (t >= step_start_time) & (t < brake_time)
t_valid = t[valid_mask]
v_out_valid = v_out[valid_mask]

# 若有效區段太短，直接標示無法計算
if len(t_valid) < 3:
    print("=== Step 響應 KPI ===")
    print(f"目標速度: {v_target:.2f} m/s")
    print("⚠️ 有效資料不足（可能一開始就觸發安全制動），無法計算 KPI。")
    if np.isfinite(brake_time):
        print(f"安全制動觸發時間: {brake_time:.3f} s")
    raise SystemExit(0)

# 將時間原點平移到步階開始，讓 KPI 時間更直觀
t0 = step_start_time
t_rel = t_valid - t0

# === KPI 計算（皆在安全觸發前的有效窗內） ===
low = 0.1 * v_target
high = 0.9 * v_target

# 上升時間：10%→90%（線性內插）
t10 = first_cross_time(t_rel, v_out_valid, low, "up")
t90 = first_cross_time(t_rel, v_out_valid, high, "up")
rise_time = (t90 - t10) if (not np.isnan(t10) and not np.isnan(t90) and t90 >= t10) else np.nan

# 超調量（有效窗內的峰值相對目標的百分比）
v_max_valid = float(np.max(v_out_valid)) if len(v_out_valid) else np.nan
overshoot = max(0.0, (v_max_valid - v_target) / v_target * 100.0) if not np.isnan(v_max_valid) else np.nan

# 穩定時間：進入 ±2% band 且之後都在 band 內
band_upper = 1.02 * v_target
band_lower = 0.98 * v_target
settling_time = np.nan
# 找最後一次「離開 band」的時間點之後的第一個「全部在 band 內」時刻
inside = (v_out_valid >= band_lower) & (v_out_valid <= band_upper)
# 從頭掃描，找到第一個 index，使得其後皆為 True
if inside.any():
    last_false = np.where(~inside)[0]
    if len(last_false) == 0:
        # 一開始就都在 band 內
        settling_time = 0.0
    else:
        j = int(last_false[-1]) + 1
        if j < len(t_rel) and inside[j:].all():
            settling_time = float(t_rel[j])

# 穩態誤差：在有效窗的最後 10% 取平均
tail_n = max(5, int(len(v_out_valid) * 0.1))  # 至少取 5 個點
steady_mean = float(np.mean(v_out_valid[-tail_n:]))
ess = steady_mean - v_target

# === 輔助訊息：是否有安全觸發 ===
brake_note = ""
if np.isfinite(brake_time):
    brake_note = f"⚠️ 安全制動在 {brake_time:.3f} s 觸發，KPI 僅計算至觸發前的資料。"
    # 若 90% 尚未達到就被觸發，讓使用者知道為何 rise time 是 NaN
    if np.isnan(t90):
        brake_note += "（在觸發前尚未達到 90% 目標，因此上升時間為 NaN）"

# === 輸出結果 ===
print("=== Step 響應 KPI ===")
print(f"目標速度: {v_target:.2f} m/s")
print(f"步階開始時間: {t0:.3f} s")
if brake_note:
    print(brake_note)
print(f"上升時間 (10%→90%): {np.nan if np.isnan(rise_time) else round(rise_time, 3)} s")
print(f"超調量: {np.nan if np.isnan(overshoot) else round(overshoot, 2)} %")
print(f"穩定時間 (±2% band): {np.nan if np.isnan(settling_time) else round(settling_time, 3)} s")
print(f"穩態誤差: {ess:.3f} m/s")