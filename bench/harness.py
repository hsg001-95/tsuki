#!/usr/bin/env python3
"""
OS-FaultBench Scenario Runner and Ground-Truth Dataset Generator.
Executes declarative scenario YAMLs, runs concurrent background workloads,
injects calibrated faults, records probe latency ground truth, and exports
labeled time-series Parquet datasets for model training and baseline evaluation.
Follows rules EVAL-01 through EVAL-06 and PRD requirements.
"""

import argparse
import json
import os
import sys
import threading
import time
from typing import Any, Dict, List
import pandas as pd
import yaml

from probe import ResponsivenessProbe
from workloads.build_sim import BuildWorkload
from workloads.cache_sim import CacheWorkload
from workloads.db_sim import DatabaseWorkload
from injectors.cpu_contention import CpuContentionInjector
from injectors.mem_thrash import MemoryThrashInjector


def sample_telemetry() -> Dict[str, Any]:
    """Capture instantaneous PSI and vmstat counters."""
    ts_ms = int(time.time() * 1000)
    data = {
        "timestamp_ms": ts_ms,
        "psi_mem_some": 0.0,
        "psi_mem_full": 0.0,
        "psi_cpu_some": 0.0,
        "psi_io_some": 0.0,
        "psi_io_full": 0.0,
        "pgfault": 0,
        "pgmajfault": 0,
        "refault": 0,
    }

    try:
        with open("/proc/pressure/memory") as f:
            for line in f:
                parts = line.split()
                if parts[0] == "some":
                    data["psi_mem_some"] = float(parts[1].split("=")[1])
                elif parts[0] == "full":
                    data["psi_mem_full"] = float(parts[1].split("=")[1])
    except Exception:
        pass

    try:
        with open("/proc/pressure/cpu") as f:
            for line in f:
                parts = line.split()
                if parts[0] == "some":
                    data["psi_cpu_some"] = float(parts[1].split("=")[1])
    except Exception:
        pass

    try:
        with open("/proc/pressure/io") as f:
            for line in f:
                parts = line.split()
                if parts[0] == "some":
                    data["psi_io_some"] = float(parts[1].split("=")[1])
                elif parts[0] == "full":
                    data["psi_io_full"] = float(parts[1].split("=")[1])
    except Exception:
        pass

    try:
        with open("/proc/vmstat") as f:
            for line in f:
                parts = line.split()
                if parts[0] == "pgfault":
                    data["pgfault"] = int(parts[1])
                elif parts[0] == "pgmajfault":
                    data["pgmajfault"] = int(parts[1])
                elif parts[0].startswith("workingset_refault"):
                    data["refault"] += int(parts[1])
    except Exception:
        pass

    return data


class ScenarioHarness:
    def __init__(self, config_path: str):
        with open(config_path) as f:
            self.cfg = yaml.safe_load(f)

        self.scenario_id = self.cfg["scenario_id"]
        self.category = self.cfg.get("category", "generic")
        self.timeline = self.cfg.get("timeline", {})
        self.workload_cfg = self.cfg.get("workload", {})
        self.fault_cfg = self.cfg.get("fault", {})
        self.labels_cfg = self.cfg.get("labels", {})

        self.total_duration = self.timeline.get("total_duration_sec", 15.0)
        self.horizon_sec = self.timeline.get("horizon_sec", 30.0)

    def _create_workload(self):
        w_type = self.workload_cfg.get("type", "build")
        if w_type == "build":
            return BuildWorkload(
                jobs=self.workload_cfg.get("jobs", 4),
                memory_per_job_mb=self.workload_cfg.get("memory_per_job_mb", 64),
            )
        elif w_type == "db":
            return DatabaseWorkload(clients=self.workload_cfg.get("clients", 4))
        elif w_type == "cache":
            return CacheWorkload(key_count=self.workload_cfg.get("keys", 50_000))
        return None

    def _create_injector(self):
        f_type = self.fault_cfg.get("type", "none")
        if f_type == "memory_thrash":
            return MemoryThrashInjector(target_mb=self.fault_cfg.get("target_mb", 256))
        elif f_type == "cpu_contention":
            return CpuContentionInjector(threads=self.fault_cfg.get("threads", 4))
        return None

    def run(self, output_dir: str = "bench/data") -> str:
        os.makedirs(output_dir, exist_ok=True)
        print(f"\n=======================================================")
        print(f" OS-FaultBench Executing Scenario: [{self.scenario_id}]")
        print(f" Category: {self.category} | Duration: {self.total_duration}s")
        print(f"=======================================================")

        # 1. Start Responsiveness Probe
        probe = ResponsivenessProbe(interval_sec=0.05, freeze_threshold_ms=500.0)
        probe.start()

        # 2. Start Workload
        workload = self._create_workload()
        if workload:
            print(f"[*] Starting workload ({self.workload_cfg.get('type')})...")
            workload.start(self.total_duration)

        # 3. Telemetry Collection Loop
        telemetry_records: List[Dict[str, Any]] = []
        start_time = time.time()
        injector = self._create_injector()
        injector_started = False

        delay_sec = self.fault_cfg.get("delay_sec", 0.0)
        fault_duration = self.fault_cfg.get("duration_sec", self.total_duration)

        while time.time() - start_time < self.total_duration:
            elapsed = time.time() - start_time

            # Check if time to trigger fault injector
            if injector and not injector_started and elapsed >= delay_sec:
                print(f"[!] Triggering Fault Injector ({self.fault_cfg.get('type')}) at t={elapsed:.1f}s...")
                injector.start(fault_duration)
                injector_started = True

            t_sample = sample_telemetry()
            t_sample["elapsed_sec"] = round(elapsed, 2)
            telemetry_records.append(t_sample)
            time.sleep(1.0)

        # Stop everything cleanly
        if injector:
            injector.stop()
        if workload:
            workload.stop()
        probe_records = probe.stop()

        print("[*] Workload and probe stopped. Merging data and labeling...")

        # 4. Process Ground-Truth Labels
        freeze_timestamps = [
            r.timestamp_ms for r in probe_records if r.is_freeze
        ]

        df = pd.DataFrame(telemetry_records)

        # Compute per-second rate differences
        df["pgfault_rate"] = df["pgfault"].diff().fillna(0).clip(lower=0)
        df["pgmajfault_rate"] = df["pgmajfault"].diff().fillna(0).clip(lower=0)
        df["refault_rate"] = df["refault"].diff().fillna(0).clip(lower=0)

        # Early-warning label: is there a freeze within the horizon (H seconds)?
        horizon_ms = self.horizon_sec * 1000.0
        early_warning_labels = []
        is_frozen_now = []

        for _, row in df.iterrows():
            ts = row["timestamp_ms"]
            # Freeze within [ts, ts + horizon_ms]
            has_future_freeze = any(ts <= f_ts <= ts + horizon_ms for f_ts in freeze_timestamps)
            early_warning_labels.append(1 if has_future_freeze else 0)

            # Active freeze at sample time
            has_current_freeze = any(abs(f_ts - ts) <= 1000 for f_ts in freeze_timestamps)
            is_frozen_now.append(1 if has_current_freeze else 0)

        df["early_warning_label"] = early_warning_labels
        df["is_freeze_ground_truth"] = is_frozen_now
        df["scenario_id"] = self.scenario_id
        df["category"] = self.category
        df["workload_family"] = self.workload_cfg.get("type", "unknown")
        df["fault_type"] = self.labels_cfg.get("expected_fault_type", "none")
        df["is_negative"] = self.labels_cfg.get("is_negative", False)

        # Export to Parquet
        parquet_path = os.path.join(output_dir, f"{self.scenario_id}.parquet")
        df.to_parquet(parquet_path, index=False)
        print(f"[✅] Exported {len(df)} labeled samples to: {parquet_path}")

        # Summary printout
        probe_summary = probe.get_summary()
        print(f"Max p99 Latency: {probe_summary['max_p99_ms']:.2f} ms")
        print(f"Freeze Duration: {probe_summary['freeze_duration_sec']:.2f} s")
        print(f"Early-warning Positive Samples: {sum(early_warning_labels)}/{len(early_warning_labels)}")

        return parquet_path


def main():
    parser = argparse.ArgumentParser(description="OS-FaultBench Scenario Runner")
    parser.add_argument("scenario", type=str, help="Path to scenario YAML config")
    parser.add_argument("--output-dir", type=str, default="bench/data", help="Output directory for Parquet")
    args = parser.parse_args()

    harness = ScenarioHarness(args.scenario)
    harness.run(args.output_dir)


if __name__ == "__main__":
    main()
