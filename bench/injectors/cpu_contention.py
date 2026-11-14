#!/usr/bin/env python3
"""
OS-FaultBench Fault Injector: CPU Contention / Noisy Neighbor.
Spawns aggressive spinning compute threads to saturate CPU run-queues and induce scheduling latency.
"""

import argparse
import multiprocessing
import time


def cpu_spinner(stop_flag):
    while not stop_flag.value:
        # Tightly looping compute
        _ = sum(i * i for i in range(10_000))


class CpuContentionInjector:
    def __init__(self, threads: int = 4):
        self.threads = threads
        self._processes = []
        self._stop_flag = multiprocessing.Value("b", False)

    def start(self, duration_sec: float):
        self._stop_flag.value = False
        self._processes = []
        for _ in range(self.threads):
            p = multiprocessing.Process(
                target=cpu_spinner, args=(self._stop_flag,), daemon=True
            )
            p.start()
            self._processes.append(p)

    def stop(self):
        self._stop_flag.value = True
        for p in self._processes:
            p.join(timeout=2.0)
            if p.is_alive():
                p.terminate()


def main():
    parser = argparse.ArgumentParser(description="CPU Contention Fault Injector")
    parser.add_argument("--threads", type=int, default=4, help="Number of spinning threads")
    parser.add_argument("--duration", type=int, default=5, help="Duration in seconds")
    args = parser.parse_args()

    print(f"[*] Starting CPU Contention Fault: {args.threads} threads for {args.duration}s...")
    inj = CpuContentionInjector(threads=args.threads)
    inj.start(args.duration)
    time.sleep(args.duration)
    inj.stop()
    print("[*] CPU Contention finished.")


if __name__ == "__main__":
    main()
