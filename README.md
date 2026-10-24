# Tsuki (月)

> **Local-first Linux OS fault detection, diagnosis, and reversible remediation with cited evidence.**

Tsuki detects memory, CPU, and I/O saturation before machines freeze, diagnoses root causes using retrieved kernel documentation and telemetry windows, and executes validated, reversible remediations with zero unauthorized actions.

---

## Architecture Overview

Tsuki operates under a strict **Two-Plane Architecture** to prevent the observer effect from worsening host saturation:

- **Guard Plane (`tsuki-guard`, Rust):**
  - Always-on, native, memory-locked via `mlockall`.
  - Runs in an isolated cgroup slice with static CPU quota and negative OOM score.
  - Samples PSI, `/proc/vmstat`, and cgroup v2 metrics at 1–10 Hz.
  - Runs quantized early-warning models via ONNX Runtime without heap allocation in the hot path.
  - Emits immutable evidence snapshots to a bounded ring-buffer directory.

- **Actuation & Watchdog (`tsuki-actuator`, `tsuki-rollbackd`, Rust):**
  - Validates proposed remediation intents against an allow-list, parameter bounds, and protected target list.
  - Enforces the remediation state machine (`Proposed → Validated → Approved → Applied → Observing → Committed / RolledBack`).
  - Journals prior cgroup states with TTL; rolls back automatically on timeout, watchdog reset, or panic.

- **Reasoning Plane (`tsuki-agent`, `tsuki-api`, Python):**
  - On-demand, strictly memory-capped, expendable.
  - Read-only agent tools (no shell, no direct cgroupfs writes, no network egress).
  - Hybrid RAG (SQLite FTS5 + dense FAISS embeddings + cross-encoder reranking) over kernel/systemd docs.
  - Generates structured diagnosis reports with cited chunk IDs.

---

## Repository Structure

```
├── Cargo.toml                    # Virtual workspace
├── crates/
│   ├── tsuki-common/             # Shared types, config, error, telemetry, schemas
│   ├── tsuki-guard/              # Native guard daemon & telemetry collector
│   ├── tsuki-actuator/           # Safe actuator & state machine
│   └── tsuki-rollbackd/          # Rollback watchdog
├── reasoning/                    # Python reasoning plane (RAG, agent, API, dashboard)
├── bench/                        # OS-FaultBench harness & workload definitions
├── training/                     # Offline ML models (TCN, baselines, ONNX export)
├── docs/                         # ADRs, model cards, threat model
└── .github/workflows/            # Automated CI workflows
```

---

## Safety Invariants

- **Default Shadow Mode:** Tsuki defaults to observing and logging; no actions are applied unless explicitly configured.
- **Reversible Only:** Initial action catalog supports only `throttle_memory`, `reweight_cpu`, and `freeze_cgroup`. Autonomous killing of processes is strictly prohibited.
- **Panic Switch:** `tsuki panic` reverts all open actions and locks down actuation immediately.

---

## License

Apache-2.0
