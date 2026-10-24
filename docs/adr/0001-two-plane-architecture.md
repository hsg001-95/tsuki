# ADR 0001: Two-Plane Architecture for Resource Saturation Safety

- **Status:** Accepted
- **Date:** 2026-10-24
- **Author:** @Ghoy / Snoxx Tech

## Context and Problem Statement
Linux systems experience freeze states under heavy memory and CPU pressure. Running complex diagnostics, RAG, and LLM inference directly on a saturated system can exacerbate the saturation (the "Observer Effect").

## Decision Drivers
- PERF-01: Guard plane must run in isolated, memory-locked cgroup with static CPU quota.
- PERF-04: Reasoning plane must be memory-capped, expendable, and the guard plane must never block on it.
- SAF-01: LLM agent must not have write paths to the host.

## Decision Outcome
Split the system into two distinct runtime planes:
1. **Guard Plane (Rust):** High-reliability, native, memory-locked via `mlockall`, static CPU quota, negative OOM score adjustment. Collects telemetry, evaluates quantized TCN detector via ONNX Runtime, writes immutable snapshots to ring buffer, and deterministically executes/rolls back approved actions.
2. **Reasoning Plane (Python):** Expendable, runs on-demand with a strict `memory.max`. Runs RAG, LLM agent, and dashboard. Only communicates with Guard Plane via read-only snapshot ingestion and intent proposal sockets.

### Consequences
- Positive: Guaranteed monitoring survival during severe system thrash.
- Positive: Safe failure mode (guard degrades to monitoring only if reasoning plane is OOM-killed).
- Negative: Requires inter-plane IPC (read-only snapshots and Unix domain socket for intent proposals).
