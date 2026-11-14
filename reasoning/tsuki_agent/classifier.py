"""
Tsuki Fault Classifier Service.
Classifies telemetry windows into distinct operating system failure modes:
  - memory_thrash: Reclaim death spiral, high refaults, and page-in stalls
  - cpu_contention: Scheduling queue saturation, thread preemption stalls
  - io_saturation: Block I/O full writeback stalls
"""

from typing import Dict, Tuple


class FaultClassifier:
    def classify(self, sample: Dict[str, any]) -> Tuple[str, float]:
        pressure = sample.get("pressure", {})
        mem = pressure.get("memory", {})
        cpu = pressure.get("cpu", {})
        io = pressure.get("io", {})
        vmstat = sample.get("vmstat", {})

        mem_some = mem.get("some_avg10", 0.0)
        mem_full = mem.get("full_avg10", 0.0)
        cpu_some = cpu.get("some_avg10", 0.0)
        io_full = io.get("full_avg10", 0.0)
        refault = vmstat.get("workingset_refault", 0)

        # 1. Memory thrashing check
        if mem_full > 10.0 or (mem_some > 20.0 and refault > 500):
            conf = min(0.98, 0.70 + (mem_full / 50.0))
            return "memory_thrash", round(conf, 2)

        # 2. CPU contention check
        if cpu_some > 50.0:
            conf = min(0.95, 0.65 + (cpu_some / 150.0))
            return "cpu_contention", round(conf, 2)

        # 3. I/O saturation check
        if io_full > 20.0:
            conf = min(0.95, 0.65 + (io_full / 100.0))
            return "io_saturation", round(conf, 2)

        return "healthy", 0.99
