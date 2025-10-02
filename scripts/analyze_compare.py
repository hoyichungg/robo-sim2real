#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
analyze_compare.py
- 讀取 out_const.csv / out_sin.csv / out_step.csv
- 計算「FailSafe 觸發前」的 RMS 誤差（Const / Sin / Step）
- 對 Step 另外計算 KPI（上升時間、超調%、穩定時間、穩態誤差），只統計 FailSafe 前
- 以時間自適應範圍畫出三種測試（輸出 vs. 目標）的疊圖
"""

import os
import argparse
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt
from typing import Optional

# =====(可選) 中文字型缺字處理：盡力指定常見中文字型，沒有也不報錯 =====
try:
    from matplotlib import rcParams
    rcParams["axes.unicode_minus"] = False
    # 依平台常見字型嘗試
    candidate_fonts = [
        "Noto Sans CJK TC",  # win/linux 常見安裝
        "PingFang TC",       # macOS
        "Microsoft JhengHei",
        "Heiti TC",
        "Noto Sans CJK JP",
    ]
    for fam in candidate_fonts:
        # 直接把候選字型放到 sans-serif 首位
        rcParams["font.sans-serif"] = [fam] + rcParams.get("font.sans-serif", [])
except Exception:
    pass

# --------- 設定（檔名可依需求更改）---------
DEFAULT_FILES = {
    "Const": ["out_const.csv", "run/out_const.csv"],
    "Sin":   ["out_sin.csv", "run/out_sin.csv"],
    "Step":  ["out_step.csv", "run/out_step.csv"],
}

def resolve_path(arg: Optional[str], candidates: list[str]) -> Optional[str]:
    if arg:
        return arg if os.path.exists(arg) else None
    for p in candidates:
        if os.path.exists(p):
            return p
    return None

def parse_args():
    ap = argparse.ArgumentParser(description="Compare Const/Sin/Step runs before FailSafe")
    ap.add_argument("--const", type=str, default=None, help="Const CSV 路徑（預設找 out_const.csv 或 run/out_const.csv）")
    ap.add_argument("--sin", type=str, default=None, help="Sin CSV 路徑（預設找 out_sin.csv 或 run/out_sin.csv）")
    ap.add_argument("--step", type=str, default=None, help="Step CSV 路徑（預設找 out_step.csv 或 run/out_step.csv）")
    ap.add_argument("--out", type=str, default="compare.png", help="輸出的疊圖檔名")
    return ap.parse_args()

# --------- 小工具：線性內插找穿越時刻 ----------
def first_cross_time(t, y, level, direction="up"):
    """
    回傳第一次穿越 level 的時間（線性內插）
    direction: "up" 代表從低→高，"down" 代表從高→低
    若找不到則回傳 np.nan
    """
    for i in range(len(y) - 1):
        y0, y1 = y[i], y[i + 1]
        if direction == "up":
            if (y0 < level) and (y1 >= level):
                if y1 == y0:
                    return t[i + 1]
                return t[i] + (level - y0) / (y1 - y0) * (t[i + 1] - t[i])
        else:
            if (y0 > level) and (y1 <= level):
                if y1 == y0:
                    return t[i + 1]
                return t[i] + (level - y0) / (y1 - y0) * (t[i + 1] - t[i])
    return np.nan

# --------- 計算：FailSafe 前的 RMS 誤差 ----------
def rms_error_before_brake(df):
    """
    只取 state == "Run" 的區段；若有觸發（第一個 != Run），則僅計算到觸發「之前」。
    回傳 (rms, t_valid, v_des_valid, v_out_valid, brake_time or np.inf)
    """
    # 讀欄位
    t = df["t"].to_numpy(float)
    v_des = df["desired_v"].to_numpy(float)
    # 這邊用左右輪平均代表實際輸出速度
    v_out = ((df["left"] + df["right"]) / 2.0).to_numpy(float)
    # 若沒有 state 欄位，視為全程 Run
    state = df["state"].astype(str).to_numpy() if "state" in df.columns else np.array(["Run"] * len(df))

    # 第一個非 Run 的時間當作安全觸發時間
    first_nonrun = np.where(state != "Run")[0]
    brake_time = t[int(first_nonrun[0])] if len(first_nonrun) else np.inf

    # 僅保留觸發前與 state==Run 的樣本
    mask = (state == "Run") & (t < brake_time)
    t_valid = t[mask]
    v_des_valid = v_des[mask]
    v_out_valid = v_out[mask]

    if len(t_valid) == 0:
        return np.nan, t_valid, v_des_valid, v_out_valid, brake_time

    err = v_out_valid - v_des_valid
    rms = float(np.sqrt(np.mean(err**2)))
    return rms, t_valid, v_des_valid, v_out_valid, brake_time

# --------- Step KPI（僅至 FailSafe 前） ----------
def step_kpis(df):
    """
    自動偵測步階開始時間（desired_v 首次明顯變化），然後只在 FailSafe 觸發前評估：
    - 上升時間(10→90%), 超調%, 穩定時間(±2%), 穩態誤差
    回傳 dict；若資料不足或未達 90% 則回傳 NaN 並附原因。
    """
    t = df["t"].to_numpy(float)
    v_des = df["desired_v"].to_numpy(float)
    v_out = ((df["left"] + df["right"]) / 2.0).to_numpy(float)
    state = df["state"].astype(str).to_numpy() if "state" in df.columns else np.array(["Run"] * len(df))

    # 目標值取 desired_v 的最大值（適用一般「由 0 踩到某值」的 step）
    v_target = float(np.max(v_des))
    if v_target <= 1e-9:
        return dict(target=0.0, t0=np.nan, rise=np.nan, overshoot=np.nan,
                    settling=np.nan, ess=np.nan, note="目標速度為 0，非有效 step 測試。")

    # 找步階開始：desired_v 的相鄰差值 > 5% 目標
    dv = np.diff(v_des)
    idx = np.where(np.abs(dv) >= 0.05 * v_target)[0]
    if len(idx):
        step_t0 = t[int(idx[0]) + 1]
    else:
        # 後備方案：第一次達到 10% 目標
        step_t0 = first_cross_time(t, v_des, 0.1 * v_target, "up")

    # FailSafe 觸發時間
    non_run = np.where(state != "Run")[0]
    brake_time = t[int(non_run[0])] if len(non_run) else np.inf

    # 只取步階開始之後、FailSafe 之前
    mask = (t >= step_t0) & (t < brake_time)
    t_valid = t[mask]
    v_valid = v_out[mask]
    if len(t_valid) < 3:
        note = f"有效資料不足；FailSafe 在 {brake_time:.3f}s 觸發。" if np.isfinite(brake_time) else "有效資料不足。"
        return dict(target=v_target, t0=step_t0, rise=np.nan, overshoot=np.nan,
                    settling=np.nan, ess=np.nan, note=note)

    # 以步階開始為原點
    t_rel = t_valid - step_t0

    # 上升時間（線性內插 10% / 90%）
    low, high = 0.1 * v_target, 0.9 * v_target
    t10 = first_cross_time(t_rel, v_valid, low, "up")
    t90 = first_cross_time(t_rel, v_valid, high, "up")
    rise = (t90 - t10) if (not np.isnan(t10) and not np.isnan(t90) and t90 >= t10) else np.nan

    # 超調%
    vmax = float(np.max(v_valid))
    overshoot = max(0.0, (vmax - v_target) / v_target * 100.0)

    # 穩定時間（進入 ±2% band 且後續都在 band 內）
    band_upper, band_lower = 1.02 * v_target, 0.98 * v_target
    inside = (v_valid >= band_lower) & (v_valid <= band_upper)
    settling = np.nan
    if inside.any():
        last_false = np.where(~inside)[0]
        if len(last_false) == 0:
            settling = 0.0
        else:
            j = int(last_false[-1]) + 1
            if j < len(t_rel) and inside[j:].all():
                settling = float(t_rel[j])

    # 穩態誤差（尾端 10% 平均）
    tail_n = max(5, int(len(v_valid) * 0.1))
    ess = float(np.mean(v_valid[-tail_n:])) - v_target

    note = ""
    if np.isfinite(brake_time):
        note = f"FailSafe 於 {brake_time:.3f}s 觸發；KPI 僅計到觸發前。"
        if np.isnan(t90):
            note += "（觸發前未達 90% 目標）"

    return dict(target=v_target, t0=step_t0, rise=rise, overshoot=overshoot,
                settling=settling, ess=ess, note=note)

# --------- 主流程 ----------
def main():
    args = parse_args()
    records = []   # 收集 RMS 表
    available = {} # 繪圖用資料（各自的時間與曲線）

    files = {
        "Const": resolve_path(args.const, DEFAULT_FILES["Const"]),
        "Sin": resolve_path(args.sin, DEFAULT_FILES["Sin"]),
        "Step": resolve_path(args.step, DEFAULT_FILES["Step"]),
    }

    # 讀三種檔案，計算 RMS（FailSafe 前）
    for name, path in files.items():
        if not path:
            # 指出找過哪些候選
            print(f"⚠️ 找不到 {name}，請用 --{name.lower()} 指定或放在 {DEFAULT_FILES[name]} 中任何一個")
            continue

        df = pd.read_csv(path)
        rms, t_valid, v_des_valid, v_out_valid, brake_time = rms_error_before_brake(df)
        available[name] = dict(t=t_valid, v_des=v_des_valid, v_out=v_out_valid)

        records.append({
            "測試": name,
            "RMS 誤差 (m/s)": np.nan if np.isnan(rms) else round(rms, 4),
            "FailSafe 觸發時間 (s)": (np.inf if np.isinf(brake_time) else round(float(brake_time), 3))
        })

    # 列印 RMS 表
    if records:
        table = pd.DataFrame.from_records(records)
        # 將 inf 轉成「—」方便閱讀
        table["FailSafe 觸發時間 (s)"] = table["FailSafe 觸發時間 (s)"].apply(lambda x: "—" if x == np.inf else x)
        print("\n=== Const / Sin / Step RMS 誤差（只計到 FailSafe 前） ===")
        print(table.to_string(index=False))
    else:
        print("沒有任何 CSV 被讀到，請確認檔案是否存在於目前目錄。")

    # Step KPI
    if "Step" in available and files.get("Step"):
        df_step = pd.read_csv(files["Step"])
        k = step_kpis(df_step)
        print("\n=== Step 響應 KPI（FailSafe 前） ===")
        print(f"目標速度: {k['target']:.2f} m/s")
        print(f"步階開始時間: {k['t0']:.3f} s")
        print(f"上升時間 (10%→90%): {np.nan if np.isnan(k['rise']) else round(k['rise'],3)} s")
        print(f"超調量: {np.nan if np.isnan(k['overshoot']) else round(k['overshoot'],2)} %")
        print(f"穩定時間 (±2%): {np.nan if np.isnan(k['settling']) else round(k['settling'],3)} s")
        print(f"穩態誤差: {np.nan if np.isnan(k['ess']) else round(k['ess'],3)} m/s")
        if k["note"]:
            print(k["note"])

    # --------- 作圖：三條曲線疊圖（時間自適應） ----------
    if available:
        plt.figure(figsize=(14, 6))

        # 依存在的資料繪製
        if "Const" in available:
            plt.plot(available["Const"]["t"], available["Const"]["v_out"], label="Const output")
            plt.plot(available["Const"]["t"], available["Const"]["v_des"], "--", label="Const desired")
        if "Sin" in available:
            plt.plot(available["Sin"]["t"], available["Sin"]["v_out"], label="Sin output")
            plt.plot(available["Sin"]["t"], available["Sin"]["v_des"], "--", label="Sin desired")
        if "Step" in available:
            plt.plot(available["Step"]["t"], available["Step"]["v_out"], label="Step output")
            plt.plot(available["Step"]["t"], available["Step"]["v_des"], "--", label="Step desired")

        # 自適應時間軸（依所有可用資料的最小/最大時間加少量 padding）
        ts = [available[k]["t"] for k in available if available[k]["t"].size > 0]
        if ts:
            t_min = min(np.nanmin(a) for a in ts)
            t_max = max(np.nanmax(a) for a in ts)
            span = max(1e-9, t_max - t_min)
            pad = 0.02 * span
            plt.xlim(t_min - pad, t_max + pad)

        plt.title("Const / Sin / Step 疊圖")
        plt.xlabel("Time (s)")
        plt.ylabel("Velocity (m/s)")
        plt.grid(True)
        plt.legend(loc="best")
        plt.tight_layout()
        plt.savefig(args.out, dpi=150)
        print(f"\n已輸出疊圖：{args.out}")

if __name__ == "__main__":
    main()
