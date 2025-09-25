#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
run_pid_sweep.py  (grid scan 版 · 支援 err / adapt_gain)
------------------------------------------------------------
功能：
1) 以「範圍 + 步距」自動產生 PID (Kp, Ki, Kd) 組合，逐一執行你的 binary。
2) 每組只計 FailSafe 前的 RMS 誤差。
3) 額外統計：
   - 平均 |誤差| (Mean|Err|)，單位 m/s
   - 平均自適應增益 (MeanAdaptGain)，沒開 adaptive 時 = 1.0
4) 產出總表（CSV/終端列印）與疊圖：
   - 上圖：量測速度 vs desired
   - 中圖：控制輸出
   - 下圖：adaptive gain 變化（如果有）
   → 圖例會顯示 RMS，最佳組會高亮。

用法（常見範例見檔尾註解）：
    python scripts/run_pid_sweep.py \
      --extra "--bench --bench-tau 0.8 --bench-gain 0.6 -v 0.6"

    # A/B 對比：開啟自適應
    python scripts/run_pid_sweep.py \
      --adaptive \
      --extra "--bench --bench-tau 0.8 --bench-gain 0.6 -v 0.6"

輸出：
- 圖檔：<out-dir>/<plot>
- 總表：<out-dir>/<summary>
- 每組 CSV：<out-dir>/pid_kpXX_kiYY_kdZZ_hz##_sec#.csv
------------------------------------------------------------
"""

import os
import shlex
import argparse
import subprocess
import itertools as it
from datetime import datetime

import numpy as np
import pandas as pd
import matplotlib.pyplot as plt


# ========= 執行前綴（你的 binary） =========
RUN_CMD_PREFIX = ["cargo", "run", "-p", "platform_rpi", "--"]

# ========= 預設掃描範圍 =========
KP_RANGE = (0.2, 1.2, 0.2)   # start, stop, step
KI_RANGE = (0.00, 0.20, 0.05)
KD_RANGE = (0.00, 0.12, 0.04)


# ========= CLI =========
def parse_args():
    p = argparse.ArgumentParser(description="PID grid sweep runner")
    p.add_argument("--hz", type=int, default=100, help="取樣頻率 (Hz)")
    p.add_argument("--seconds", type=float, default=8, help="執行秒數")
    p.add_argument("--adaptive", action="store_true",
                   help="啟用自適應（把 --adaptive 轉發給 binary）")
    p.add_argument("--extra", type=str, default="",
                   help='其它旗標，例如："--bench --bench-tau 0.8 --bench-gain 0.6 -v 0.6"')
    p.add_argument("--max-combos", type=int, default=30,
                   help="最多測試的 PID 組數（避免不小心跑太多）")
    p.add_argument("--out-dir", type=str, default="run",
                   help="CSV 與輸出圖表的資料夾")
    p.add_argument("--plot", type=str, default="sweep_compare.png",
                   help="輸出的疊圖檔名")
    p.add_argument("--summary", type=str, default="sweep_summary.csv",
                   help="輸出的總表 CSV 檔名")
    # 掃描範圍覆蓋
    p.add_argument("--kp", type=str, default=None,
                   help="覆蓋 Kp 範圍，格式 start,stop,step")
    p.add_argument("--ki", type=str, default=None,
                   help="覆蓋 Ki 範圍，格式 start,stop,step")
    p.add_argument("--kd", type=str, default=None,
                   help="覆蓋 Kd 範圍，格式 start,stop,step")
    return p.parse_args()


def build_extra_flags(args):
    flags = []
    if args.adaptive:
        flags.append("--adaptive")
    if args.extra:
        flags += shlex.split(args.extra)
    return flags


# ========= 小工具 =========
def frange(start, stop, step):
    """含 stop 的浮點區間"""
    n = int(round((stop - start) / step)) + 1
    return [round(start + i * step, 10) for i in range(n)]


def parse_range(s, default_triplet):
    if not s:
        return default_triplet
    parts = [float(x.strip()) for x in s.split(",")]
    if len(parts) != 3:
        raise ValueError(f"range 需為 'start,stop,step'，但拿到：{s}")
    return tuple(parts)


def make_pid_grid(kp_rng, ki_rng, kd_rng, max_combos):
    kp_vals = frange(*kp_rng)
    ki_vals = frange(*ki_rng)
    kd_vals = frange(*kd_rng)
    grid = [(kp, ki, kd) for kp, ki, kd in it.product(kp_vals, ki_vals, kd_vals)]

    # 簡單過濾：Ki 與 Kd 同時偏大容易發散
    filtered = []
    for kp, ki, kd in grid:
        if ki > 0.15 and kd > 0.08:
            continue
        filtered.append((kp, ki, kd))

    if len(filtered) > max_combos:
        print(f"⚠️ 組數 {len(filtered)} 超過 MAX_COMBOS={max_combos}，只取前 {max_combos} 組。")
        filtered = filtered[:max_combos]
    return filtered


def ensure_parent_dir(path):
    os.makedirs(os.path.dirname(os.path.abspath(path)), exist_ok=True)


def run_one(pid, hz, seconds, out_dir, extra_flags):
    """以一組 (kp,ki,kd) 執行測試，回傳 CSV 路徑"""
    kp, ki, kd = pid
    tag = f"kp{kp:.2f}_ki{ki:.2f}_kd{kd:.2f}_hz{hz}_sec{seconds}"
    csv_path = os.path.join(out_dir, f"pid_{tag}.csv")

    if os.path.exists(csv_path):
        print(f"🟡 已存在，跳過：{csv_path}")
        return csv_path

    ensure_parent_dir(csv_path)

    cmd = RUN_CMD_PREFIX + [
        "--kp", str(kp), "--ki", str(ki), "--kd", str(kd),
        "--hz", str(hz), "--seconds", str(seconds),
        "--csv", csv_path
    ] + extra_flags

    print(f"▶️  執行：{' '.join(cmd)}")
    try:
        subprocess.run(cmd, check=True)
    except subprocess.CalledProcessError as e:
        print(f"❌ 執行失敗：{e}")
        return None

    return csv_path if os.path.exists(csv_path) else None


def read_csv(csv_path):
    """讀取一個 CSV，回傳包含 err / adapt_gain"""
    df = pd.read_csv(csv_path)

    t = df["t"].to_numpy(float)
    v_des = df["desired_v"].to_numpy(float)
    state = df["state"].astype(str).to_numpy()

    lc = [c.lower() for c in df.columns]

    def pair(a, b):
        if a in lc and b in lc:
            return df.columns[lc.index(a)], df.columns[lc.index(b)]
        return None

    # 量測速度
    vpair = pair("meas_left", "meas_right") or pair("vel_left", "vel_right")
    if vpair:
        v_out = (df[vpair[0]].to_numpy(float) + df[vpair[1]].to_numpy(float)) / 2.0
        has_meas = True
    else:
        v_out = None
        has_meas = False

    # 控制輸出
    upair = pair("left", "right") or pair("u_left", "u_right")
    u = (df[upair[0]].to_numpy(float) + df[upair[1]].to_numpy(float)) / 2.0 if upair else None

    # 誤差與自適應增益
    err = df["err"].to_numpy(float) if "err" in df.columns else None
    adapt_gain = df["adapt_gain"].to_numpy(float) if "adapt_gain" in df.columns else None

    return dict(t=t, v_des=v_des, v_out=v_out, u=u, state=state,
                has_meas=has_meas, err=err, adapt_gain=adapt_gain)


def rms_before_failsafe(t, v_des, v_out, state):
    """回傳 (RMS, brake_time)"""
    if v_out is None:
        return np.nan, np.inf
    nonrun = np.where(state != "Run")[0]
    brake_time = t[nonrun[0]] if len(nonrun) else np.inf
    mask = (state == "Run") & (t < brake_time)
    if not mask.any():
        return np.nan, brake_time
    rms = float(np.sqrt(np.mean((v_out[mask] - v_des[mask]) ** 2)))
    return rms, brake_time


def plot_curves(curves, out_png):
    """上=速度，中=控制輸出，下=adaptive gain"""
    has_meas_any = any(c["has_meas"] for c in curves)
    has_adapt = any(c.get("adapt_gain") is not None for c in curves)

    nrows = 3 if has_adapt else 2
    fig, axes = plt.subplots(nrows, 1, figsize=(14, 10), sharex=True,
                             gridspec_kw={"height_ratios": [2, 1, 1][:nrows]})
    if nrows == 3:
        ax_v, ax_u, ax_g = axes
    else:
        ax_v, ax_u = axes
        ax_g = None

    # 速度曲線
    if has_meas_any:
        ax_v.plot(curves[0]["t"], curves[0]["v_des"], "--", label="desired")
        valid = [c for c in curves if not np.isnan(c["rms"])]
        best = min(valid, key=lambda c: c["rms"]) if valid else None
        for c in curves:
            if c["has_meas"] and c["v_out"] is not None:
                kp, ki, kd = c["pid"]
                label = f"Kp={kp:.2f}, Ki={ki:.2f}, Kd={kd:.2f}"
                if not np.isnan(c["rms"]):
                    label += f" (RMS={c['rms']:.3f})"
                lw = 2.6 if (best is not None and c is best) else 1.0
                ax_v.plot(c["t"], c["v_out"], label=label, linewidth=lw)
        ax_v.set_title("Measured velocity vs desired")
        ax_v.set_ylabel("Velocity (m/s)")
        ax_v.grid(True)
        ax_v.legend()

    # 控制輸出
    for c in curves:
        if c["u"] is not None:
            kp, ki, kd = c["pid"]
            ax_u.plot(c["t"], c["u"], label=f"Kp={kp:.2f}, Ki={ki:.2f}, Kd={kd:.2f}")
    ax_u.set_title("Control output (avg of left/right)")
    ax_u.set_xlabel("Time (s)")
    ax_u.set_ylabel("u (norm)")
    ax_u.grid(True)
    ax_u.legend()

    # Adaptive gain
    if ax_g is not None:
        for c in curves:
            if c.get("adapt_gain") is not None:
                kp, ki, kd = c["pid"]
                ax_g.plot(c["t"], c["adapt_gain"], label=f"Kp={kp:.2f}, Ki={ki:.2f}, Kd={kd:.2f}")
        ax_g.set_title("Adaptive gain evolution")
        ax_g.set_ylabel("Gain")
        ax_g.set_xlabel("Time (s)")
        ax_g.grid(True)
        ax_g.legend()

    plt.tight_layout()
    plt.savefig(out_png, dpi=150)


# ========= 主程式 =========
def main():
    args = parse_args()
    extra_flags = build_extra_flags(args)

    kp_rng = parse_range(args.kp, KP_RANGE)
    ki_rng = parse_range(args.ki, KI_RANGE)
    kd_rng = parse_range(args.kd, KD_RANGE)

    os.makedirs(args.out_dir, exist_ok=True)
    plot_path = os.path.join(args.out_dir, args.plot)
    summary_path = os.path.join(args.out_dir, args.summary)

    grid = make_pid_grid(kp_rng, ki_rng, kd_rng, args.max_combos)

    print("==== PID Grid 掃描 ====")
    print(f"時間：{datetime.now():%Y-%m-%d %H:%M:%S}")
    print(f"共 {len(grid)} 組，Hz={args.hz}, Sec={args.seconds}")
    print(f"adaptive={args.adaptive}, extra_flags={extra_flags}")
    print(f"輸出目錄={args.out_dir}\n")

    summaries, curves = [], []

    for kp, ki, kd in grid:
        csv = run_one((kp, ki, kd), args.hz, args.seconds, args.out_dir, extra_flags)
        if not csv:
            continue
        data = read_csv(csv)
        rms, brake = rms_before_failsafe(data["t"], data["v_des"], data["v_out"], data["state"])

        summaries.append({
            "Kp": kp, "Ki": ki, "Kd": kd,
            "RMS(m/s)": None if np.isnan(rms) else round(rms, 4),
            "Mean|Err|(m/s)": None if data["err"] is None else round(float(np.mean(np.abs(data["err"]))), 4),
            "MeanAdaptGain": None if data["adapt_gain"] is None else round(float(np.mean(data["adapt_gain"])), 3),
            "FailSafe(s)": "—" if np.isinf(brake) else round(float(brake), 3),
            "CSV": os.path.relpath(csv),
        })
        curves.append({
            "pid": (kp, ki, kd),
            "t": data["t"], "v_des": data["v_des"], "v_out": data["v_out"],
            "u": data["u"], "has_meas": data["has_meas"],
            "rms": rms, "brake_time": brake,
            "adapt_gain": data["adapt_gain"],
        })

    if not summaries:
        print("沒有可用結果，請檢查執行/寫檔是否成功。")
        return

    df = pd.DataFrame(summaries).sort_values(by=["RMS(m/s)"], na_position="last")
    print("\n=== PID Grid 結果（RMS 只算到 FailSafe 前）===")
    print(df.to_string(index=False))
    df.to_csv(summary_path, index=False)
    print(f"\n已輸出總表：{summary_path}")

    if curves:
        plot_curves(curves, plot_path)
        print(f"已輸出疊圖：{plot_path}")


if __name__ == "__main__":
    main()

# ========= 使用說明 =========
#
# 1) 最小實驗（Bench + 常值 0.6 m/s）：
#    python scripts/run_pid_sweep.py \
#      --extra "--bench --bench-tau 0.8 --bench-gain 0.6 -v 0.6"
#
# 2) A/B 對照（純 PID vs. 自適應 PID）：
#    # 純 PID
#    python scripts/run_pid_sweep.py \
#      --extra "--bench --bench-tau 0.8 --bench-gain 0.6 -v 0.6"
#
#    # 開 adaptive
#    python scripts/run_pid_sweep.py \
#      --adaptive \
#      --extra "--bench --bench-tau 0.8 --bench-gain 0.6 -v 0.6"
#
# 3) 調整掃描範圍：
#    python scripts/run_pid_sweep.py \
#      --kp "0.1,0.6,0.1" --ki "0.00,0.10,0.05" --kd "0.00,0.08,0.04" \
#      --adaptive \
#      --extra "--bench --bench-tau 0.8 --bench-gain 0.6 -v 0.6"
#
# 4) A/B best overlay：
#    python scripts/run_pid_sweep.py \
#      --out-dir run/base \
#      --plot base_overlay.png --summary base_summary.csv \
#      --kp "0.1,0.3,0.05" --ki "0.00,0.05,0.025" --kd "0.00,0.04,0.02" \
#      --extra "--bench --bench-tau 0.3 --bench-gain 1.0 -v 0.3"
#
#    python scripts/run_pid_sweep.py \
#      --out-dir run/adapt \
#      --plot adapt_overlay.png --summary adapt_summary.csv \
#      --kp "0.1,0.3,0.05" --ki "0.00,0.05,0.025" --kd "0.00,0.04,0.02" \
#      --adaptive \
#      --extra "--bench --bench-tau 0.3 --bench-gain 1.0 -v 0.3 --e-small 0.01 --e-large 0.1 --gain-min 0.2 --gain-max 3.0"
#
#       python scripts/ab_compare.py
#

# 結果：
# - summary.csv：包含 RMS / Mean|Err| / MeanAdaptGain。
# - sweep_compare.png：三子圖（速度 / 控制輸出 / adaptive gain）。
