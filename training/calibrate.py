#!/usr/bin/env python3
"""
Tsuki Model Calibration & Threshold Selector.
Implements Rule ML-03 & NFR-6:
- Calibrates early-warning detector probabilities using Temperature Scaling.
- Chooses operational decision threshold on validation data under strict false-alarm budget.
- Never optimizes decision thresholds on test data.
"""

import argparse
import numpy as np


class TemperatureScaler:
    def __init__(self, temperature: float = 1.0):
        self.temperature = max(0.01, temperature)

    def fit(self, logits: np.ndarray, y_val: np.ndarray):
        """Fit temperature parameter to minimize cross-entropy on validation set."""
        best_t = 1.0
        best_nll = float("inf")

        for t_candidate in np.linspace(0.1, 5.0, 50):
            scaled_logits = logits / t_candidate
            probs = 1.0 / (1.0 + np.exp(-scaled_logits))
            probs = np.clip(probs, 1e-7, 1.0 - 1e-7)

            # Negative log-likelihood
            nll = -np.mean(y_val * np.log(probs) + (1.0 - y_val) * np.log(1.0 - probs))
            if nll < best_nll:
                best_nll = nll
                best_t = t_candidate

        self.temperature = best_t
        print(f"[*] Optimal temperature calibrated: T = {self.temperature:.3f} (NLL: {best_nll:.4f})")

    def transform(self, logits: np.ndarray) -> np.ndarray:
        scaled = logits / self.temperature
        return 1.0 / (1.0 + np.exp(-scaled))


def find_threshold_under_false_alarm_budget(
    val_probs: np.ndarray, y_val: np.ndarray, max_false_alarms_per_hour: float = 1.0
) -> float:
    """
    Search thresholds from high to low to find operating point
    that satisfies the NFR-6 false alarm budget on healthy/negative validation samples.
    """
    negatives = val_probs[y_val == 0]
    if len(negatives) == 0:
        return 0.5

    # Target false positive rate: assuming 1 sample/sec (3600 samples/hr)
    target_fpr = max_false_alarms_per_hour / 3600.0

    thresholds = np.linspace(0.99, 0.05, 100)
    selected_threshold = 0.5

    for th in thresholds:
        fpr = (negatives >= th).mean()
        if fpr <= target_fpr:
            selected_threshold = th
            break

    print(f"[*] Selected decision threshold under budget: {selected_threshold:.3f} (FPR: {fpr:.6f})")
    return selected_threshold


def main():
    parser = argparse.ArgumentParser(description="Calibrate Model and Set Operational Threshold")
    parser.add_argument("--max-fa-per-hr", type=float, default=1.0, help="Max allowed false alarms per 8 hrs")
    args = parser.parse_args()

    # Synthetic validation demo
    np.random.seed(42)
    val_logits = np.random.normal(0.0, 2.0, 200)
    y_val = (val_logits > 1.0).astype(int)

    scaler = TemperatureScaler()
    scaler.fit(val_logits, y_val)
    calibrated_probs = scaler.transform(val_logits)

    th = find_threshold_under_false_alarm_budget(
        calibrated_probs, y_val, max_false_alarms_per_hour=args.max_fa_per_hr / 8.0
    )


if __name__ == "__main__":
    main()
