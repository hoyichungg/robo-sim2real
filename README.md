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
	•	docs/arch.md: architecture overview & data flow
	•	docs/safety.md: fail-safe rules & ISO 26262/21434 checklist
	•	configs/sim/default.toml: example config

⸻

⚖️ License

MIT or Apache-2.0 (choose one).
