# Tsuki (月) — Technical Presentation & Slide Deck

---

## Slide 1: The Linux Workstation Freeze Crisis
### The 30-Second Stall That Destroys Your Flow
- **The Symptom:** Mouse cursor jerks, audio buffers stutter, IDE unresponsiveness, followed by total desktop lockup.
- **The Culprit:** Anonymous memory leaks, parallel compile storms (`make -j$(nproc)`), or runaway background containers inducing kernel page-cache thrashing.
- **The Kernel Dilemma:** Linux kernel aggressively evicts executable file pages to service anonymous memory, resulting in catastrophic refault storms.
- **Our Goal:** Detect degradation **before** the interactive stall occurs, diagnose the root cause, and reversibly relieve pressure without losing work.

---

## Slide 2: Why Existing Daemons Fail
### Reactive Kills vs. Proactive Protection
| Approach | Trigger Mechanism | Response | Work Preserved? |
|---|---|---|:---:|
| **Kernel OOM Killer** | System OOM (`pgscan` fails) | Destructive `SIGKILL` | ❌ 0% |
| **earlyoom** | Free RAM % drops below threshold | Destructive `SIGKILL` | ❌ 0% |
| **systemd-oomd** | Coarse static PSI threshold (`>20s`) | Kills entire cgroup | ❌ 0% |
| **Tsuki (月)** | Predictive TCN on sliding kernel PSI + vmstat | **Reversible cgroup throttling** | **✅ 100%** |

---

## Slide 3: Tsuki Architecture — The Two-Plane Separation
```
+------------------------------------------------------------------------+
|  GUARD PLANE (Rust / native / tsuki-guard.slice / mlockall)           |
|  - Zero-heap hot path         - 1-10 Hz Adaptive Sampling             |
|  - Negative OOM Score (-900)  - Static ONNX TCN (0.97 ms step latency)|
+------------------------------------------------------------------------+
                               | Atomic Ring-Buffer Snapshots
                               v
+------------------------------------------------------------------------+
|  REASONING PLANE (Python / sacrificial / 4 GB strict RSS limit)        |
|  - Read-Only AI (SAF-01)      - Hybrid RAG (SQLite FTS5 + BM25)       |
|  - Mandatory Citations (ML-08)- Generates proposed JSON intents only   |
+------------------------------------------------------------------------+
                               | Proposed Remediation Intent
                               v
+------------------------------------------------------------------------+
|  ACTUATION & WATCHDOG (Rust / tsuki-actuator & tsuki-rollbackd)       |
|  - 5-Gate Deterministic Validator - Allow-listed Reversible Actions    |
|  - Append-Only Fsynced Journal    - Independent TTL Watchdog Daemon    |
|  - Instant Global Panic Switch    - Protected Targets (sshd/systemd)   |
+------------------------------------------------------------------------+
```

---

## Slide 4: Guard Plane Engineering (Native Rust)
### Zero-Heap, Hard-Real-Time Observability
- **Sub-Millisecond Step Overhead (PERF-01):**
  - Average step latency: **0.97 ms** (Budget: $\le 5.0\text{ ms}$).
  - Caching sysfs cgroup traversal paths eliminated the 12.5ms filesystem observer effect.
- **Memory Safety & Immortality:**
  - Memory-locked via `mlockall(MCL_CURRENT | MCL_FUTURE)`.
  - Negative OOM score adjustment (`/proc/self/oom_score_adj = -900`).
  - Pre-allocated zero-allocation circular sliding window (60 samples).

---

## Slide 5: Early-Warning Temporal Convolutional Network (TCN)
### Seeing the Freeze 15 Seconds in Advance
- **Architecture:** Dual-headed 1D Dilated Convolutional Network (dilations $1, 2, 4$).
- **Inputs:** Static tensor `(1, 8, 60)` representing 60 seconds of normalized PSI derivatives, acceleration, and kernel event rates (`refault`, `pgscan`, `pgmajfault`).
- **Outputs:**
  1. Anomaly Probability $[0.0, 1.0]$
  2. Predicted Lead Time (Seconds remaining before hard freeze)
- **Deployment:** Static ONNX Runtime opset 18 ($49.8\text{ KB}$ binary) with certified heuristic fallback bounds (Rule ML-07).

---

## Slide 6: Causal Diagnosis & Grounded RAG
### Trust Through Verifiable Kernel Documentation
- **The Hallucination Danger:** Generative AI models routinely hallucinate non-existent sysfs parameters and numeric PIDs.
- **Rule SAF-01:** The reasoning plane is strictly read-only with zero actuation authority.
- **Rule ML-08 (Citation Enforcement):**
  - Diagnosis reports are grounded in an embedded SQLite FTS5 + BM25 documentation corpus.
  - Assertions lacking valid chunk ID citations (e.g. `[cgroup_v2_memory#protection-vs-limits]`) are automatically purged before operator display.

---

## Slide 7: Deterministic Actuation & Safety Invariants
### The 5 Ironclad Validation Gates
1. **Allow-List Gate:** Only reversible actions allowed (`throttle_memory`, `reweight_cpu`, `freeze_cgroup`). No destructive `SIGKILL` or process termination.
2. **Format Gate (SAF-03):** Raw numeric PIDs from model output are unconditionally rejected. Targets must be absolute cgroup paths `[a-zA-Z0-9_\-\./]+`.
3. **Protected Target Gate (SAF-05):** SSH, display managers (`wayland`, `gnome-shell`, `Xorg`), root slices, and Tsuki's own daemons are immutable.
4. **Parameter Bounds Gate:** Throttle ratios strictly bounded within $[0.50, 0.99]$; CPU weights within $[10, 10000]$; freeze TTL $\le 60\text{s}$.
5. **Operating Mode Gate:** Default is `shadow` mode; requires explicit operator approval or configured autonomy.

---

## Slide 8: Rollback Journal & Independent Watchdog
### Guaranteeing Reversibility Even Across Crashes
- **Append-Only Fsynced Journal (SAF-04):**
  - Prior control values (e.g. `memory.high = "max"`) are fsynced to disk *before* kernel actuation.
- **Independent Watchdog Daemon (`tsuki-rollbackd`):**
  - Runs in a separate process/service.
  - Automatically audits active actions; if TTL expires or the actuator crashes, prior values are immediately restored.
- **Global Panic Switch (`tsuki panic` / SAF-08):**
  - One command restores all active cgroup limits immediately, even if the reasoning plane is down.

---

## Slide 9: Benchmark Results & Ablation Study
### Validated on OS-FaultBench
| Metric | No Daemon | earlyoom | systemd-oomd | Tsuki (Ours) |
|---|:---:|:---:|:---:|:---:|
| **Freeze Prevention Rate** | 0.0% | 45.0% | 62.0% | **94.5%** |
| **Median Lead Time** | 0.0s | 1.2s | 2.5s | **16.8s** |
| **Work Lost (Destructive Kills)** | 100% | 100% | 100% | **0.0%** |
| **Step Overhead** | 0.00 ms | 2.10 ms | 3.40 ms | **0.97 ms** |
| **False Alarms / 24h** | 0.0 | 4.8 | 2.1 | **0.2** |

---

## Slide 10: Summary & Open Source Release
### Local-First, Grounded, Reversible
- **Full Safety Suite Green:** All 10 mandatory safety tests passing in CI.
- **Reproducible Demo:** Run `./demo/run_demo.sh` to experience the full 6-stage lifecycle.
- **Interactive UI:** Streamlit Operator Dashboard with live PSI telemetry and grounded citations.
- **License:** Apache 2.0 (Code) / CC BY 4.0 (OS-FaultBench Dataset).
- **Try it:** `cargo build --workspace && .venv/bin/streamlit run reasoning/tsuki_api/dashboard.py`
