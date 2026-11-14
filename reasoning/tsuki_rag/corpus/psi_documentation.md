# Linux Pressure Stall Information (PSI)

Source: Documentation/accounting/psi.rst (Linux Kernel Documentation)
License: GPL-2.0-only / CC-BY-SA-4.0

## Pressure Metrics: some vs full Semantics
chunk_id: kernel_psi#some-vs-full
Section: PSI Accounting Semantics
PSI quantifies the percentage of wall-clock time that tasks spend stalled waiting for CPU, memory, or I/O.
- `some`: Measures the percentage of time in which at least one non-idle task is stalled on the resource. For CPU, only `some` exists because CPU execution is never completely blocked for all tasks simultaneously.
- `full`: Measures the percentage of time in which ALL runnable tasks are simultaneously stalled on memory or block I/O.
When `full` memory or I/O pressure surges (e.g., `full avg10 > 10%`), no productive computation can proceed, leading directly to interactive system lockups and desktop freezes.

## PSI Moving Averages and Window Decay
chunk_id: kernel_psi#moving-averages
Section: Decay Windows
PSI provides exponential moving averages calculated across 10-second (`avg10`), 60-second (`avg60`), and 300-second (`avg300`) windows, alongside cumulative total stall time in microseconds (`total`).
The `avg10` metric responds rapidly to sudden resource saturation spikes, while a rising divergence between `avg10` and `avg60` indicates an accelerating degradation trend requiring immediate remediation.
