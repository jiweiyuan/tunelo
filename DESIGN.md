# Tunelo — Design Document

> **Status**: MVP working end-to-end. 1,165 lines of Rust. 9 source files.

## 0. Performance Results (measured)

| Metric | Value |
|--------|-------|
| Gateway RSS after 5000 req stress | **8 MB** |
| Client RSS | **8 MB** |
| Binary size (gateway) | **3.5 MB** (stripped, LTO) |
| Binary size (client) | **3.3 MB** (stripped, LTO) |
| Tunnel overhead vs direct | **~14%** (0.56s vs 0.49s / 100 req) |
| Sequential latency | **~6ms/req** (localhost, including Python backend) |
| Throughput (20 concurrent) | **670 req/s** |
| Throughput (50 concurrent) | **685 req/s** |
| Throughput (200 concurrent) | **672 req/s** |
| Errors under stress (5000 req) | **0** |
| Large file (136KB) relay | byte-perfect, zero-copy |

### Architecture (v2 — zero-copy)

The data plane does **ZERO HTTP parsing** on the tunnel path:
1. Gateway peeks at TCP socket (non-consuming) to read Host header
2. Opens a QUIC bidi stream to the client
3. `tokio::io::copy_bidirectional` between TCP socket and QUIC stream
4. Client `copy_bidirectional` between QUIC stream and localhost TCP

No serialization, no buffering, no protocol messages on data streams.
Only the control stream uses msgpack framing (registration + heartbeats).

---

## 1. Prior Art Review

### 1.1 bore (ekzhang/bore)

**What it does well:**
- Minimal code (~500 lines total across 5 files). Entire project is client.rs, server.rs, shared.rs, auth.rs, main.rs
- Beautiful CLI UX: `bore local 3000 --to bore.pub`
- Control channel + data channel separation is clean: one persistent TCP connection for control messages, new TCP connections per proxied request
- Uses JSON-over-null-delimited-frames for the control protocol (simple, debuggable)
- UUID-based connection correlation: server assigns UUID to each incoming visitor connection, tells client to "Accept(uuid)", client opens a new connection referencing that UUID, then bidirectional copy begins
- `tokio::io::copy_bidirectional` for the data path — zero-copy where the OS supports it

**What Tunelo should borrow:**
- The control/data channel split architecture
- UUID-based connection-to-stream correlation
- `copy_bidirectional` for the data plane
- The overall code simplicity and file count discipline
- clap derive for CLI

**What Tunelo should NOT copy:**
- bore routes by TCP port, not by hostname. Tunelo must route by subdomain (like cloudflared)
- bore's data channels are new TCP connections per request — Tunelo should multiplex over QUIC streams instead
- bore has no TLS termination, no HTTP awareness, no hostname routing
- bore has no reconnection logic

**Most relevant for:** CLI UX, code simplicity

---

### 1.2 rathole (rathole-org/rathole)

**What it does well:**
- Clean Transport trait abstraction: `trait Transport { type Stream; fn connect(); fn bind(); fn accept(); fn handshake(); }`
- Supports TCP, TLS (native-tls + rustls), Noise protocol, WebSocket — all behind the same trait
- Control channel + data channel with session_key-based correlation (similar to bore's UUID, but uses cryptographic nonces)
- Connection pooling: pre-creates data channels so they're ready when visitors arrive
- Hot-reload of configuration
- Feature flags for transport selection (compile-time)
- SocketOpts abstraction for TCP tuning (nodelay, keepalive)

**What Tunelo should borrow:**
- The Transport trait abstraction — but simplified for QUIC-first
- Connection pool concept (pre-warmed QUIC streams)
- The control channel / data channel naming convention
- The clean separation of protocol messages from transport

**What Tunelo should NOT copy:**
- rathole is config-file heavy (TOML), Tunelo should be CLI-first with minimal config
- rathole routes by service name hash, not by hostname — Tunelo needs subdomain routing
- Over-engineering of transport variants for MVP — start with QUIC only
- Binary protocol with fixed-size messages (bincode) — harder to debug than length-prefixed msgpack/JSON

**Most relevant for:** Code structure, transport abstraction, performance model

---

### 1.3 frp (fatedier/frp)

**What it does well:**
- Full-featured virtual host routing: `pkg/util/vhost/` has a domain→location→httpUser router
- Wildcard domain matching (`*.example.com`)
- HTTP-aware muxing: reads Host header from incoming connections, routes to the right proxy
- Subdomain support: client can request `test.frps.example.com` and frps routes HTTP to it
- Multi-proxy-type support: HTTP, HTTPS, TCP, UDP, STCP, XTCP
- Control/work connection model similar to bore/rathole

**What Tunelo should borrow:**
- The vhost routing model: accept HTTP connection → peek Host header → look up registered tunnel → forward
- Domain-based routing table (map[hostname] → tunnel_session)
- The concept that the client declares what subdomain it wants and the server validates/assigns it

**What Tunelo should NOT copy:**
- frp is massively complex (~50k+ lines). Tunelo must be 10x simpler
- frp's configuration model is heavyweight (INI/TOML files on both client and server)
- frp uses its own custom multiplexing (yamux). Tunelo should use QUIC streams natively
- frp mixes too many features (dashboards, plugins, load balancing) — scope creep

**Most relevant for:** Routing model, hostname architecture

---

### 1.4 reverst (flipt-io/reverst)

**What it does well:**
- QUIC-native: uses `quic-go` for the tunnel transport
- Client opens QUIC connection to server → registers as listener → server uses that connection as an `http3.RoundTripper` to proxy requests back through the tunnel
- Elegant "reverse HTTP/3 server" pattern: the client literally becomes an HTTP/3 server over the QUIC connection, and the server-side acts as a reverse proxy forwarding HTTP requests through it
- Round-robin load balancing across multiple tunnel connections
- TunnelGroup concept: hostnames map to groups, groups map to connections

**What Tunelo should borrow:**
- QUIC as the tunnel transport (but in Rust via quinn)
- The "client registers, server routes HTTP to it via the tunnel connection" model
- The hostname → tunnel group → connection routing chain
- Using the tunnel QUIC connection to carry HTTP requests as multiplexed streams

**What Tunelo should NOT copy:**
- reverst depends heavily on Go's http3 package to serve HTTP/3 over the QUIC connection — Rust doesn't have an equally mature equivalent, so Tunelo will use raw QUIC streams instead
- reverst's registration protocol uses msgpack — Tunelo should use a simpler frame format
- The config-file-based tunnel group definition — Tunelo should be dynamic

**Most relevant for:** QUIC architecture, tunnel protocol design

---

### 1.5 Cloudflare Tunnel (cloudflared)

**What it does well (conceptually):**
- The user-facing model: "run `cloudflared tunnel` and get a public hostname"
- Outbound-only connection: the connector (client) establishes the connection, the edge routes traffic into it. No inbound ports needed
- Public hostname routing: `abc123.trycloudflare.com` → edge looks up tunnel → forwards request through the tunnel
- Quick tunnel (TryCloudflare): zero-config, instant public URL — exactly the UX Tunelo targets
- Multiplexed tunnel: single connection carries many proxied requests
- HTTP/2 multiplexing over the tunnel connection (h2mux)

**What Tunelo should borrow:**
- The zero-config instant URL model: `tunelo port 3000` → get a URL
- The "public hostname bound to a live tunnel session" mental model
- Outbound-only connection model
- Multiplexed request forwarding over a single tunnel

**What Tunelo should NOT copy:**
- cloudflared's complexity (Argo Tunnel, named tunnels, DNS routing, access policies)
- HTTP/2-based tunnel mux — use QUIC streams instead (better head-of-line blocking, built-in multiplexing)
- The enterprise control plane concepts — Tunelo MVP needs no dashboard, no teams

**Most relevant for:** Product UX, hostname model, mental model

---

### 1.6 Summary: Which Project is Most Relevant For Each Concern

| Concern              | Most Relevant Project | Reasoning |
|----------------------|----------------------|-----------|
| CLI UX               | **bore**             | Minimal, clean, clap-based |
| Tunnel protocol      | **reverst + bore**   | QUIC connection + control/data channel split |
| Routing model        | **frp + cloudflared**| Subdomain → tunnel hostname routing |
| Code structure       | **bore + rathole**   | Small file count, clear module boundaries |
| Performance model    | **rathole**          | Rust, zero-copy data path, transport trait |
| QUIC implementation  | **reverst** (conceptually) + **quinn** (library) | QUIC-native tunnel design |

---

## 2. Rust-First Decision Review

### 2.1 Is the ~2x cost-efficiency hypothesis plausible?

**Yes, plausibly.** Here's why:

The gateway/relay is fundamentally a **data-plane proxy**: it terminates TLS on the public side, looks up a hostname → tunnel mapping, and copies bytes between the public connection and a QUIC stream to the client. This is:

- **I/O bound with many concurrent connections** — Rust's async model (tokio + epoll/io_uring) has lower per-task overhead than Go goroutines (~2-8KB stack each in Go vs ~few hundred bytes per Rust future)
- **Memory-sensitive at scale** — A gateway holding 10K concurrent tunnel sessions, each with burst traffic, benefits enormously from Rust's zero-overhead async and no-GC memory model. Go's GC pauses can cause tail latency spikes under memory pressure
- **Data copy intensive** — `tokio::io::copy_bidirectional` can use splice/sendfile on Linux. Go's `io.Copy` also uses splice, but Rust's ownership model means fewer intermediate buffer copies in the framing layer
- **TLS termination is CPU-bound** — rustls is competitive with Go's crypto/tls, and Rust generally wins on CPU-intensive workloads

**Concrete estimate:** For a relay server handling 10K concurrent tunnels:
- Go: ~500MB-1GB RSS, ~2-5ms p99 GC pauses under load
- Rust: ~100-200MB RSS, no GC pauses
- At 2x less memory per server = 2x more tunnels per VM = ~2x infra cost reduction for the data plane

This is conservative. Real-world Rust network services (e.g., Cloudflare's pingora, Linkerd2-proxy) consistently show 2-5x memory improvement over Go equivalents.

### 2.2 Where Rust matters most

| Component       | Rust Benefit | Verdict |
|----------------|-------------|---------|
| **Gateway/relay data plane** | HIGH — memory, throughput, tail latency | ✅ Must be Rust |
| **Client CLI** | MEDIUM — single static binary, fast startup, low memory | ✅ Rust (shared codebase) |
| **Protocol library** | HIGH — shared between client and server, zero-copy parsing | ✅ Rust |
| **Control plane / API** | LOW — small request volume, correctness > perf | Could be Go/Rust, but Rust for codebase unity |
| **Edge TLS termination** | MEDIUM-HIGH — CPU-bound, concurrent | ✅ Rust |

### 2.3 Decision

**Use Rust for everything in the MVP.** Rationale:
1. Single language = single build system, single CI, one set of dependencies
2. The gateway IS the product — it must be fast and memory-efficient
3. The client and server share the tunnel protocol library — one crate
4. Rust's ecosystem (tokio, quinn, rustls, clap, hyper) is production-ready for this workload
5. bore and rathole prove that Rust tunnel tools can be simple and ship fast

**Risk mitigation:**
- If control-plane complexity grows (dashboards, billing, team mgmt), consider a separate Go/TypeScript service later
- For MVP, the control plane is literally "assign a random subdomain" — trivial in any language

---

## 3. Architecture

### 3.1 High-Level Design

```
                         ┌─────────────────────────────────┐
                         │          tunelo gateway          │
  Public Internet        │                                  │
                         │  ┌───────────┐  ┌────────────┐  │
  User browser ─────────►│  │ HTTPS     │  │ Tunnel     │◄─┼──── tunelo client
  abc123.tunelo.net      │  │ Listener  │──│ Router     │  │     (QUIC connection)
                         │  │ (rustls)  │  │            │  │
                         │  └───────────┘  └────────────┘  │
                         │       │               │         │
                         │       └───────┬───────┘         │
                         │               │                 │
                         │  ┌────────────▼──────────────┐  │
                         │  │    QUIC Tunnel Manager     │  │
                         │  │  hostname → QUIC conn map  │  │
                         │  └───────────────────────────┘  │
                         └─────────────────────────────────┘
                                         │
                                    QUIC stream
                                         │
                         ┌───────────────▼─────────────────┐
                         │        tunelo client             │
                         │                                  │
                         │  QUIC conn → control stream      │
                         │           → data streams         │
                         │                                  │
                         │  Forward to localhost:3000       │
                         └─────────────────────────────────┘
```

### 3.2 Connection Flow

1. **Client starts:** `tunelo port 3000`
2. **Client connects:** Opens QUIC connection to `gateway.tunelo.net:4433`
3. **Registration:** Client opens QUIC stream 0 (control stream), sends `Register { local_port: 3000 }` message
4. **Gateway assigns hostname:** Gateway generates `abc123.tunelo.net`, stores mapping `abc123 → quic_connection`, replies with `Registered { hostname: "abc123.tunelo.net", tunnel_id: "..." }`
5. **Client displays:** `✔ Tunnel ready: https://abc123.tunelo.net → localhost:3000`
6. **Public request arrives:** Browser hits `https://abc123.tunelo.net/api/hello`
7. **Gateway routes:** TLS termination → reads Host header → looks up `abc123` → finds QUIC connection → opens new QUIC stream → sends HTTP request bytes
8. **Client receives:** Reads from QUIC stream → opens TCP connection to `localhost:3000` → forwards request → reads response → sends back over QUIC stream
9. **Gateway responds:** Reads response from QUIC stream → sends to browser

### 3.3 Protocol Design

**Control Stream** (bidirectional QUIC stream 0):
```
Client → Server: Register { version: 1, requested_subdomain: Option<String> }
Server → Client: Registered { hostname: String, tunnel_id: String }
                 | Error { code: u16, message: String }

Server → Client: Heartbeat
Client → Server: HeartbeatAck

Server → Client: Shutdown { reason: String }
```

**Data Streams** (new bidirectional QUIC stream per request):
```
Server → Client: RequestHeader { 
    method: String, 
    uri: String, 
    headers: Vec<(String, String)>,
    has_body: bool 
}
Server → Client: [request body bytes, if any]

Client → Server: ResponseHeader {
    status: u16,
    headers: Vec<(String, String)>,
    has_body: bool
}
Client → Server: [response body bytes, if any]
```

Messages are length-prefixed (4-byte big-endian length + msgpack payload). Body bytes are raw.

### 3.4 Crate Structure

```
tunelo/
├── Cargo.toml              (workspace)
├── tunelo-protocol/        (shared protocol types + codec)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── messages.rs     (Register, Registered, RequestHeader, ResponseHeader, etc.)
│       └── codec.rs        (length-prefixed frame read/write)
├── tunelo-client/          (CLI + client logic)
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs         (clap CLI)
│       ├── tunnel.rs       (QUIC connection management, control stream)
│       └── proxy.rs        (data stream → localhost forwarding)
├── tunelo-gateway/         (gateway server)
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs         (server startup, TLS, QUIC listener)
│       ├── router.rs       (hostname → tunnel_session routing table)
│       ├── tunnel.rs       (tunnel session management, control stream handling)
│       ├── http_listener.rs (accept public HTTPS, peek Host, forward)
│       └── proxy.rs        (open QUIC stream to client, relay request/response)
└── DESIGN.md
```

This is 8 source files total. Similar to bore's 5-file discipline, but with proper separation.
