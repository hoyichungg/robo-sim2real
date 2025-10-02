#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
plot_run.py
- 讀取 CSV 並繪製 output vs desired_v 與距離曲線
- 預設會嘗試以下路徑依序查找：
  1) --csv 指定路徑（若提供）
  2) run.csv（舊版預設）
  3) run/telemetry.csv（platform_rpi 預設）
  4) run/sim.csv（sim2d 預設）
- 可用 --out 覆蓋輸出圖檔名
"""

import os
import argparse
import pandas as pd
import matplotlib.pyplot as plt
from typing import Optional

def resolve_csv_path(arg_path: Optional[str]) -> str:
    if arg_path:
        if not os.path.exists(arg_path):
            raise FileNotFoundError(f"找不到 {arg_path}")
        return arg_path
    candidates = ["run.csv", "run/telemetry.csv", "run/sim.csv"]
    for p in candidates:
        if os.path.exists(p):
            return p
    raise FileNotFoundError("找不到 CSV，請用 --csv 指定（例如 run/sample_platform.csv）")

def main():
    ap = argparse.ArgumentParser(description="Plot velocity output vs desired and distance")
    ap.add_argument("--csv", type=str, default=None, help="輸入 CSV 路徑")
    ap.add_argument("--out", type=str, default=None, help="輸出圖檔名（預設依 CSV 決定或 run.png）")
    args = ap.parse_args()

    csv_path = resolve_csv_path(args.csv)
    out_img = args.out or (f"plot_{os.path.splitext(os.path.basename(csv_path))[0]}.png" if args.csv else "run.png")

    # 讀取資料
    df = pd.read_csv(csv_path)
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
    ax1.set_title(f"Controller response ({os.path.basename(csv_path)})")
    ax1.legend()
    ax1.grid(True)

    # 距離曲線
    ax2.plot(t, dist, label="distance", color="tab:orange")
    ax2.set_xlabel("Time (s)")
    ax2.set_ylabel("Distance (m)")
    ax2.legend()
    ax2.grid(True)

    plt.tight_layout()
    plt.savefig(out_img, dpi=150)
    plt.show()
    print(f"已輸出 {out_img}")

if __name__ == "__main__":
    main()
