#!/usr/bin/env python3
import pandas as pd
import matplotlib.pyplot as plt

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