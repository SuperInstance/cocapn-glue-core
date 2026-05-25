# cocapn-glue-core

Cross-tier binary wire protocol for the Cocapn fleet. One format from Cortex-M0+ microcontrollers to CUDA GPUs.

Dual implementation: **Rust crate** (`#![no_std]` + Postcard serialization) for embedded/tier communication, and **Python module** (msgpack-based) for keeper↔fleet communication.

---

## The Tiers

| Tier | Target | Memory | Heap |
|------|--------|--------|------|
| **Mini** | `thumbv6m-none-eabi` (Cortex-M0+) | 32 KB flash, 4 KB RAM | None (`heapless` only) |
| **Std** | `x86_64` / `aarch64` | Any | Full `std` |
| **Edge** | `aarch64` (Jetson) | Any | `std` + UUID networking |
| **Thor** | CUDA GPUs | Device memory | GPU UUID prefix |

---

## Rust Crate

### Install

```bash
cargo add cocapn-glue-core                    # no_std, heapless only
cargo add cocapn-glue-core --features std     # Vec, Box, LRU cache
cargo add cocapn-glue-core --features async   # async transport (implies std)
```

### Wire Message Envelope

Every message on the wire is one of these variants:

```rust
use cocapn_glue_core::wire::*;

// TierId — 8-byte identifier, constructed per tier
let id = TierId::from_pid_timestamp(42, 1000);
// TierId(2a000000e8030000)

let id_from_mac = TierId::from_mac(&[0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);
// Uses MAC + XOR checksum for last 2 bytes

let id_from_uuid = TierId::from_uuid_prefix(&[0u8; 16]);
// First 8 bytes of a UUID

// Handshake — exchanged on connection
let msg = WireMessage::Handshake(Handshake {
    sender: id,
    capabilities: 0b101,     // Mini + Std + Thor
    protocol_version: 1,
});

// Data chunk — payload with sequencing for reassembly
let chunk = WireMessage::DataChunk(DataChunk {
    sender: id,
    chunk_id: 1,
    total_chunks: 3,
    sequence: 0,
    payload: vec![0xDE, 0xAD, 0xBE, 0xEF],
});

// Acknowledgement
let ack = WireMessage::Ack(Ack {
    receiver: id,
    chunk_id: 1,
    sequence: 0,
});

// Wire-level error
let err = WireMessage::Error(WireError::BufferOverflow);
```

### Serialization (Postcard)

```rust
use cocapn_glue_core::wire::{serialize_message, deserialize_message, WireMessage};

let msg = WireMessage::Handshake(Handshake {
    sender: TierId::from_pid_timestamp(42, 1000),
    capabilities: 0b101,
    protocol_version: 1,
});

// Encode to bytes
let bytes: Vec<u8> = serialize_message(&msg).unwrap();

// Decode back
let decoded: WireMessage = deserialize_message(&bytes).unwrap();
assert_eq!(msg, decoded);
```

### TierId Broadcast

```rust
use cocapn_glue_core::wire::TierId;

let broadcast = TierId::BROADCAST;  // [0xFF; 8] — send to all tiers
let zero = TierId::ZERO;            // [0x00; 8] — null/unset
```

### Wire Diagram

```
Frame format (length-delimited, no sentinel bytes):

┌──────────────────┬──────────────────────────────┐
│  4 bytes (BE)    │  N bytes                     │
│  payload length  │  Postcard-serialized message  │
└──────────────────┴──────────────────────────────┘

Message types (Postcard enum, first byte):

  0x00  Handshake { sender: TierId, capabilities: u32, protocol_version: u16 }
  0x01  DataChunk  { sender: TierId, chunk_id: u64, total_chunks: u32, sequence: u32, payload: Vec<u8> }
  0x02  Ack        { receiver: TierId, chunk_id: u64, sequence: u32 }
  0x03  Error      { enum: UnknownTier | MalformedMessage | BufferOverflow | UnsupportedVersion(u16) | TransportError(u32) }

TierId: 8 bytes fixed — no heap allocation for the ID itself.
```

### Discovery (Beacons + Capabilities)

```rust
use cocapn_glue_core::discovery::*;

// Build a capability bitmask
let mut caps = Capabilities::none();
caps.set(Capability::NoStd);
caps.set(Capability::Cuda);
caps.has(Capability::NoStd);   // true
caps.has(Capability::Async);   // false

// Create a beacon for fleet discovery
let beacon = Beacon::new(
    TierId::from_pid_timestamp(1, 100),
    caps,
    1,  // protocol version
    1716480000,  // timestamp
);

// Discovered peer representation
let peer = DiscoveredPeer::new(beacon.sender, beacon.capabilities, beacon.protocol_version);
peer.has_capability(Capability::Cuda);  // true
```

### Provenance (Merkle Tree over Verification Traces)

```rust
use cocapn_glue_core::provenance::*;

// Record constraint verification
let trace = VerificationTrace::new(
    TierId::from_pid_timestamp(42, 1000),
    1,                                    // trace_id
    vec![0xAB; 32],                       // constraint hash
    0,                                    // result: 0 = pass
    1716480000,                           // timestamp
);
trace.is_pass();  // true
let hash: [u8; 32] = trace.hash();       // SHA-256 of the trace

// Build a Merkle tree from multiple traces
let traces = vec![trace1, trace2, trace3];
let tree = MerkleTree::from_traces(&traces);
let root: &[u8; 32] = tree.root();       // Merkle root hash
tree.len();                               // 3 leaves
```

### PLATO Sync (Generation-Based Delta)

```rust
use cocapn_glue_core::plato::*;

let gen = SyncGeneration(1);
let next = gen.next();                    // SyncGeneration(2)
next.is_newer_than(gen);                  // true

// Full snapshot
let snapshot = PlatoSyncPayload::Snapshot {
    room_id: vec![1, 2, 3],
    generation: SyncGeneration(5),
    data: vec![0xAA; 128],
};

// Incremental delta
let delta = PlatoSyncPayload::Delta {
    room_id: vec![1, 2, 3],
    from_gen: SyncGeneration(4),
    to_gen: SyncGeneration(5),
    patch: vec![0x00; 32],  // binary diff
};

// Invalidation notice
let invalidate = PlatoSyncPayload::Invalidate {
    room_id: vec![1, 2, 3],
    generation: SyncGeneration(5),
};
```

### Tile Cache (LRU, `std` feature only)

```rust
use cocapn_glue_core::plato::{TileCache, SyncGeneration};

let mut cache = TileCache::new(100);  // max 100 entries

cache.insert(vec![1], SyncGeneration(1), vec![10]);
cache.insert(vec![2], SyncGeneration(2), vec![20]);

let entry = cache.get(&[1]);    // Some(CacheEntry { generation: 1, data: [10] })
cache.invalidate(&[2]);         // Remove entry
cache.len();                     // 1
```

### Configuration (`std` feature)

Environment variables with `GLUE_` prefix:

```bash
export GLUE_TIER_ID=0102030405060708         # 8 bytes hex
export GLUE_PROTOCOL_VERSION=1
export GLUE_MAX_MESSAGE_SIZE=65536
export GLUE_PLATO_SYNC_INTERVAL_MS=5000
export GLUE_BEACON_INTERVAL_MS=1000
```

```rust
#[cfg(feature = "std")]
use cocapn_glue_core::config::Config;

let config = Config::from_env();
```

---

## Python Module

For keeper↔fleet communication using msgpack framing.

### Wire Format (Python)

```
Frame: [4-byte length (big-endian)][msgpack payload]

Message types:
  0x01 HEARTBEAT    — keep-alive + stats
  0x02 STATUS       — full agent/vessel status
  0x03 COMMAND      — directive from keeper to agent
  0x04 RESPONSE     — result from agent to keeper
  0x05 TILE         — PLATO tile forwarded through glue
  0x06 ALERT        — elevated attention signal
  0x07 REGISTER     — agent joining the fleet
  0x08 DEREGISTER   — agent leaving
```

### Keeper Setup

```python
from glue_core import GlueProtocol, MessageType

keeper = GlueProtocol(role="keeper")

# Register message handlers
def on_tile(msg, sock):
    print(f"Tile from {msg.sender}: {msg.payload['question']}")
    # Forward to PLATO...

def on_register(msg, sock):
    print(f"Agent joined: {msg.payload['agent']}")

keeper.on(MessageType.TILE, on_tile)
keeper.on(MessageType.REGISTER, on_register)

keeper.listen(port=8901)
```

### Agent Setup

```python
from glue_core import GlueProtocol, MessageType

agent = GlueProtocol(role="agent", agent_name="ccc")
sock = agent.connect("keeper.fleet.internal", 8901)

# Send heartbeat
agent.send_heartbeat(sock)

# Send a PLATO tile through the glue protocol
agent.send_tile(sock, "Fleet status?", "All systems nominal", room="fleet_ops")

# Get accumulated stats
print(agent.get_stats())
# {'messages_sent': 2, 'messages_received': 0, 'bytes_sent': 156, ...}
```

### Custom Message Handlers

```python
def on_command(msg, sock):
    cmd = msg.payload
    if cmd["command"] == "check_health":
        result = run_health_check()
        agent.send_response(sock, cmd["id"], result)

agent.on(MessageType.COMMAND, on_command)
```

---

## Feature Flags

| Flag | Implies | What it enables |
|------|---------|-----------------|
| (default) | — | `no_std`, `heapless`, Postcard |
| `std` | — | `Config::from_env()`, `TileCache`, heap types |
| `async` | `std` | `AsyncTransport` trait |
| `cuda` | — | CUDA capability flag in discovery |
| `plato` | `std` | PLATO sync payloads |

---

## Architecture

```
cocapn-glue-core/
├── src/                          # Rust crate
│   ├── lib.rs                    # Crate root (#![no_std])
│   ├── config.rs                 # Env-based config (std)
│   ├── wire/
│   │   ├── addr.rs               # TierId (8-byte identifier)
│   │   ├── message.rs            # WireMessage enum
│   │   ├── transport.rs          # Transport + AsyncTransport traits
│   │   └── serde.rs              # Postcard serialize/deserialize
│   ├── discovery/
│   │   ├── capabilities.rs       # Capability bitmask
│   │   ├── beacon.rs             # Beacon broadcast + Discovery trait
│   │   └── peer.rs               # DiscoveredPeer
│   ├── provenance/
│   │   ├── trace.rs              # VerificationTrace
│   │   └── merkle.rs             # MerkleTree (SHA-256)
│   └── plato/
│       ├── sync.rs               # SyncGeneration + PlatoSyncPayload
│       ├── cache.rs              # LRU TileCache (std)
│       └── invalidation.rs       # Cache invalidation
├── glue_core.py                  # Python msgpack wire protocol
├── __main__.py                   # Python entry point
├── test_integration.py           # Python integration test
└── tests/
    └── glue_test.rs              # Rust tests
```

---

## Tests

```bash
# Rust
cargo test
# 10 tests: TierId construction, serialization roundtrips, Merkle tree, capabilities, cache

# Python (requires msgpack)
pip install msgpack
python test_integration.py
```

---

## Related

| Repo | What |
|------|------|
| [cocapn](https://github.com/SuperInstance/cocapn) | Fleet-wide coordination |
| [cocapn-plato](https://github.com/SuperInstance/cocapn-plato) | PLATO engine + SDK |
| [plato-core](https://github.com/SuperInstance/plato-core) | Foundation types and mesh registry |
| [beacon-protocol](https://github.com/SuperInstance/beacon-protocol) | Fleet discovery and registry |

## License

MIT
