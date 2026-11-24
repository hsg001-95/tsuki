# What Tsuki Never Does Automatically (Invariants & Safety Bounds)

This document establishes the binding non-negotiable boundaries of Tsuki. Trust in autonomous systems requires absolute clarity on what the system **will never do**, regardless of model inference, pressure severity, or operator absence.

---

### 1. What Tsuki Never Does

1. **Never Runs Actuation by Default (Default to Shadow Mode)**:
   - Fresh installations of Tsuki boot strictly in `mode = "shadow"`. In this mode, telemetry and diagnosis run continuously, but all actuation writes to `/sys/fs/cgroup` are hard-disabled at the state machine level.
   - Moving to `approval` or `autonomous` mode requires an explicit human operator configuration change.

2. **Never Touches Protected System Cgroups or Core Daemons (Rule SAF-02)**:
   - Tsuki rejects any candidate intent targeting system infrastructure:
     - `/system.slice/sshd.service` (Remote SSH management)
     - `/system.slice/systemd-journald.service` (System logging)
     - `/system.slice/dbus.service` (Inter-process communication)
     - `/system.slice/tsuki-*.service` (Guard Plane and Actuator)
     - `/user.slice` (Interactive operator sessions)
   - These targets are pre-filled in `/etc/tsuki/tsuki.toml` under `[protected.cgroups]` and validated with fail-closed kernel checks.

3. **Never Issues Unbounded or Destructive Cgroup Throttles (Rule SAF-03)**:
   - Tsuki will never depress a cgroup's `memory.high` below $50\%$ of its observed working set or lower than $128\,\text{MB}$.
   - Tsuki will never issue `cgroup.kill` or `cgroup.freeze` commands automatically. It only manipulates reversible memory pressure boundaries.

4. **Never Applies Mutations Without an Enforced Rollback Lease (Rule SAF-04)**:
   - Every single cgroup mutation carries an absolute Time-To-Live lease ($\le 300\,\text{seconds}$).
   - The independent `tsuki-rollbackd` watchdog monitors `/var/lib/tsuki/rollback.journal`. If a lease expires, the daemon crashes, or the reasoning plane fails, the watchdog automatically restores the cgroup's baseline limits.

5. **Never Uploads Telemetry, Dumps, or Kernel State to External Servers**:
   - Tsuki is **100% local-first**. Telemetry, pressure samples, diagnosis reports, and audit logs stay entirely on the local host.
   - No remote analytics, no phone-home pings, no telemetry telemetry ingestion endpoints.
   - If optional crash reporting is enabled in the future, all process names, paths, and environment variables must be cryptographically scrubbed prior to export.

6. **Never Overrides or Bypasses the Global Panic Switch (Rule SAF-08)**:
   - When `tsukictl panic`, the Web Cockpit panic button, or the `/var/run/tsuki/actuator.sock` panic command is triggered, all active cgroup modifications are reverted to baseline within $<10\,\text{ms}$, and the actuation state machine is latched closed.
   - The latch can only be cleared by an authenticated operator.

7. **Never Automatically Updates Binaries or Models in Production**:
   - Model updates (ONNX temporal models, RAG database indices) and daemon binaries are versioned, signed packages. Tsuki will never auto-pull or hot-patch models into live execution without staged human promotion.

8. **Never Starves Kernel Memory or Host Slices (Rule PERF-04)**:
   - `tsuki-guard` is memory-locked via `mlockall()` to a fixed $42\,\text{MB}$ RSS inside `tsuki-guard.slice` (`OOMScoreAdjust=-900`).
   - The Reasoning Plane runs in `tsuki-reasoning.slice` (`OOMScoreAdjust=500`). Under extreme OOM conditions, the Python reasoning engine is sacrificed first by the kernel while the Guard Plane and host remain unharmed.
