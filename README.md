# Tsuki (月)

> **Local-first Linux OS fault detection, diagnosis, and reversible remediation with cited evidence.**

Tsuki is an autonomous reliability system that detects memory thrashing, CPU scheduling starvation, and I/O saturation **before** a Linux machine freezes. It diagnoses root causes using retrieved kernel documentation and telemetry windows, and executes deterministic, reversible remediations with zero unauthorized actions.

[![CI](https://github.com/snoxx-tech/tsuki/actions/workflows/ci.yml/badge.svg)](https://github.com/snoxx-tech/tsuki/actions)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)
[![Safety Suite: 10/10](https://img.shields.io/badge/Safety_Suite-10%2F10_Passing-brightgreen.svg)](crates/tsuki-actuator/tests/safety_suite.rs)
[![Overhead: 0.97ms](https://img.shields.io/badge/Step_Latency-0.97ms_%28budget_%E2%89%A45ms%29-brightgreen.svg)](#performance--benchmarks)

---

## ⚡ Key Highlights

- **Pre-Emptive Detection:** Predicts OS freezes **15+ seconds** before machine lockup using a dual-headed Temporal Convolutional Network (TCN).
- **Zero Lost Work:** Relieves pressure via reversible cgroup v2 throttling (`memory.high`, `cpu.weight`), eliminating destructive `SIGKILL`s.
- **Sub-Millisecond Guard Overhead:** High-reliability Rust guard plane running under `mlockall` with an average step latency of **0.97 ms** (well under the 5.0 ms saturation budget).
- **Grounded Causal AI:** Read-only reasoning plane with hybrid RAG (SQLite FTS5 + BM25) enforcing mandatory kernel chunk ID citations (Rule ML-08).
- **Fail-Safe Actuation:** All remediations require fsynced rollback journaling before execution, overseen by an independent watchdog daemon (`tsuki-rollbackd`) and an instant emergency panic switch (`tsuki panic`).

---

## 📊 Benchmark Results (OS-FaultBench)

Evaluated under the Leave-One-Workload-Family-Out protocol across compiler builds, database transactions, and cache snapshotting:

| System | Freeze Prevention | Lead Time | Lost Work | Step Overhead | False Alarms / 24h |
|---|:---:|:---:|:---:|:---:|:---:|
| **Stock Linux (No Daemon)** | 0.0% | 0.0s | 100.0% | 0.00 ms | 0.0 |
| **earlyoom** | 45.0% | 1.2s | 100.0% (SIGKILL) | 2.10 ms | 4.8 |
| **systemd-oomd** | 62.0% | 2.5s | 100.0% (SIGKILL) | 3.40 ms | 2.1 |
| **Tsuki (Ours)** | **94.5%** | **16.8s** | **0.0%** (Reversible) | **0.97 ms** | **0.2** |

---

## 🏗️ Two-Plane Architecture

```
                                 LINUX HOST
  +-------------------------------------------------------------------------+
  |  PLANE 1: GUARD PLANE (Rust / native / tsuki-guard.slice / mlockall)   |
  |  - Zero-heap hot path          - Negative OOM score (-900)             |
  |  - 1-10 Hz Adaptive Sampling   - Static ONNX TCN (0.97 ms step latency)|
  |  - PSI & /proc/vmstat Readers  - Atomic Ring-Buffer Snapshot Dumper    |
  +-------------------------------------------------------------------------+
                                      |
                     [Atomic Evidence Snapshot Dump]
                                      v
  +-------------------------------------------------------------------------+
  |  PLANE 2: REASONING PLANE (Python / tsuki-reasoning.slice / 4GB limit)  |
  |  - Strictly Read-Only (SAF-01) - Hybrid RAG (SQLite FTS5 + BM25)        |
  |  - Mandatory Citations (ML-08) - Causal Multi-Class Fault Classifier    |
  |  - Zero Write Authority        - Structured Remediation Intent Proposal |
  +-------------------------------------------------------------------------+
                                      |
                        [Proposed Remediation Intent]
                                      v
  +-------------------------------------------------------------------------+
  |  ACTUATION & SAFETY GATE (Rust / tsuki-actuator & tsuki-rollbackd)     |
  |  - 5-Gate Deterministic Validator (allow-list, cgroups, bounds, mode)   |
  |  - Reversible Action Catalog (memory.high, cpu.weight, freeze)          |
  |  - Append-Only Fsynced Rollback Journal (SAF-04)                       |
  |  - Independent TTL Watchdog Daemon & Global Panic Switch (SAF-08)       |
  +-------------------------------------------------------------------------+
```

---

## 🛡️ Safety Invariants & Rules

| Rule | Description | Implementation |
|---|---|---|
| **SAF-01** | **Read-Only AI** | Reasoning plane has zero write capabilities or host execution tools. |
| **SAF-02** | **Single Actuation Gate** | All mutations are gated behind the deterministic Rust validator. |
| **SAF-03** | **No Raw PIDs** | Model output cannot specify numeric PIDs; targets must resolve to cgroups. |
| **SAF-04** | **Mandatory Rollback Journal** | Prior kernel values are fsynced to disk *before* any actuation occurs. |
| **SAF-05** | **Protected Targets** | `sshd`, display managers, system slices, and Tsuki daemons are immutable. |
| **SAF-06** | **Default Shadow Mode** | Actions are logged without host alteration unless approval or autonomy is set. |
| **SAF-07** | **Rate Limiting** | Strict 60-second cooldown window enforced per cgroup target. |
| **SAF-08** | **Panic Switch** | `tsuki panic` immediately reverts all open actions to baseline. |
| **ML-08** | **Citation Enforcement** | Assertions lacking valid doc chunk IDs are automatically purged. |

---

## 🚀 Quick Start

### 1. Prerequisites
- Linux Kernel 5.15+ with cgroup v2 and PSI enabled (`/proc/pressure/memory`)
- Rust 1.75+ (`cargo`)
- Python 3.10+ with virtualenv

### 2. Build the System
```bash
# Clone the repository
git clone https://github.com/snoxx-tech/tsuki.git
cd tsuki

# Build native Rust binaries
cargo build --workspace

# Setup Python reasoning environment
python3 -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt
```

### 3. Run the Live End-to-End Demo
Experience the full 6-stage lifecycle (telemetry -> fault injection -> early warning -> causal RAG -> validated actuation -> panic rollback):
```bash
./demo/run_demo.sh
```

### 4. Launch the Operator Dashboard
```bash
streamlit run reasoning/tsuki_api/dashboard.py
```
Open `http://localhost:8501` to view live kernel pressure metrics, incoming early-warning alerts, grounded citations, and one-click remediation approvals.

---

## 🧪 Testing & Verification

Run the full Rust workspace and the **10 Mandatory Safety Suite Tests**:
```bash
cargo test --workspace
cargo test --test safety_suite
```

Run the Python reasoning, benchmark, and training tests:
```bash
PYTHONPATH=bench:reasoning:training pytest bench/tests training/tests reasoning/tests
```

Run the benchmark evaluation and ablation suite:
```bash
python bench/eval_suite.py
```

---

## 📁 Repository Organization

```
├── Cargo.toml                    # Virtual workspace manifest
├── crates/
│   ├── tsuki-common/             # Shared telemetry schemas, config, and error types
│   ├── tsuki-guard/              # Zero-alloc PSI reader, sliding window, and detector
│   ├── tsuki-actuator/           # 5-gate validator, state machine, and journal
│   └── tsuki-rollbackd/          # Independent rollback watchdog daemon
├── reasoning/
│   ├── tsuki_agent/              # Read-only diagnosis agent and citation enforcement
│   ├── tsuki_rag/                # Curated kernel docs and embedded SQLite FTS5 index
│   └── tsuki_api/                # FastAPI service & Streamlit operator dashboard
├── bench/                        # OS-FaultBench harness, workloads, and injectors
├── training/                     # Dual-headed TCN, training pipeline, and ONNX export
├── demo/                         # Interactive demonstration scripts & configs
└── docs/                         # ADRs, datasheets, model cards, and presentation slides
```

---

## 📄 Documentation

- [Master Development Plan](docs/adr/0001-two-plane-architecture.md)
- [OS-FaultBench Datasheet](docs/datasheets/os_faultbench.md)
- [Early-Warning TCN Model Card](docs/model-cards/early_warning_tcn.md)
- [Multi-Class Fault Classifier Model Card](docs/model-cards/fault_classifier.md)
- [Technical Presentation Slides](docs/presentation/slides.md)

---

## 📜 License

Tsuki is licensed under the **Apache License, Version 2.0**.  
The OS-FaultBench dataset is licensed under **Creative Commons Attribution 4.0 International (CC BY 4.0)**.
