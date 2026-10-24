# Tsuki Security Policy and Threat Model Invariants

## Core Safety Invariants

1. **SAF-01: Read-Only AI**
   The reasoning plane (LLM agent) has ZERO direct write access to the host, no shell execution, no direct cgroupfs writes, and no network egress by default.
2. **SAF-02: Single Gate to Action**
   The deterministic actuator validator and state machine are the ONLY path from intent proposal to host actuation.
3. **SAF-03: No PIDs from Model Output**
   Targets must be identified by cgroup ID or pidfds resolved directly by the actuator, never by PID/name from model text.
4. **SAF-04: Reversible Actions & Mandatory Rollback**
   Every action in v1 must be reversible (`memory.high`, `cpu.weight`, `cgroup.freeze`), carry a TTL, and be fsynced in an append-only journal before application.
5. **SAF-05: Protected Target List**
   Display servers, session managers, sshd, databases, essential system services, and Tsuki itself can NEVER be altered.
6. **SAF-06: Shadow Mode Default**
   Tsuki defaults to Shadow Mode (log only). Approval or limited autonomous mode requires explicit configuration.
7. **SAF-08: Panic Switch**
   Running `tsuki panic` reverts all active actions and disables actuation immediately, functioning even if the reasoning plane is unresponsive.

## Reporting a Vulnerability

If you discover a security vulnerability or safety bypass in Tsuki, please report it via private disclosure. Do not file public issues for active safety bypasses.
