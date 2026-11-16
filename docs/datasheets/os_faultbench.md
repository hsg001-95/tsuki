# Datasheet for Dataset: OS-FaultBench

## 1. Motivation
- **For what purpose was the dataset created?**  
  OS-FaultBench was created to evaluate Linux early-warning fault detection, causal diagnosis, and reversible remediation systems under realistic desktop and developer workstation loads.
- **Who created the dataset?**  
  The Tsuki Project Team (Snoxx Tech / @Ghoy).
- **Who funded the creation of the dataset?**  
  Self-funded open research initiative.

## 2. Composition
- **What do the instances comprise?**  
  Synchronous time-series telemetry rows sampled at 1 Hz–10 Hz comprising:
  - Linux Kernel Pressure Stall Information (`/proc/pressure/{memory,cpu,io}`)
  - Virtual memory counters from `/proc/vmstat` (e.g. `workingset_refault`, `pgscan_kswapd`, `pgmajfault`)
  - Cgroup v2 hierarchical statistics (`memory.current`, `cpu.stat`, `memory.high`)
  - Responsiveness probe ground-truth measurements (synchronous 4KB `O_DIRECT`/`fsync` write latency + scheduler wake-up delay)
- **Labeling:**  
  Binary freeze flags (`is_frozen = true` when rolling 5s $p99 \ge 500\text{ ms}$), anomaly lead times in seconds, and multi-class fault classifications (`memory_thrash`, `cpu_contention`, `io_saturation`, `healthy`).
- **Data Splits:**  
  Partitioned using **Leave-One-Workload-Family-Out (LOWFO)** across 3 disjoint background workload families:
  1. `build_sim`: Parallel compilation & link job tree.
  2. `db_sim`: SQLite WAL concurrent read/write query transactions.
  3. `cache_sim`: In-memory key-value cache with periodic BGSAVE fork snapshotting.

## 3. Collection Process
- **Data Acquisition Mechanism:**  
  Automated bench harness (`bench/harness.py`) executing declared YAML scenario specifications on Ubuntu Linux kernel 6.x with cgroup v2 unified hierarchy.
- **Sampling Rate:**  
  Adaptive 1.0 Hz baseline ramping to 5.0–10.0 Hz under anomaly pressure. Zero synthetic interpolation or extrapolation is applied.
- **Scrubbing & Privacy:**  
  Hostnames, raw PIDs, user credentials, environmental paths, and internal memory contents are strictly omitted. Only relative cgroup paths and aggregate kernel pressure statistics are retained.

## 4. Preprocessing & Uses
- **Feature Extraction:**  
  Calculated over a fixed 60-sample sliding window: moving averages, 10s derivatives ($\Delta\%/\text{s}$), and acceleration ratios.
- **Appropriate Uses:**  
  Benchmarking early-warning freeze detectors, proactive memory thrashing controllers, and Linux user-space OOM mitigation daemons.
- **Inappropriate Uses:**  
  Predicting specific user keyboard keystrokes, application-specific business logic bugs, or profiling proprietary algorithms.

## 5. Distribution & Licensing
- **Format:** Apache Parquet with Snappy compression and typed schemas.
- **License:** Creative Commons Attribution 4.0 International (CC BY 4.0).
- **Maintenance:** Managed via the Tsuki open source repository.
