#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
ab_compare.py
- 載入 baseline 與 adaptive 的 sweep 總表
- 依 (Kp,Ki,Kd) 合併並比較 RMS（Adaptive 是否下降）
- 印出改善名單（由大到小），並畫出兩邊的「最佳一組」疊圖
"""

import os
import sys
from pathlib import Path

import pandas as pd
import numpy as np
import matplotlib.pyplot as plt

SCRIPT_DIR = Path(__file__).resolve().parent
if str(SCRIPT_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPT_DIR))

from common.telemetry import load_dataframe, telemetry_vectors  # noqa: E402

BASE_DIR = "run/base"
ADPT_DIR = "run/adapt"
BASE_SUM = os.path.join(BASE_DIR, "base_summary.csv")
ADPT_SUM = os.path.join(ADPT_DIR, "adapt_summary.csv")

def resolve_csv_path(summary_path: str, csv_field: str) -> str:
    """
    將 summary 內的 CSV 欄位轉成「可用的絕對路徑」。
    兼容三種情況：
      1) 只有檔名：               pid_xxx.csv
      2) 相對路徑含資料夾：       run/base/pid_xxx.csv
      3) 已是絕對路徑：           /abs/path/to/pid_xxx.csv
    並避免像 run/base + run/base/pid_xxx.csv 這種重複前綴。
    """
    root = os.path.dirname(os.path.abspath(summary_path))
    c = csv_field.strip()

    candidates = []
    if os.path.isabs(c):
        candidates.append(c)
    else:
        # 1) 直接把它視為相對於 repo 的路徑
        candidates.append(os.path.abspath(c))
        # 2) 視為相對於 summary 所在資料夾
        candidates.append(os.path.abspath(os.path.join(root, c)))
        # 3) 只拿檔名接在 summary 所在資料夾
        candidates.append(os.path.abspath(os.path.join(root, os.path.basename(c))))

    for path in candidates:
        if os.path.exists(path):
            return path

    # 找不到就回傳最合理的一個，並讓上層在 read_csv 時爆錯比較好查
    return candidates[-1]

def load_summary(path, label):
    df = pd.read_csv(path)
    df["tag"] = label
    df["CSV_abs"] = df["CSV"].apply(lambda p: resolve_csv_path(path, p))
    return df

def prefer_meas_or_avg(df):
    """回傳 (t, v_des, v_out, state)；優先用 meas_left/right，否則用 (left+right)/2"""
    vectors = telemetry_vectors(df)
    return vectors.time, vectors.desired, vectors.measured, vectors.state

def rms_until_brake(t, v_des, v_out, state):
    nonrun = np.where(state != "Run")[0]
    brake = t[int(nonrun[0])] if len(nonrun) else np.inf
    m = (state == "Run") & (t < brake)
    if not m.any():
        return np.nan, brake
    return float(np.sqrt(np.mean((v_out[m] - v_des[m]) ** 2))), brake

def plot_best_curve(df_base, df_adpt):
    """抓兩邊 RMS 最小的那條，畫在一起對比"""
    def pick_best(df):
        tmp = df.dropna(subset=["RMS(m/s)"]).copy()
        if tmp.empty:
            return None
        best = tmp.iloc[tmp["RMS(m/s)"].astype(float).argmin()]
        return best

    b = pick_best(df_base)
    a = pick_best(df_adpt)
    if b is None or a is None:
        print("⚠️ 找不到最佳曲線（可能總表為空），跳過作圖。")
        return

    b_path = b["CSV_abs"]
    a_path = a["CSV_abs"]
    try:
        dfb = load_dataframe(b_path)
        dfa = load_dataframe(a_path)
    except FileNotFoundError:
        print("❌ ab_compare: 找不到 CSV 文件。請檢查以下路徑是否存在：")
        print("   BASE:", b_path)
        print("   ADAPT:", a_path)
        return

    tb, vdb, vb, sb = prefer_meas_or_avg(dfb)
    ta, vda, va, sa = prefer_meas_or_avg(dfa)

    plt.figure(figsize=(14, 6))
    plt.plot(tb, vb, label=f"BASE best Kp={b['Kp']},Ki={b['Ki']},Kd={b['Kd']} RMS={b['RMS(m/s)']}", alpha=0.9)
    plt.plot(tb, vdb, "--", label="desired (base)", alpha=0.5)
    plt.plot(ta, va, label=f"ADAPT best Kp={a['Kp']},Ki={a['Ki']},Kd={a['Kd']} RMS={a['RMS(m/s)']}", alpha=0.9)
    plt.plot(ta, vda, "--", label="desired (adapt)", alpha=0.5)
    plt.title("Best curve overlay (baseline vs adaptive)")
    plt.xlabel("Time (s)"); plt.ylabel("Velocity (m/s)")
    plt.grid(True); plt.legend(loc="best")
    out_png = os.path.join("run", "ab_best_overlay.png")
    os.makedirs("run", exist_ok=True)
    plt.tight_layout(); plt.savefig(out_png, dpi=150)
    print(f"已輸出：{out_png}")

def main():
    if not os.path.exists(BASE_SUM) or not os.path.exists(ADPT_SUM):
        raise SystemExit("找不到 base/adapt summary，請先跑 sweep。")

    base = load_summary(BASE_SUM, "base")
    adapt = load_summary(ADPT_SUM, "adapt")

    # 與 PID 係數對齊後比較
    cols = ["Kp", "Ki", "Kd", "RMS(m/s)", "CSV_abs"]
    m = base[cols].merge(adapt[cols], on=["Kp", "Ki", "Kd"], suffixes=("_base", "_adapt"), how="outer")

    # 計算改善量（正值代表 adaptive 較小，也就是變好）
    m["improve"] = m["RMS(m/s)_base"].astype(float) - m["RMS(m/s)_adapt"].astype(float)

    # 排序與輸出
    m_sorted = m.sort_values(by="improve", ascending=False)
    print("\n=== A/B RMS 比較（baseline - adaptive；正值=adaptive 改善） ===")
    cols_out = ["Kp", "Ki", "Kd", "RMS(m/s)_base", "RMS(m/s)_adapt", "improve"]
    print(m_sorted[cols_out].to_string(index=False))

    # 另存 CSV
    out_csv = os.path.join("run", "ab_compare.csv")
    os.makedirs("run", exist_ok=True)
    m_sorted.to_csv(out_csv, index=False)
    print(f"\n已輸出：{out_csv}")

    # 畫「兩邊最佳一組」曲線
    plot_best_curve(base, adapt)

if __name__ == "__main__":
    main()
