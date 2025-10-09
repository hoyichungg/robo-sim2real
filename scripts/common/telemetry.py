"""Shared helpers for reading and analysing telemetry CSV files."""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Optional, Sequence, Tuple

import numpy as np
import pandas as pd

DEFAULT_CANDIDATES: Tuple[str, ...] = ("run.csv", "run/telemetry.csv", "run/sim.csv")


def resolve_csv_path(path: Optional[str], candidates: Sequence[str] = DEFAULT_CANDIDATES) -> Path:
    """Resolve a telemetry CSV path.

    If *path* is provided it must exist, otherwise the first existing candidate is used.
    """

    if path:
        candidate = Path(path)
        if not candidate.exists():
            raise FileNotFoundError(f"CSV not found: {candidate}")
        return candidate

    for cand in candidates:
        candidate = Path(cand)
        if candidate.exists():
            return candidate

    raise FileNotFoundError(
        "Could not locate telemetry CSV. Provide --csv or place a file in run/"
    )


def load_dataframe(path: Path | str) -> pd.DataFrame:
    """Load a telemetry CSV as a pandas DataFrame."""

    return pd.read_csv(path)


def load_telemetry(path: Optional[str], candidates: Sequence[str] = DEFAULT_CANDIDATES) -> Tuple[pd.DataFrame, Path]:
    """Resolve and load telemetry, returning (DataFrame, resolved_path)."""

    resolved = resolve_csv_path(path, candidates)
    return load_dataframe(resolved), resolved


def measured_velocity(df: pd.DataFrame) -> Tuple[np.ndarray, bool]:
    """Return measured velocity and whether it came from dedicated measurement columns."""

    if {"meas_left", "meas_right"}.issubset(df.columns):
        meas = 0.5 * (df["meas_left"].to_numpy(float) + df["meas_right"].to_numpy(float))
        if np.isfinite(meas).any():
            return meas, True
    if {"vel_left", "vel_right"}.issubset(df.columns):
        meas = 0.5 * (df["vel_left"].to_numpy(float) + df["vel_right"].to_numpy(float))
        if np.isfinite(meas).any():
            return meas, True
    # fall back to command average
    return 0.5 * (_to_numpy(df, "left") + _to_numpy(df, "right")), False


def _to_numpy(df: pd.DataFrame, column: str, default: float = 0.0) -> np.ndarray:
    if column in df.columns:
        return df[column].to_numpy(float)
    return np.full(len(df), default, dtype=float)


def _to_state(df: pd.DataFrame) -> np.ndarray:
    if "state" in df.columns:
        return df["state"].astype(str).to_numpy()
    return np.array(["Run"] * len(df), dtype=str)


def _to_error(df: pd.DataFrame, desired: np.ndarray, measured: np.ndarray) -> np.ndarray:
    if "err" in df.columns:
        return df["err"].to_numpy(float)
    return desired - measured


def _to_adapt(df: pd.DataFrame, length: int) -> np.ndarray:
    if "adapt_gain" in df.columns:
        return df["adapt_gain"].to_numpy(float)
    return np.ones(length, dtype=float)


@dataclass
class TelemetryVectors:
    time: np.ndarray
    desired: np.ndarray
    measured: np.ndarray
    left: np.ndarray
    right: np.ndarray
    distance: np.ndarray
    state: np.ndarray
    error: np.ndarray
    adapt: np.ndarray
    has_measured: bool

    def failsafe_time(self, run_label: str = "Run") -> float:
        indices = np.where(self.state != run_label)[0]
        if len(indices):
            return float(self.time[int(indices[0])])
        return float("inf")

    def run_mask(self, run_label: str = "Run") -> np.ndarray:
        t_fail = self.failsafe_time(run_label)
        return (self.state == run_label) & (self.time < t_fail)


def telemetry_vectors(df: pd.DataFrame) -> TelemetryVectors:
    time = _to_numpy(df, "t")
    desired = _to_numpy(df, "desired_v")
    left = _to_numpy(df, "left")
    right = _to_numpy(df, "right")
    measured, has_measured = measured_velocity(df)
    distance = _to_numpy(df, "distance", default=float("nan"))
    state = _to_state(df)
    error = _to_error(df, desired, measured)
    adapt = _to_adapt(df, len(df))

    return TelemetryVectors(
        time=time,
        desired=desired,
        measured=measured,
        left=left,
        right=right,
        distance=distance,
        state=state,
        error=error,
        adapt=adapt,
        has_measured=has_measured,
    )
