# systemd Resource Control and Slice Management

Source: systemd.resource-control(5), systemd-oomd(8)
License: LGPL-2.1-or-later

## systemd Slice Hierarchies and Delegation
chunk_id: systemd#slice-hierarchy
Section: Slice Management
systemd partitions system processes into unified cgroup v2 slices:
- `system.slice`: Dedicated to system infrastructure services and daemons.
- `user.slice`: Contains user login sessions, desktop environments, and user processes.
Sub-slices inherit resource constraints unless explicitly configured. Delegated subtrees allow unprivileged user managers or sandboxed daemons to configure resource limits beneath their assigned slice root without requiring root access to parent hierarchy slices.

## systemd-oomd Thresholds and False Kills
chunk_id: systemd#oomd-limitations
Section: systemd-oomd Evaluation
`systemd-oomd` acts upon static, coarse-grained PSI thresholds (e.g., default `10%` memory pressure on `user@.service` over 20 seconds).
Because it evaluates pressure statically without understanding whether the stall is caused by a runaway background compilation or a harmless transient batch burst, it frequently selects and kills critical interactive applications (e.g., IDEs or desktop sessions) prematurely. Tsuki addresses this limitation by diagnosing the exact root cause and applying reversible `memory.high` throttling rather than destructive process termination.
