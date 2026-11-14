# Linux Control Group v2: CPU Controller

Source: Documentation/admin-guide/cgroup-v2.rst (Linux Kernel Documentation)
License: GPL-2.0-only / CC-BY-SA-4.0

## cpu.weight Proportional Scheduling
chunk_id: kernel_cgroup_v2#cpu-weight
Section: CPU Interface Files
`cpu.weight` controls the proportional distribution of CPU bandwidth across sibling cgroups in the CFS/EEVDF scheduler hierarchy.
Valid weights span `[1, 10000]`, with a default of `100`.
When CPU contention arises, each runnable cgroup receives CPU time proportional to its weight relative to active siblings. Adjusting `cpu.weight` dynamically down (e.g. from 100 to 20) safely attenuates CPU saturation caused by runaway background tasks without terminating or freezing them.

## cpu.max Bandwidth Limiting
chunk_id: kernel_cgroup_v2#cpu-max
Section: CPU Interface Files
`cpu.max` sets a hard quota limit on total CPU time available to a cgroup within a fixed period (e.g., `50000 100000` limits execution to 50ms every 100ms, effectively clamping the group to 0.5 cores).
Unlike `cpu.weight`, which is work-conserving, `cpu.max` enforces hard throttling even if CPU cores are idle.
