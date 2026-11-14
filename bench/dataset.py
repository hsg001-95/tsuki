#!/usr/bin/env python3
"""
OS-FaultBench Dataset Partitioner and Split Manager.
Implements Rule ML-01: Train/validation/test splits MUST separate workload families
(Leave-One-Workload-Family-Out validation).
Never split randomly across adjacent time windows to prevent data leakage.
"""

import glob
import os
from typing import Dict, List, Tuple
import numpy as np
import pandas as pd


FEATURE_COLUMNS = [
    "psi_mem_some",
    "psi_mem_full",
    "psi_cpu_some",
    "psi_io_some",
    "psi_io_full",
    "pgfault_rate",
    "pgmajfault_rate",
    "refault_rate",
]

TARGET_COLUMN = "early_warning_label"


class FaultBenchDataset:
    def __init__(self, data_dir: str = "bench/data"):
        self.data_dir = data_dir
        self.df = self._load_all_parquets()

    def _load_all_parquets(self) -> pd.DataFrame:
        files = glob.glob(os.path.join(self.data_dir, "*.parquet"))
        if not files:
            return pd.DataFrame()

        dfs = [pd.read_parquet(f) for f in files]
        combined = pd.concat(dfs, ignore_index=True)
        return combined

    def summary(self) -> Dict[str, any]:
        if self.df.empty:
            return {"total_samples": 0}

        return {
            "total_samples": len(self.df),
            "scenarios": self.df["scenario_id"].nunique(),
            "workload_families": self.df["workload_family"].unique().tolist(),
            "categories": self.df["category"].unique().tolist(),
            "positive_samples": int(self.df[TARGET_COLUMN].sum()),
            "negative_samples": int((self.df[TARGET_COLUMN] == 0).sum()),
        }

    def get_leave_one_family_out_split(
        self, test_family: str
    ) -> Tuple[np.ndarray, np.ndarray, np.ndarray, np.ndarray]:
        """
        Partition dataset: train on all workload families EXCEPT test_family.
        Evaluates out-of-distribution generalization.
        Returns: (X_train, y_train, X_test, y_test)
        """
        if self.df.empty:
            raise ValueError("No data available in dataset.")

        train_mask = self.df["workload_family"] != test_family
        test_mask = self.df["workload_family"] == test_family

        train_df = self.df[train_mask]
        test_df = self.df[test_mask]

        X_train = train_df[FEATURE_COLUMNS].to_numpy(dtype=np.float32)
        y_train = train_df[TARGET_COLUMN].to_numpy(dtype=np.int64)

        X_test = test_df[FEATURE_COLUMNS].to_numpy(dtype=np.float32)
        y_test = test_df[TARGET_COLUMN].to_numpy(dtype=np.int64)

        return X_train, y_train, X_test, y_test


def main():
    ds = FaultBenchDataset()
    print("=== OS-FaultBench Dataset Summary ===")
    print(ds.summary())


if __name__ == "__main__":
    main()
