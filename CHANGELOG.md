# Changelog

All notable changes to **Tsuki (月)** will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-16 (Milestone M5 MVP Release)

### Added
- **Guard Plane (`tsuki-guard` in Rust):**
  - Zero-heap allocation telemetry collectors parsing `/proc/pressure/{memory,cpu,io}`, `/proc/vmstat`, and `/sys/fs/cgroup`.
  - Cached cgroup path discovery eliminating the 12.5ms sysfs traversal observer effect down to 0.97ms.
  - Memory locking (`mlockall`) and negative OOM score adjustment (`-900`).
  - Pre-allocated zero-allocation circular sliding window (60 samples).
  - Atomic snapshot writer emitting evidence dumps with FIFO ring-buffer pruning.
- **Early-Warning ML Detection (`training/`):**
  - Dual-headed PyTorch 1D Dilated Temporal Convolutional Network (TCN) predicting freeze probability and lead time.
  - Leave-One-Workload-Family-Out validation partitioner (Rule ML-01).
  - Static shape ONNX opset 18 export (`models/detector.onnx`, 49.8 KB) with numerical equivalence verification.
  - Platt scaling temperature calibration ($T = 0.700$).
  - Certified heuristic fallback bounds (Rule ML-07).
- **Reasoning Plane (`reasoning/` in Python):**
  - Read-only agent tools (`get_snapshot`, `top_cgroups`, `search_docs`, `protected_list`) with zero write authority (Rule SAF-01).
  - Embedded SQLite FTS5 + BM25 hybrid RAG retriever over curated kernel and systemd documentation.
  - Citation enforcement engine automatically purging ungrounded assertions lacking valid doc chunk IDs (Rule ML-08).
  - FastAPI service (`tsuki_api/app.py`) and Streamlit operator dashboard (`tsuki_api/dashboard.py`).
- **Deterministic Actuation & Safety (`tsuki-actuator`, `tsuki-rollbackd` in Rust):**
  - 5-gate deterministic validator enforcing allow-listed reversible actions, cgroup target format (no numeric PIDs, Rule SAF-03), protected targets (Rule SAF-05), parameter bounds, and rate-limiting cooldown (Rule SAF-07).
  - Reversible action catalog (`throttle_memory`, `reweight_cpu`, `freeze_cgroup`).
  - Append-only, fsynced rollback journal (`RollbackJournal`, Rule SAF-04).
  - Independent watchdog daemon (`tsuki-rollbackd`) automatically reverting expired or orphaned actions.
  - Emergency global panic switch (`tsuki-actuator panic`, Rule SAF-08).
- **10 Mandatory Safety Tests (`tests/safety_suite.rs`):**
  - All 10 tests passing with zero failures (hallucinated PID, protected targets, prompt injection, parameter bounds, watchdog crash recovery, reasoning resilience, model fallback, rate limiting, panic switch, and sub-5ms saturation overhead).
- **Evaluation & Benchmarks (`bench/`):**
  - OS-FaultBench harness with responsiveness probe (wake-up + synchronous disk write latency).
  - Comprehensive evaluation suite (`bench/eval_suite.py`) and end-to-end interactive demo (`demo/run_demo.sh`).
