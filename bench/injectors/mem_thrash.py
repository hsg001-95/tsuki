#!/usr/bin/env python3
"""
OS-FaultBench Fault Injector: Memory Thrashing.
Induces active memory pressure by allocating and continuously dirtying anonymous pages.
Triggers kernel workingset eviction, page faults, and high PSI memory stall.
"""

import argparse
import multiprocessing
import os
import time


def thrash_worker(target_mb: int, duration_sec: float, stop_flag):
    chunk_size = 1024 * 1024  # 1 MB
    chunks = []

    print(f"[Injector] Allocating {target_mb} MB to induce memory pressure...")
    try:
        for _ in range(target_mb):
            # Allocate and dirty page
            b = bytearray(os.urandom(chunk_size))
            chunks.append(b)
    except MemoryError:
        print("[Injector] Memory limit reached during allocation.")

    start = time.time()
    # Cycle through chunks and dirty memory to prevent idle page deactivation
    idx = 0
    while time.time() - start < duration_sec and not stop_flag.value:
        if chunks:
            chunk = chunks[idx % len(chunks)]
            # Touch pages across the chunk
            for step in range(0, len(chunk), 4096):
                chunk[step] = (chunk[step] + 1) % 256
            idx += 1
        time.sleep(0.005)


class MemoryThrashInjector:
    def __init__(self, target_mb: int = 512):
        self.target_mb = target_mb
        self._proc = None
        self._stop_flag = multiprocessing.Value("b", False)

    def start(self, duration_sec: float):
        self._stop_flag.value = False
        self._proc = multiprocessing.Process(
            target=thrash_worker,
            args=(self.target_mb, duration_sec, self._stop_flag),
            daemon=True,
        )
        self._proc.start()

    def stop(self):
        self._stop_flag.value = True
        if self._proc:
            self._proc.join(timeout=3.0)
            if self._proc.is_alive():
                self._proc.terminate()


def main():
    parser = argparse.ArgumentParser(description="Memory Thrash Fault Injector")
    parser.add_argument("--mb", type=int, default=256, help="Target MB to thrash")
    parser.add_argument("--duration", type=int, default=5, help="Duration in seconds")
    args = parser.parse_args()

    print(f"[*] Starting Memory Thrash Fault: {args.mb} MB for {args.duration}s...")
    inj = MemoryThrashInjector(target_mb=args.mb)
    inj.start(args.duration)
    time.sleep(args.duration)
    inj.stop()
    print("[*] Memory Thrash finished.")


if __name__ == "__main__":
    main()
