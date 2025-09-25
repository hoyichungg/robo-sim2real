#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
plot_run.py
- 讀取 run.csv
- 繪製 output vs desired_v
- 同時畫出距離曲線（FailSafe 觸發時可看到 distance 收斂）
- 會輸出 run.png
"""

import os
import pandas as pd
import matplotlib.pyplot as plt

CSV_FILE = "run.csv"   # 你的輸入 CSV
OUT_IMG = "run.png"    # 輸出的圖檔

if not os.path.exists(CSV_FILE):
    raise FileNotFoundError(f"找不到 {CSV_FILE}")

# 讀取資料
df = pd.read_csv(CSV_FILE)
t = df["t"]
v_des = df["desired_v"]
v_out = (df["left"] + df["right"]) / 2.0
dist = df["distance"]

# 畫圖
fig, (ax1, ax2) = plt.subplots(2, 1, figsize=(12, 7), sharex=True)

# 速度比較
ax1.plot(t, v_out, label="output")
ax1.plot(t, v_des, "--", label="desired", alpha=0.7)
ax1.set_ylabel("Velocity (m/s)")
ax1.set_title("Controller response (run.csv)")
ax1.legend()
ax1.grid(True)

# 距離曲線
ax2.plot(t, dist, label="distance", color="tab:orange")
ax2.set_xlabel("Time (s)")
ax2.set_ylabel("Distance (m)")
ax2.legend()
ax2.grid(True)

plt.tight_layout()
plt.savefig(OUT_IMG, dpi=150)
plt.show()

print(f"已輸出 {OUT_IMG}")