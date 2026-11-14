#!/usr/bin/env python3
"""
OS-FaultBench Workload: Parallel Build Simulator.
Simulates parallel compilation pipelines (e.g., Linux kernel or LLVM builds).
Generates CPU activity across worker threads and realistic memory allocations.
"""

import argparse
import multiprocessing
import os
import tempfile
import time


def compile_worker(worker_id: int, duration_sec: float, memory_mb: int, stop_flag):
    # Allocate simulated AST / object buffers
    chunk_size = 1024 * 1024  # 1 MB
    allocated = []
    try:
        for _ in range(max(1, memory_mb)):
            allocated.append(bytearray(os.urandom(chunk_size)))
    except MemoryError:
        pass

    temp_dir = tempfile.mkdtemp(prefix=f"build_worker_{worker_id}_")
    obj_file = os.path.join(temp_dir, "test.o")

    start = time.time()
    counter = 0

    while time.time() - start < duration_sec and not stop_flag.value:
        # Simulate CPU parsing / code generation
        h = 0
        for b in allocated[:2]:
            h = (h * 31 + sum(b[:1024])) % 1000000007

        # Simulate writing object file
        with open(obj_file, "wb") as f:
            f.write(b"ELF" + bytes([h % 256] * 4096))

        counter += 1
        time.sleep(0.01)

    try:
        if os.path.exists(obj_file):
            os.remove(obj_file)
        os.rmdir(temp_dir)
    except Exception:
        pass


class BuildWorkload:
    def __init__(self, jobs: int = 4, memory_per_job_mb: int = 64):
        self.jobs = jobs
        self.memory_per_job_mb = memory_per_job_mb
        self._processes = []
        self._stop_flag = multiprocessing.Value("b", False)

    def start(self, duration_sec: float):
        self._stop_flag.value = False
        self._processes = []
        for i in range(self.jobs):
            p = multiprocessing.Process(
                target=compile_worker,
                args=(i, duration_sec, self.memory_per_job_mb, self._stop_flag),
                daemon=True,
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
    parser = argparse.ArgumentParser(description="Parallel Build Workload Simulator")
    parser.add_argument("--jobs", type=int, default=4, help="Number of parallel jobs")
    parser.add_argument("--duration", type=int, default=5, help="Duration in seconds")
    parser.add_argument("--memory-mb", type=int, default=64, help="Memory allocated per job")
    args = parser.parse_args()

    print(f"[*] Starting Build Workload: {args.jobs} jobs, {args.memory_mb} MB/job for {args.duration}s...")
    wl = BuildWorkload(jobs=args.jobs, memory_per_job_mb=args.memory_mb)
    wl.start(args.duration)
    time.sleep(args.duration)
    wl.stop()
    print("[*] Build Workload finished.")


if __name__ == "__main__":
    main()
