# Linux Control Group v2: Memory Controller

Source: Documentation/admin-guide/cgroup-v2.rst (Linux Kernel Documentation)
License: GPL-2.0-only / Creative Commons Attribution-ShareAlike 4.0

## memory.high Throttling Mechanism
chunk_id: kernel_cgroup_v2#memory-high
Section: Memory Interface Files
The `memory.high` interface is the primary high-watermark throttling mechanism in cgroup v2.
When a cgroup's memory usage surpasses `memory.high`, processes within that cgroup are throttled and subjected to proactive kernel page reclaim.
Unlike `memory.max`, exceeding `memory.high` never invokes the destructive Out-Of-Memory (OOM) killer. Instead, allocating processes are delayed proportionally to the excess allocation rate, returning CPU time to other cgroups and allowing dirty pages to flush to storage. Setting `memory.high` above the current working set prevents system-wide page-cache eviction and thrashing.

## memory.max Hard Limit and OOM Killer
chunk_id: kernel_cgroup_v2#memory-max
Section: Memory Interface Files
`memory.max` defines the absolute hard upper ceiling for memory consumption.
If a cgroup reaches `memory.max` and reclaim is unable to free sufficient pages, the kernel invokes the OOM killer (`mem_cgroup_out_of_memory`) to terminate one or more processes within the cgroup.
In systems managed by cgroup v2, setting proactive throttling via `memory.high` is strongly favored over relying on `memory.max` breaches.

## workingset_refault and Memory Thrashing
chunk_id: kernel_cgroup_v2#workingset-refault
Section: Memory Controller Events and Statistics
In `memory.stat`, `workingset_refault` measures the frequency at which recently evicted file-backed or anonymous pages are faulted back into RAM.
A steep surge in `workingset_refault` combined with high direct scan rates (`pgscan_direct`) is the defining signature of memory thrashing: the kernel is actively evicting pages that the application requires immediately, creating a vicious cycle of disk I/O stalls before the traditional OOM killer detects hard exhaustion.
