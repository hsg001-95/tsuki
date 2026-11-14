#!/usr/bin/env python3
"""
OS-FaultBench Responsiveness Probe.
Ground-truth measurement engine for OS-level freezes and latency stalls.
Operates independently of monitored daemons.
Measures:
  1. Scheduler wake-up latency (thread preemption & scheduling delays)
  2. Synchronous file I/O latency (kernel page-writeback and lock contention)
Calculates rolling p99 latency over 5-second windows.
Ground truth freeze: p99 latency > 500 ms over 5 seconds.
"""

import argparse
import json
import os
import tempfile
import threading
import time
from collections import deque
from dataclasses import asdict, dataclass
from typing import List, Optional


@dataclass
class ProbeSample:
    timestamp_ms: int
    wakeup_lat_ms: float
    io_lat_ms: float
    total_lat_ms: float
    rolling_p99_ms: float
    is_freeze: bool


class ResponsivenessProbe:
    def __init__(
        self,
        interval_sec: float = 0.05,  # 20 Hz
        window_sec: float = 5.0,
        freeze_threshold_ms: float = 500.0,
        log_file: Optional[str] = None,
    ):
        self.interval_sec = interval_sec
        self.window_sec = window_sec
        self.freeze_threshold_ms = freeze_threshold_ms
        self.log_file = log_file

        self._window_samples = int(window_sec / interval_sec)
        self._lat_history: deque[float] = deque(maxlen=self._window_samples)
        self._records: List[ProbeSample] = []

        self._stop_event = threading.Event()
        self._thread: Optional[threading.Thread] = None

        # Setup probe temp file on disk for synchronous I/O test
        self._temp_dir = tempfile.mkdtemp(prefix="tsuki_probe_")
        self._io_test_file = os.path.join(self._temp_dir, "io_probe.dat")
        self._payload = b"X" * 4096

    def start(self):
        self._stop_event.clear()
        self._thread = threading.Thread(target=self._run_loop, daemon=True)
        self._thread.start()

    def stop(self) -> List[ProbeSample]:
        self._stop_event.set()
        if self._thread:
            self._thread.join(timeout=3.0)

        # Cleanup temp file
        try:
            if os.path.exists(self._io_test_file):
                os.remove(self._io_test_file)
            if os.path.exists(self._temp_dir):
                os.rmdir(self._temp_dir)
        except Exception:
            pass

        if self.log_file:
            self.save_json(self.log_file)

        return self._records

    def _run_loop(self):
        while not self._stop_event.is_set():
            t_before = time.perf_counter()
            time.sleep(self.interval_sec)
            t_after = time.perf_counter()

            # 1. Measure wake-up latency (preemption stall)
            actual_sleep = t_after - t_before
            wakeup_lat_ms = max(0.0, (actual_sleep - self.interval_sec) * 1000.0)

            # 2. Measure synchronous disk write latency (4KB fdatasync)
            t_io_start = time.perf_counter()
            try:
                fd = os.open(
                    self._io_test_file,
                    os.O_WRONLY | os.O_CREAT | os.O_TRUNC,
                    0o644,
                )
                os.write(fd, self._payload)
                os.fdatasync(fd)
                os.close(fd)
                io_lat_ms = (time.perf_counter() - t_io_start) * 1000.0
            except Exception:
                io_lat_ms = 0.0

            total_lat_ms = wakeup_lat_ms + io_lat_ms
            self._lat_history.append(total_lat_ms)

            # Calculate rolling p99
            sorted_history = sorted(self._lat_history)
            p99_idx = int(len(sorted_history) * 0.99)
            p99_ms = sorted_history[min(p99_idx, len(sorted_history) - 1)]

            is_freeze = p99_ms >= self.freeze_threshold_ms

            sample = ProbeSample(
                timestamp_ms=int(time.time() * 1000),
                wakeup_lat_ms=round(wakeup_lat_ms, 3),
                io_lat_ms=round(io_lat_ms, 3),
                total_lat_ms=round(total_lat_ms, 3),
                rolling_p99_ms=round(p99_ms, 3),
                is_freeze=is_freeze,
            )
            self._records.append(sample)

    def save_json(self, path: str):
        with open(path, "w") as f:
            json.dump([asdict(r) for r in self._records], f, indent=2)

    def get_summary(self) -> dict:
        if not self._records:
            return {"samples": 0, "max_p99_ms": 0.0, "freeze_count": 0}

        p99_values = [r.rolling_p99_ms for r in self._records]
        freeze_samples = sum(1 for r in self._records if r.is_freeze)

        return {
            "samples": len(self._records),
            "max_p99_ms": max(p99_values),
            "avg_p99_ms": round(sum(p99_values) / len(p99_values), 2),
            "freeze_samples": freeze_samples,
            "freeze_duration_sec": round(freeze_samples * self.interval_sec, 2),
        }


def main():
    parser = argparse.ArgumentParser(description="Tsuki OS-FaultBench Responsiveness Probe")
    parser.add_argument("--duration", type=int, default=3, help="Duration to run probe in seconds")
    parser.add_argument("--threshold", type=float, default=500.0, help="Freeze threshold in ms")
    parser.add_argument("--output", type=str, default="/tmp/probe_output.json", help="Output JSON path")
    args = parser.parse_args()

    probe = ResponsivenessProbe(
        interval_sec=0.05,
        window_sec=2.0,
        freeze_threshold_ms=args.threshold,
        log_file=args.output,
    )

    print(f"[*] Starting Responsiveness Probe for {args.duration}s (threshold={args.threshold}ms)...")
    probe.start()
    time.sleep(args.duration)
    records = probe.stop()

    summary = probe.get_summary()
    print("\n=== Probe Summary ===")
    print(f"Total Samples    : {summary['samples']}")
    print(f"Max p99 Latency  : {summary['max_p99_ms']} ms")
    print(f"Avg p99 Latency  : {summary['avg_p99_ms']} ms")
    print(f"Freeze Duration  : {summary['freeze_duration_sec']} s")
    print(f"Logs saved to    : {args.output}")


if __name__ == "__main__":
    main()
