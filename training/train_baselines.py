#!/usr/bin/env python3
"""
OS-FaultBench Baseline Model Evaluator.
Implements Tasks 3.1 & 3.2:
- Task 3.1: PSI threshold rule heuristic baseline
- Task 3.2: Gradient-boosted tree baseline (HistGradientBoosting / LightGBM)
Evaluates out-of-distribution performance on Leave-One-Workload-Family-Out splits.
"""

import argparse
import sys
import os
import numpy as np

# Ensure bench is in PYTHONPATH
sys.path.insert(0, os.path.abspath("bench"))
from dataset import FaultBenchDataset, FEATURE_COLUMNS, TARGET_COLUMN
from sklearn.ensemble import HistGradientBoostingClassifier
from sklearn.metrics import (
    average_precision_score,
    classification_report,
    roc_auc_score,
    precision_recall_curve,
)


class HeuristicThresholdBaseline:
    def __init__(self, mem_threshold: float = 20.0, cpu_threshold: float = 60.0):
        self.mem_threshold = mem_threshold
        self.cpu_threshold = cpu_threshold

    def predict_proba(self, X: np.ndarray) -> np.ndarray:
        # Columns: 0=psi_mem_some, 2=psi_cpu_some
        mem = X[:, 0]
        cpu = X[:, 2]

        score = np.maximum(
            np.clip(mem / self.mem_threshold, 0.0, 1.0),
            np.clip(cpu / self.cpu_threshold, 0.0, 1.0),
        )
        return np.column_stack([1.0 - score, score])

    def predict(self, X: np.ndarray) -> np.ndarray:
        proba = self.predict_proba(X)[:, 1]
        return (proba >= 0.5).astype(int)


def evaluate_model(name: str, y_true: np.ndarray, y_proba: np.ndarray):
    print(f"\n--- {name} Results ---")
    if len(np.unique(y_true)) < 2:
        print(f"[!] Warning: Only one class present in test ground truth: {np.unique(y_true)}")
        preds = (y_proba >= 0.5).astype(int)
        acc = (preds == y_true).mean()
        print(f"Accuracy: {acc:.4f} (single class)")
        return {"pr_auc": 1.0, "roc_auc": 1.0, "accuracy": acc}

    pr_auc = average_precision_score(y_true, y_proba)
    roc_auc = roc_auc_score(y_true, y_proba)
    preds = (y_proba >= 0.5).astype(int)

    print(f"PR-AUC (Average Precision): {pr_auc:.4f}")
    print(f"ROC-AUC                  : {roc_auc:.4f}")
    print("\nClassification Report:")
    print(classification_report(y_true, preds, zero_division=0))

    return {"pr_auc": pr_auc, "roc_auc": roc_auc}


def main():
    parser = argparse.ArgumentParser(description="Train and Evaluate Baseline Models")
    parser.add_argument("--test-family", type=str, default="build", help="Workload family to hold out")
    args = parser.parse_args()

    ds = FaultBenchDataset("bench/data")
    summary = ds.summary()
    print(f"Loaded dataset: {summary['total_samples']} samples across {summary['scenarios']} scenarios.")

    if summary["total_samples"] == 0:
        print("Error: No benchmark data found. Run bench/harness.py first.")
        return

    try:
        X_train, y_train, X_test, y_test = ds.get_leave_one_family_out_split(args.test_family)
    except Exception as e:
        print(f"Splitting error: {e}")
        return

    print(f"Train samples: {len(X_train)} | Test samples (held-out family '{args.test_family}'): {len(X_test)}")

    # 1. Evaluate Heuristic Baseline
    heuristic = HeuristicThresholdBaseline()
    h_proba = heuristic.predict_proba(X_test)[:, 1]
    evaluate_model("Heuristic Threshold Baseline", y_test, h_proba)

    # 2. Evaluate Gradient-Boosted Tree Baseline
    if len(np.unique(y_train)) >= 2:
        tree = HistGradientBoostingClassifier(
            max_iter=100,
            learning_rate=0.05,
            class_weight="balanced",
            random_state=42,
        )
        tree.fit(X_train, y_train)
        t_proba = tree.predict_proba(X_test)[:, 1]
        evaluate_model("Gradient-Boosted Tree (HistGB) Baseline", y_test, t_proba)
    else:
        print("[!] Note: Train set contains single class in current synthetic run; tree training skipped until more samples are generated.")


if __name__ == "__main__":
    main()
