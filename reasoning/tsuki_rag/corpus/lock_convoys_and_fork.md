# Operating System Friction: Lock Convoys and Fork Bottlenecks

Source: Persistent Architectural Impediments in Modern Operating Systems (Snoxx Tech Research)
License: CC-BY-4.0

## The Fork-Exec Bottleneck in Large Address Spaces
chunk_id: os_friction#fork-bottleneck
Section: Process Abstractions
The legacy POSIX `fork()` primitive duplicates the entire virtual memory address space of the calling process.
In modern applications with multi-gigabyte heaps (e.g. Redis, JVM, in-memory caches), executing `fork()` forces the kernel to synchronously allocate and copy millions of page-table entries.
During this copy-on-write (COW) setup, all threads in the parent process stall, inducing latency spikes exceeding hundreds of milliseconds. When memory pressure is high, subsequent page writes trigger fault cascades, degrading system responsiveness.

## Lock Convoys and Priority Inversion Under CPU Saturation
chunk_id: os_friction#lock-convoys
Section: Scheduler Impediments
When background tasks saturate available CPU cores, the standard Linux scheduler may preempt a thread holding an in-process user-space spinlock or mutex.
All subsequent threads attempting to acquire the locked resource are forced into a convoy wait state.
Interactive applications freeze because their UI event loop is blocked waiting on an uncontended lock whose holder has been preempted by low-priority compute jobs. Dynamic reweighting of sibling cgroups (`cpu.weight`) resolves the convoy by accelerating the lock-holder's execution.
