#!/usr/bin/env python3
import pandas as pd
import matplotlib.pyplot as plt

# ===== 使用說明 =====
# 這個腳本用來同時比較三種 profile 測試 (Const / Sin / Step)
# 1. 先跑你的 Rust 程式，分別輸出三個 CSV：
#    cargo run -p platform_rpi -- --bench -v 0.6 --seconds 8 \
#       --v-profile Const --csv out_const.csv
#    cargo run -p platform_rpi -- --bench -v 0.6 --seconds 8 \
#       --v-profile Sin   --csv out_sin.csv
#    cargo run -p platform_rpi -- --bench -v 0.6 --seconds 8 \
#       --v-profile Step  --csv out_step.csv --step-at 1.5
#
# 2. 確認這三個檔案 (out_const.csv / out_sin.csv / out_step.csv) 都在當前目錄
#
# 3. 執行：
#    python analyze_compare.py
#
# 4. 輸出 compare.png，包含：
#    - 上圖：三種 profile 的實際輸出 vs 期望速度
#    - 下圖：三種 profile 的距離曲線
#
# 備註：
# - v_out = (left+right)/2 當作平均車速
# - distance 欄位來自 FailSafe 感測器（數值越小越接近障礙物）
# - 可以依需求修改 cases 裡的檔名

cases = {
    "const": "out_const.csv",
    "sin": "out_sin.csv",
    "step": "out_step.csv",
}

fig, (ax1, ax2) = plt.subplots(2, 1, figsize=(12, 7), sharex=True)

for label, path in cases.items():
    df = pd.read_csv(path)
    t = df["t"]
    v_des = df["desired_v"]
    v_out = (df["left"] + df["right"]) / 2.0  # 平均輪速作為近似車速
    dist = df["distance"]

    ax1.plot(t, v_out, label=f"{label} output")
    ax1.plot(t, v_des, "--", alpha=0.7, label=f"{label} desired")

    ax2.plot(t, dist, label=f"{label} distance")

ax1.set_ylabel("Velocity (m/s)")
ax1.set_title("Controller response (const/sin/step)")
ax1.legend()
ax1.grid(True)

ax2.set_xlabel("Time (s)")
ax2.set_ylabel("Distance (m)")
ax2.legend()
ax2.grid(True)

plt.tight_layout()
plt.savefig("compare.png", dpi=150)
plt.show()