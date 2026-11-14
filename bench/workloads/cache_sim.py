#!/usr/bin/env python3
"""
OS-FaultBench Workload: In-Memory Cache Simulator.
Simulates high-throughput key-value caches (like Redis) with fork-based snapshotting.
Demonstrates page-table replication overhead and COW fragmentation.
"""

import argparse
import os
import random
import sys
import threading
import time


class CacheWorkload:
    def __init__(self, key_count: int = 100_000, val_size_bytes: int = 512):
        self.key_count = key_count
        self.val_size_bytes = val_size_bytes
        self._data = {}
        self._stop_event = threading.Event()
        self._worker_thread = None
        self._snapshot_thread = None

        self._populate()

    def _populate(self):
        val = b"V" * self.val_size_bytes
        for i in range(self.key_count):
            self._data[f"k:{i}"] = val

    def _worker_loop(self, duration_sec: float):
        start = time.time()
        while time.time() - start < duration_sec and not self._stop_event.is_set():
            k = f"k:{random.randint(0, self.key_count - 1)}"
            if random.random() < 0.8:
                _ = self._data.get(k)
            else:
                self._data[k] = b"U" * self.val_size_bytes
            time.sleep(0.0005)

    def _snapshot_loop(self, duration_sec: float):
        start = time.time()
        while time.time() - start < duration_sec and not self._stop_event.is_set():
            time.sleep(1.0)
            if self._stop_event.is_set():
                break

            # Simulate Redis BGSAVE via fork
            pid = os.fork()
            if pid == 0:
                # Child process: simulate writing RDB file and exit
                try:
                    time.sleep(0.05)
                finally:
                    os._exit(0)
            else:
                # Parent process: wait for child
                try:
                    os.waitpid(pid, 0)
                except Exception:
                    pass

    def start(self, duration_sec: float):
        self._stop_event.clear()
        self._worker_thread = threading.Thread(
            target=self._worker_loop, args=(duration_sec,), daemon=True
        )
        self._snapshot_thread = threading.Thread(
            target=self._snapshot_loop, args=(duration_sec,), daemon=True
        )

        self._worker_thread.start()
        self._snapshot_thread.start()

    def stop(self):
        self._stop_event.set()
        if self._worker_thread:
            self._worker_thread.join(timeout=2.0)
        if self._snapshot_thread:
            self._snapshot_thread.join(timeout=2.0)


def main():
    parser = argparse.ArgumentParser(description="Cache Workload Simulator")
    parser.add_argument("--keys", type=int, default=50_000, help="Number of keys")
    parser.add_argument("--duration", type=int, default=3, help="Duration in seconds")
    args = parser.parse_args()

    print(f"[*] Starting Cache Workload: {args.keys} keys for {args.duration}s...")
    wl = CacheWorkload(key_count=args.keys)
    wl.start(args.duration)
    time.sleep(args.duration)
    wl.stop()
    print("[*] Cache Workload finished.")


if __name__ == "__main__":
    main()
