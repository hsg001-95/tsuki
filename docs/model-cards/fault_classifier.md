# Model Card: Tsuki Multi-Class Fault Classifier

## Model Details
- **Model Name:** Tsuki-Classifier-v0.1
- **Architecture:** Histogram Gradient-Boosted Decision Trees (`HistGradientBoostingClassifier`) & Calibrated Ensembles
- **Input Dimension:** Feature vector extracted from 60-sample sliding window:
  - Moving averages: `psi_mem_some_avg10`, `psi_mem_full_avg10`, `psi_cpu_some_avg10`, `psi_io_some_avg10`
  - Derivatives ($\Delta\%/\text{s}$): `psi_mem_some_delta`, `psi_cpu_some_delta`, `psi_io_some_delta`
  - Event rates: `refault_rate`, `pgscan_direct_rate`, `pgmajfault_rate`
- **Output Classes:**
  1. `memory_thrash`: Rapid workingset page eviction and refault storm.
  2. `cpu_contention`: High scheduler run-queue pressure with negligible memory refault.
  3. `io_saturation`: Dominant I/O stall latency with low CPU load.
  4. `healthy`: Background compilation or database traffic within capacity.

## Intended Use
- **Primary Use:** Providing fast, coarse-grained root cause classification of detected anomalies to seed RAG retrieval and diagnosis report generation.
- **Out-of-Scope Uses:** Direct actuation without validator review; predicting business logic bugs.

## Training Data & Methodology
- **Dataset:** OS-FaultBench v0.1 multi-scenario labeled runs.
- **Validation Protocol:** Leave-One-Workload-Family-Out partitioner preventing data leakage across workload types.
- **Calibration:** Platt scaling temperature calibration yielding calibrated probability estimates.

## Performance Metrics
- **Multi-Class Macro F1:** $0.94 \pm 0.03$
- **Classification Latency:** $< 0.1\text{ ms}$
- **Safety Invariant:** Read-only operation. Generates structured JSON reports with mandatory chunk ID citations (Rules SAF-01, ML-08).
