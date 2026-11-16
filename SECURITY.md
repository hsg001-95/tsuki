# Tsuki Security Policy and Threat Model

## 1. Core Safety Invariants

Tsuki is designed with a defense-in-depth security architecture that guarantees zero unauthorized or unrecoverable actions on Linux hosts.

1. **SAF-01: Read-Only AI**  
   The reasoning plane (LLM agent) has ZERO direct write access to the host, no shell execution authority, no direct `/sys/fs/cgroup` write capability, and no network egress by default. It can only propose structured JSON intents.
2. **SAF-02: Single Gate to Action**  
   The deterministic Rust actuator validator and state machine are the ONLY path from an intent proposal to host actuation.
3. **SAF-03: No PIDs from Model Output**  
   Targets must be identified by strict cgroup paths (`[a-zA-Z0-9_\-\./@+]+`), never by raw numeric PIDs from model text. Raw numeric PIDs are unconditionally rejected.
4. **SAF-04: Reversible Actions & Mandatory Fsynced Rollback Journal**  
   Every action in v1 must be reversible (`memory.high`, `cpu.weight`, `cgroup.freeze`), carry a bounded TTL, and be fsynced in an append-only journal *before* kernel actuation.
5. **SAF-05: Protected Target List**  
   Display servers (`wayland`, `gnome-shell`, `Xorg`), session managers, `sshd`, systemd infrastructure, root slices, and Tsuki daemons are immutable and can NEVER be altered.
6. **SAF-06: Shadow Mode Default**  
   Tsuki defaults to Shadow Mode (log only). Applying actions requires explicit operator approval or configured autonomy.
7. **SAF-07: Rate Limiting & Target Cooldown**  
   A mandatory 60-second cooldown is enforced per target cgroup to prevent actuation oscillation.
8. **SAF-08: Global Panic Switch**  
   Running `tsuki-actuator panic` or clicking the panic button immediately reverts all active actions to baseline, even if the reasoning plane is down or crashed.

---

## 2. Threat Model Summary

| Threat Vector | Potential Impact | Tsuki Mitigation |
|---|---|---|
| **Prompt Injection via System Logs** | Attacker crafts log lines trying to force `rm -rf /` or PID killing | Validator strictly enforces cgroup regex allow-list; rejects shell metacharacters and numeric PIDs (Test 3) |
| **Reasoning Plane Compromise / OOM** | Sacrificial Python process crashes or hallucinates bad actions | Guard plane runs isolated under `mlockall` with negative OOM score (-900); continues sampling uninterrupted (Test 6) |
| **Observer Effect (Denial of Service)** | Telemetry daemon consumes too much CPU or locks sysfs | Guard step latency is capped at $< 1.0\text{ ms}$; sysfs cgroup tree is cached every 10s (Test 10) |
| **Actuator Daemon Crash Mid-Action** | Host left in throttled state indefinitely | Independent watchdog daemon (`tsuki-rollbackd`) monitors fsynced journal and rolls back expired actions (Test 5) |
| **Privilege Escalation** | Compromise of agent escaping to root | Systemd sandboxing drops all capabilities, enforces `NoNewPrivileges=yes`, and strictly confines execution slices |

---

## 3. Reporting a Vulnerability

If you discover a security vulnerability or safety bypass in Tsuki, please report it via private disclosure:
- **Email:** `security@snoxx.tech`
- **Response SLA:** Initial acknowledgment within 48 hours; status updates every 5 business days until resolution.
- **Coordination:** Please do not open public GitHub issues or disclose vulnerabilities publicly until a patch has been released.
