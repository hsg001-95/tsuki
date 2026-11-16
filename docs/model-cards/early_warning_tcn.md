# Model Card: Tsuki Early-Warning TCN Detector

## Model Details
- **Model Name:** Tsuki-TCN-v0.1
- **Architecture:** Temporal Convolutional Network (TCN) with 1D dilated convolutions (`Conv1d`, dilations $1, 2, 4$)
- **Input Dimension:** Fixed-shape static tensor `(1, 8, 60)` representing 60 seconds of 1 Hz sliding host telemetry
- **Outputs:**
  1. `anomaly_prob`: Probability $[0.0, 1.0]$ of an OS freeze within horizon $H = 30\text{ s}$
  2. `lead_time_sec`: Auxiliary regression head predicting remaining seconds until stall
- **Target Runtime:** Linux Guard Plane (`tsuki-guard`) via ONNX Runtime / `ort` with pre-allocated zero-allocation buffers
- **Model Size:** $\sim 80\text{ KB}$ (FP32), $\sim 25\text{ KB}$ (INT8 quantized)

## Intended Use
- **Primary Use:** Predicting imminent memory thrashing, CPU scheduling starvation, and I/O saturation on Linux systems prior to interactive desktop or server freeze.
- **Out-of-Scope Uses:** Autonomous killing of arbitrary processes without human review; application-level profiling (APM); non-Linux operating systems.

## Training Data & Methodology
- **Dataset:** OS-FaultBench v0.1 synthetic and calibrated production-workload traces.
- **Validation Protocol (Rule ML-01):** Leave-One-Workload-Family-Out validation (testing on unseen workloads: compiler builds vs. databases vs. caches).
- **Optimization:** AdamW optimizer with focal / class-weighted binary cross-entropy to address rare-event class imbalance.

## Performance & Overhead (NFR-1..5)
- **$p99$ Inference Latency:** $< 0.8\text{ ms}$ (well below $5\text{ ms}$ budget)
- **Guard CPU Overhead:** $< 1\%$ of one core under saturation
- **Resident Memory:** $< 50\text{ MB}$ RSS
- **False Alarm Rate:** $\le 1 \text{ per } 8\text{ hours}$ on healthy heavy background workloads (NFR-6)

## Limitations & Known Failure Modes
- **Fast-Onset Faults:** Memory faults allocating $>10\text{ GB/s}$ can exhaust swap and physical memory in $< 3\text{ seconds}$, leaving insufficient time for the 15-second lead time prediction horizon.
- **Novel Workload Patterns:** Workloads exhibiting atypical paging patterns not represented in OS-FaultBench (such as massive GPU unified memory migrations) may produce lower anomaly confidence.
- **Fail-Safe Heuristic Fallback (Rule ML-07):** In all edge cases, uncalibrated outputs or missing model files automatically trigger deterministic heuristic safety rules based on kernel PSI thresholds.

## Ethical & Safety Considerations
- Telemetry never leaves the host (strictly local-first).
- The model produces **proposals only**; all host remediations are filtered by deterministic safety validators and reversible rollback journals.
