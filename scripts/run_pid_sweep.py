#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
run_pid_sweep.py  (grid scan 版)
- 以「範圍 + 步距」自動產生 PID 參數組合，逐一執行 binary 產生 CSV
- 每組只計 FailSafe 前的 RMS 誤差（Bench 模式應該不會觸發）
- 輸出總表與疊圖（若有量測速度則畫 2 子圖）
"""

import os
import subprocess
from datetime import datetime
import itertools as it
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

# ========= 掃描設定（改這裡就好） =========
RUN_CMD_PREFIX = ["cargo", "run", "-p", "platform_rpi", "--"]

# ★ 目標與模式（建議 Bench 模式 + 常值速度）
EXTRA_FLAGS = ["--bench", "--bench-tau", "0.8", "--bench-gain", "0.6", "-v", "0.6"]

HZ = 100
SECONDS = 8

# ★ PID 掃描範圍（含端點），步距可自行調整
KP_RANGE = (0.2, 1.2, 0.2)   # start, stop, step
KI_RANGE = (0.00, 0.20, 0.05)
KD_RANGE = (0.00, 0.12, 0.04)

# 最多允許的組數（避免不小心跑太多）
MAX_COMBOS = 10

# 輸出
OUT_DIR = "runs"
PLOT_FILE = "sweep_compare.png"
os.makedirs(OUT_DIR, exist_ok=True)


# ========= 小工具 =========
def frange(start, stop, step):
    """含 stop 的浮點區間"""
    n = int(round((stop - start) / step)) + 1
    vals = [round(start + i * step, 10) for i in range(n)]
    return vals


def make_pid_grid():
    kp_vals = frange(*KP_RANGE)
    ki_vals = frange(*KI_RANGE)
    kd_vals = frange(*KD_RANGE)
    grid = [(kp, ki, kd) for kp, ki, kd in it.product(kp_vals, ki_vals, kd_vals)]

    # 可選：過濾掉太容易發散的組合（簡單啟發式）
    # 例：同時 Ki/Kd 都偏大時略過
    filtered = []
    for kp, ki, kd in grid:
        if ki > 0.15 and kd > 0.08:
            continue
        filtered.append((kp, ki, kd))

    if len(filtered) > MAX_COMBOS:
        print(f"⚠️ 組數 {len(filtered)} 超過 MAX_COMBOS={MAX_COMBOS}，將只取前 {MAX_COMBOS} 組。")
        filtered = filtered[:MAX_COMBOS]
    return filtered


def run_one(pid, hz=HZ, seconds=SECONDS):
    kp, ki, kd = pid
    tag = f"kp{kp:.2f}_ki{ki:.2f}_kd{kd:.2f}_hz{hz}_sec{seconds}"
    csv_path = os.path.join(OUT_DIR, f"pid_{tag}.csv")
    if os.path.exists(csv_path):
        print(f"🟡 已存在，跳過：{csv_path}")
        return csv_path

    cmd = RUN_CMD_PREFIX + [
        "--kp", str(kp), "--ki", str(ki), "--kd", str(kd),
        "--hz", str(hz), "--seconds", str(seconds),
        "--csv", csv_path
    ] + EXTRA_FLAGS

    print(f"▶️  執行：{' '.join(cmd)}")
    try:
        subprocess.run(cmd, check=True)
    except subprocess.CalledProcessError as e:
        print(f"❌ 執行失敗：{e}")
        return None

    return csv_path if os.path.exists(csv_path) else None


def read_csv(csv_path):
    df = pd.read_csv(csv_path)
    t = df["t"].to_numpy(float)
    v_des = df["desired_v"].to_numpy(float)
    state = df["state"].astype(str).to_numpy()

    # 量測速度欄位（Bench 模式會有 meas_left/meas_right）
    lc = [c.lower() for c in df.columns]
    def pick(a, b):
        if a in lc and b in lc:
            return df.columns[lc.index(a)], df.columns[lc.index(b)]
        return None

    pair = pick("meas_left", "meas_right") or pick("vel_left", "vel_right")
    if pair:
        v_out = (df[pair[0]].to_numpy(float) + df[pair[1]].to_numpy(float)) / 2.0
        has_meas = True
    else:
        v_out = None
        has_meas = False

    # 控制輸出（可選）：left/right or pwm_left/pwm_right
    pair_u = pick("left", "right") or pick("u_left", "u_right") or pick("pwm_left", "pwm_right")
    u = (df[pair_u[0]].to_numpy(float) + df[pair_u[1]].to_numpy(float))/2.0 if pair_u else None

    return dict(t=t, v_des=v_des, v_out=v_out, u=u, state=state, has_meas=has_meas)


def rms_before_failsafe(t, v_des, v_out, state):
    if v_out is None:
        return np.nan, np.inf
    nonrun = np.where(state != "Run")[0]
    brake_time = t[nonrun[0]] if len(nonrun) else np.inf
    m = (state == "Run") & (t < brake_time)
    if not m.any():
        return np.nan, brake_time
    return float(np.sqrt(np.mean((v_out[m] - v_des[m])**2))), brake_time


def plot_curves(curves, out_png):
    has_meas_any = any(c["has_meas"] for c in curves)

    if has_meas_any:
        fig, axes = plt.subplots(2, 1, figsize=(14, 8), sharex=True, gridspec_kw={"height_ratios":[2,1]})
        ax_v, ax_u = axes
        ax_v.plot(curves[0]["t"], curves[0]["v_des"], "--", label="desired")
        for c in curves:
            if c["has_meas"] and c["v_out"] is not None:
                kp, ki, kd = c["pid"]
                lab = f"Kp={kp:.2f}, Ki={ki:.2f}, Kd={kd:.2f}"
                if not np.isnan(c["rms"]):
                    lab += f" (RMS={c['rms']:.3f})"
                ax_v.plot(c["t"], c["v_out"], label=lab)
        ax_v.set_title("Measured velocity vs desired")
        ax_v.set_ylabel("Velocity (m/s)")
        ax_v.grid(True); ax_v.legend(loc="best")
    else:
        fig, ax_u = plt.subplots(1, 1, figsize=(14, 5))
        ax_v = None

    for c in curves:
        if c["u"] is not None:
            kp, ki, kd = c["pid"]
            ax_u.plot(c["t"], c["u"], label=f"Kp={kp:.2f}, Ki={ki:.2f}, Kd={kd:.2f}")
    ax_u.set_title("Control output (avg of left/right)")
    ax_u.set_xlabel("Time (s)")
    ax_u.set_ylabel("u (norm)")
    ax_u.grid(True); ax_u.legend(loc="best")

    plt.tight_layout(); plt.savefig(out_png, dpi=150)


# ========= 主程式 =========
def main():
    grid = make_pid_grid()
    print("==== PID Grid 掃描 ====")
    print(f"時間：{datetime.now():%Y-%m-%d %H:%M:%S}")
    print(f"共 {len(grid)} 組，Hz={HZ}, Sec={SECONDS}, 其他旗標={EXTRA_FLAGS}\n")

    summaries, curves = [], []

    for kp, ki, kd in grid:
        csv = run_one((kp, ki, kd))
        if not csv:
            continue
        data = read_csv(csv)
        rms, brake = rms_before_failsafe(data["t"], data["v_des"], data["v_out"], data["state"])

        summaries.append({
            "Kp": kp, "Ki": ki, "Kd": kd,
            "RMS(m/s)": None if np.isnan(rms) else round(rms, 4),
            "FailSafe(s)": "—" if np.isinf(brake) else round(float(brake), 3),
            "CSV": os.path.relpath(csv),
        })
        curves.append({
            "pid": (kp, ki, kd),
            "t": data["t"], "v_des": data["v_des"], "v_out": data["v_out"],
            "u": data["u"], "has_meas": data["has_meas"],
            "rms": rms, "brake_time": brake,
        })

    if summaries:
        df = pd.DataFrame(summaries).sort_values(by=["RMS(m/s)"], na_position="last")
        print("\n=== PID Grid 結果（RMS 只算到 FailSafe 前）===")
        print(df.to_string(index=False))
    else:
        print("沒有可用結果，請檢查執行/寫檔是否成功。")
        return

    if curves:
        plot_curves(curves, PLOT_FILE)
        print(f"\n已輸出疊圖：{PLOT_FILE}")


if __name__ == "__main__":
    main()
