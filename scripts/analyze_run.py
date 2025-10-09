#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
analyze_run.py
------------------------------------------------------------
專門分析單一 run.csv：
- 繪製：
  1) desired_v vs measured/output
  2) 誤差 (err) vs adaptive gain (adapt_gain)
- 計算：
  * RMS 誤差 (FailSafe 前)
  * 誤差平均值
  * adapt_gain 平均值
------------------------------------------------------------
用法：
    python scripts/analyze_run.py --csv run/sample_platform.csv
    # 若省略 --csv，將依序嘗試 run.csv → run/telemetry.csv → run/sim.csv
------------------------------------------------------------
"""

import argparse
import numpy as np
import matplotlib.pyplot as plt
import sys
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent
if str(SCRIPT_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPT_DIR))

from common.telemetry import load_telemetry, telemetry_vectors  # noqa: E402


def parse_args():
    p = argparse.ArgumentParser()
    p.add_argument("--csv", type=str, default=None, help="輸入的 CSV 檔案")
    p.add_argument("--out", type=str, default=None, help="輸出的圖檔（預設依 CSV 命名）")
    return p.parse_args()


def main():
    args = parse_args()
    df, csv_path = load_telemetry(args.csv)
    out_path = args.out or f"analyze_{csv_path.stem}.png"
    traces = telemetry_vectors(df)

    # 找 FailSafe 觸發時間
    brake_time = traces.failsafe_time()
    mask = traces.run_mask()

    # 計算 RMS 誤差
    err = traces.error
    adapt = traces.adapt

    if mask.any():
        rms = float(np.sqrt(np.mean(err[mask] ** 2)))
        mean_err = float(np.mean(np.abs(err[mask])))
        mean_gain = float(np.mean(adapt[mask]))
    else:
        rms = mean_err = mean_gain = np.nan

    print("=== CSV 分析結果 ===")
    print(f"檔案: {csv_path}")
    print(f"FailSafe 觸發時間: {'—' if np.isinf(brake_time) else round(float(brake_time), 3)} s")
    print(f"RMS 誤差: {rms:.4f} m/s")
    print(f"平均 |誤差|: {mean_err:.4f} m/s")
    print(f"平均 adapt_gain: {mean_gain:.3f}")

    # 畫圖
    fig, (ax1, ax2) = plt.subplots(2, 1, figsize=(14, 8), sharex=True)

    # 上圖：速度曲線
    ax1.plot(traces.time, traces.measured, label="Output/Measured")
    ax1.plot(traces.time, traces.desired, "--", label="Desired")
    if np.isfinite(brake_time):
        ax1.axvline(brake_time, linestyle="--", alpha=0.3, color="red", label="FailSafe")
    ax1.set_ylabel("Velocity (m/s)")
    ax1.set_title("Velocity response")
    ax1.grid(True)
    ax1.legend(loc="best")

    # 下圖：誤差 vs adapt_gain
    ax2.plot(traces.time, err, label="Error (desired - measured)")
    ax2.plot(traces.time, adapt, label="Adaptive gain")
    if np.isfinite(brake_time):
        ax2.axvline(brake_time, linestyle="--", alpha=0.3, color="red")
    ax2.set_xlabel("Time (s)")
    ax2.set_ylabel("Error / Gain")
    ax2.set_title("Error and Adaptive Gain")
    ax2.grid(True)
    ax2.legend(loc="best")

    plt.tight_layout()
    plt.savefig(out_path, dpi=150)
    print(f"\n已輸出圖檔：{out_path}")
    plt.show()


if __name__ == "__main__":
    main()
