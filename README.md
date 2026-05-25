# cocapn-glue-core


## Meta

**Domain:** core-infrastructure
**Depends on:** —
**Depended by:** keeper
**Implements:** keeper-fleet-protocol, binary-wire
**Related:** keel, fleet-ecosystem


**The nervous system. One wire protocol across every tier — Cortex-M0 microcontroller to CUDA GPU.**

The Cocapn fleet spans a wide range of hardware: [Cortex-M0+](https://en.wikipedia.org/wiki/ARM_Cortex-M0) microcontrollers running bare metal, x86_64 servers, aarch64 edge devices, and CUDA GPUs. These tiers don't share an operating system, allocator, or even a common word size. What they share is this crate — a unified binary wire protocol that works on all of them.

---

## The Tiers

| Tier | Target | Memory | Heap |
|------|--------|--------|------|
| **Mini** | `thumbv6m-none-eabi` (Cortex-M0+) | 32 KB flash, 4 KB RAM | None (`heapless` only) |
| **Std** | `x86_64` / `aarch64` | Any | Full `std` |
| **Edge** | `aarch64` (Jetson) | Any | `std` + UUID networking |
| **Thor** | CUDA GPUs | Device memory | GPU UUID prefix |

When a Mini sensor reads a constraint value and needs to relay it to a Thor cluster, the same wire format works at both ends. The Mini encodes with no allocator. The Thor decodes with a GPU memory pool.

---

## Feature Flags

```bash
cargo add cocapn-glue-core                   # no_std, heapless only
cargo add cocapn-glue-core --features std    # Vec, Box, LRU cache
cargo add cocapn-glue-core --features async  # async transport (implies std)
cargo add cocapn-glue-core --features cuda   # CUDA capability flag
cargo add cocapn-glue-core --features plato  # PLATO sync (implies std)
```

## Wire Protocol

```rust
use cocapn_glue_core::wire::*;

let id = TierId::from_pid_timestamp(42, 1000);
let msg = WireMessage::Handshake(Handshake {
    sender: id,
    capabilities: 0b101, // Mini + Std + Thor
});
```

The protocol is length-delimited, not delimited by sentinel. No escaping needed. Every frame is validated before dispatch.

---

## How It Fits

The Cocapn fleet's nervous system:

| Layer | Crate | Purpose |
|-------|-------|---------|
| **Wire** | [cocapn-glue-core](https://github.com/SuperInstance/cocapn-glue-core) | Binary protocol across all tiers (this) |
| **Discovery** | [beacon-protocol](https://github.com/SuperInstance/beacon-protocol) | Fleet discovery and registry |
| **Memory** | [PLATO](https://github.com/SuperInstance/plato-server) | Persistent tile storage |
| **Messaging** | [bottle-protocol](https://github.com/SuperInstance/bottle-protocol) | Git-native agent communication |
| **Orchestration** | [cocapn](https://github.com/SuperInstance/cocapn) | Fleet-wide coordination |

---

## Related

- **[cocapn](https://github.com/SuperInstance/cocapn)** — Fleet-wide coordination
- **[cocapn-plato](https://github.com/SuperInstance/cocapn-plato)** — PLATO integration (SDK + server + query engine)
- **[plato-core](https://github.com/SuperInstance/plato-core)** — Foundation types and mesh registry
- **[plato-engine](https://github.com/SuperInstance/plato-engine)** — Rust PLATO engine
- **[beacon-protocol](https://github.com/SuperInstance/beacon-protocol)** — Fleet discovery and registry
- **[bottle-protocol](https://github.com/SuperInstance/bottle-protocol)** — Git-native agent communication

## License

MIT
