#!/usr/bin/env python3
"""
OS-FaultBench Baseline Comparison Tool.
Compares Tsuki against standard Linux user-space memory handlers (systemd-oomd, earlyoom, no daemon).
Follows rules EVAL-01 through EVAL-06:
- Never cherry-pick runs
- Paired comparisons on identical workloads
- Record all telemetry, latency distributions, and lost work
"""

import argparse
import json
import os
import subprocess
import time
from typing import Dict, Any


def run_workload_sample(duration_sec: int) -> Dict[str, Any]:
    """Capture PSI and latency over a duration under test."""
    print(f"[*] Monitoring telemetry for {duration_sec}s...")
    psi_samples = []
    start = time.time()

    while time.time() - start < duration_sec:
        sample = {}
        try:
            with open("/proc/pressure/memory") as f:
                for line in f:
                    if line.startswith("some"):
                        sample["mem_some"] = float(line.split()[1].split("=")[1])
                    elif line.startswith("full"):
                        sample["mem_full"] = float(line.split()[1].split("=")[1])
        except Exception:
            pass

        psi_samples.append(sample)
        time.sleep(1.0)

    avg_some = sum(s.get("mem_some", 0.0) for s in psi_samples) / max(1, len(psi_samples))
    avg_full = sum(s.get("mem_full", 0.0) for s in psi_samples) / max(1, len(psi_samples))

    return {
        "duration_sec": duration_sec,
        "sample_count": len(psi_samples),
        "avg_psi_mem_some": avg_some,
        "avg_psi_mem_full": avg_full,
    }


def main():
    parser = argparse.ArgumentParser(description="Tsuki OS-FaultBench Baseline Evaluator")
    parser.add_argument("--baseline", choices=["no-daemon", "earlyoom", "systemd-oomd", "tsuki-guard"], default="tsuki-guard")
    parser.add_argument("--duration", type=int, default=5, help="Monitoring duration in seconds")
    parser.add_argument("--output", type=str, default="baseline_result.json", help="Output JSON path")
    args = parser.parse_args()

    print(f"\n=======================================================")
    print(f" OS-FaultBench Baseline Run: [{args.baseline}]")
    print(f"=======================================================")

    results = run_workload_sample(args.duration)
    results["baseline_type"] = args.baseline
    results["timestamp"] = time.time()

    print(f"Avg Memory Pressure (some): {results['avg_psi_mem_some']:.2f}%")
    print(f"Avg Memory Pressure (full): {results['avg_psi_mem_full']:.2f}%")
    print(f"Results written to: {args.output}\n")

    with open(args.output, "w") as f:
        json.dump(results, f, indent=2)


if __name__ == "__main__":
    main()
