+++
kind = "post"
title = "Building a Tiny Cooperative Process Manager (TCPM) and Actor Model in C11"
description = "Exploring cooperative multitasking, bounded message queues, and lightweight actor models in modern C11 for embedded and high-throughput real-time systems."
excerpt = "Why preemptive OS threads can be overkill for deterministic real-time game simulation and trading loops. Designing a cooperative process manager in C11 with bounded ring-buffer queues and zero lock overhead."
tags = ["C11", "Systems", "Concurrency", "Actors"]
filter_tags = ["c99", "c11", "systems", "concurrency", "actors"]
reading_minutes = 5
date = 2023-08-20
+++

Preemptive kernel threads are versatile, but for micro-controllers, game simulation ticks, and real-time network pipelines, the overhead of OS context switching, mutex contention, and non-deterministic cache eviction can severely constrain throughput.

To address this in lightweight C environments, I authored [**tcpm**](https://github.com/eloraiby/tcpm): a compact cooperative process manager and bounded queue actor model in standard C11.

## 1. The Cooperative Invariant

In a cooperative model, tasks voluntarily yield control at known suspension points rather than being interrupted arbitrarily by a hardware timer. This single design invariant brings massive advantages:

- **No Race Conditions:** Shared state accessed during a task step is strictly thread-safe within that executor thread without locking primitives.
- **Zero Kernel Overhead:** Task switching is simply a function return and pointer swap.
- **Predictable Memory Footprints:** Stack sizes and task communication buffers can be statically provisioned.

## 2. Bounded Queues & Backpressure

In distributed systems and actor frameworks (such as Akka or Erlang), unbounded mailboxes are a notorious source of silent out-of-memory crashes when message ingestion outpaces consumer drain.

:::callout
**Bounded Design:** In `tcpm`, each process actor owns a fixed-capacity ring buffer. If a mailbox is full, send operations return an immediate backpressure status, forcing the producer to either back off, yield, or drop according to system policy.
:::

```
// Message dispatch in tcpm
typedef struct tcpm_msg_t {
    uint32_t sender_id;
    uint32_t msg_type;
    uint32_t payload_len;
    uint8_t  data[MAX_PAYLOAD_SIZE];
} tcpm_msg_t;

int tcpm_send(tcpm_system_t* sys, uint32_t target_pid, const tcpm_msg_t* msg) {
    tcpm_proc_t* target = &sys->procs[target_pid];
    if (ring_buffer_is_full(&target->mailbox)) {
        return TCPM_ERR_BACKPRESSURE;
    }
    ring_buffer_push(&target->mailbox, msg);
    return TCPM_OK;
}
```

## 3. Practical Applications

This pattern has proven effective in:

- **High-Frequency Trading & Real-Time Bidding:** Where microsecond determinism matters more than preemptive flexibility.
- **Embedded Firmware:** STM32 and bare-metal environments lacking POSIX threads.
- **Deterministic Game Simulation:** Guaranteeing reproducible network synchronization across peer clients.

## Resources

Explore the C11 source and benchmarks on GitHub: [eloraiby/tcpm](https://github.com/eloraiby/tcpm).
