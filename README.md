# robo-sim2real

[![CI](https://github.com/hoyichungg/robo-sim2real/actions/workflows/ci.yml/badge.svg)](https://github.com/hoyichungg/robo-sim2real/actions)

Skeleton project for differential-drive robots with a **unified control core** that runs both in  
**2D simulation (Bevy + Rapier2D)** and on **Raspberry Pi/Linux stubs**.  
Focus: minimal control loop, safety stop (fail-safe), and reproducible workflows with CI.

---

## ✨ Features

- **Unified core logic** – Write once, run in simulation or on real hardware.
- **Differential-drive model** – Simple forward-speed PID, extendable with angular velocity.
- **Fail-safe** – Stops robot if distance < threshold or sensor error occurs.
- **Simulation** – Bevy + Rapier2D 2D environment with mock drivers.
- **Platform RPi** – Raspberry Pi driver stubs (via `rppal` in future).
- **Config-driven** – TOML configs for PID, thresholds, wheel base, wheel radius.
- **Telemetry & Replay** – Record data to CSV for deterministic replays.
- **CI-ready** – Lint, unit tests, and aarch64 cross-compilation.

---

## 🏗 Project Structure

/core
/control      # PID, filters, fail-safe state machine
/hal          # traits: Motor, DistanceSensor, Clock, Telemetry
/model        # DifferentialKinematics, Units, Command
/replay       # CSV recorder & player
/drivers
/mock         # mock motor & distance sensor
/rpi          # Raspberry Pi driver stubs
/sim2d          # Bevy + Rapier2D simulation world
/platform_rpi   # RPi entrypoint crate
/configs        # Default TOML configs
/tests          # property-based & integration tests

---

## 🚀 Quick Start

### Run 2D Simulation (desktop)

```bash
cargo run -p sim2d

➡ You should see a differential-drive robot moving forward.
If it gets too close to an obstacle, the fail-safe triggers and stops the robot.

# Advanced: with safety margin and multiple obstacles
cargo run -p sim2d -- \
  --hz 100 -v 0.6 \
  --v-profile sin --sin-amp 0.3 --sin-freq 0.2 --sin-bias 0.4 \
  --safety-margin-ratio 0.1 \
  --obstacle 300,0 --obstacle 350,120 --obstacle 420,-100

### Use a Config File

```bash
cp configs/sim2d.example.toml run/my_sim.toml
cargo run -p sim2d -- --config run/my_sim.toml

# CLI flags still override values from the file when provided
cargo run -p sim2d -- --config run/my_sim.toml --hz 200 --plant-gain 1.0
```

Config files are TOML. Relative paths (for example the CSV destination) resolve against the config file location, so you can keep per-run assets alongside the config snippet.

⸻

Run with Raspberry Pi (stub drivers)

cargo build -p platform_rpi --target aarch64-unknown-linux-gnu

➡ Produces a cross-compiled binary for Raspberry Pi (stub returns fake sensor values).

⸻

Run Tests

cargo test

	•	PID unit tests
	•	Fail-safe logic tests
	•	Property-based tests for stability

⸻

📊 Telemetry & Replay
	•	Runtime data (distance, wheel speeds, commands, states) is recorded to CSV.
	•	Replays can be run deterministically with the same inputs for debugging.

⸻

## 🧩 Driver Backends

`drivers::factory::DriverFactory` centralises hardware selection. Calling `create_all()` now returns a named `DriverHandles` struct with `motor` and `distance` fields, so additional resources (IMUs, encoders, etc.) can be added without reshaping downstream call sites.

- `Mock` – default for desktop simulation; uses in-memory stubs.
- `Bench` – reuses the mock motor so callers can supply their own plant model.
- `Rpi` – builds the Raspberry Pi hardware implementations when the `rpi` feature is enabled; otherwise gracefully falls back to the stub wrappers.

This section makes it easier to audit which backend powers a given run configuration, whether it comes from the CLI, a config file, or a future orchestrator.

⸻

## 🔧 PID Tuning (sim2d)

- Defaults (sim2d CLI): `--kp 0.6 --ki 0.05 --kd 0.0`（保守穩定，建議由此起步）
- Working example at 100 Hz（較快 plant）
  - `--kp 0.8 --ki 0.2 --kd 0.0 --tau 0.2 --plant-gain 1.0`
- Suggested ranges by control loop rate（依模型與目標調整）
  - 50 Hz：`kp 0.6–0.8`、`ki 0.03–0.10`、`kd 0.0–0.005`
  - 100 Hz：`kp 0.6–1.0`、`ki 0.05–0.20`、`kd 0.0–0.010`
  - 200 Hz：`kp 0.4–0.8`、`ki 0.02–0.10`、`kd 0.0–0.020`
- Heuristics
  - 初期先把 `kd=0`；若需要加速抑制超調，再很小幅度加 D。
  - 若輸出在 ±1 之間來回跳（抖動/打頂）：減小 `kp` 或將 `kd` 調回 0。
  - 上升太慢：增加 `kp` 或 `ki`；超調太大：降低 `kp` 或微量加入 `kd`。
  - 植物一階模型：`tau` 越小反應越快，`plant-gain` 建議 1.0 起步。
- Example（100 Hz）
  - `cargo run -p sim2d -- --hz 100 -v 0.6 --kp 0.8 --ki 0.2 --kd 0.0 --tau 0.2 --plant-gain 1.0`

📦 Roadmap
	•	v0:
	•	Core traits + PID + fail-safe
	•	Bevy + Rapier2D simulation
	•	Raspberry Pi stub + aarch64 cross-compile
	•	CI workflow (clippy, test, build artifacts)
	•	v1:
	•	Angular velocity + kinematics
	•	Headless simulation for CI
	•	Configurable multi-robot sim
	•	Prometheus metrics

⸻

📚 Documentation
	•	docs/ARCHITECTURE.md – modules, data flow, extensibility
	•	docs/API.md – HAL traits, control APIs, and usage snippets
	•	docs/TUNING.md – PID/FailSafe/plant 調參流程與建議
	•	CSV schema (platform/sim2d unified): `t,dt,desired_v,left,right,distance,state,meas_left,meas_right,err,adapt_gain`

⸻

⚖️ License

MIT or Apache-2.0 (choose one).
