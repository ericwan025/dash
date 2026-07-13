# dash

A small automotive-style **infotainment system**, built to demonstrate a
multi-service architecture with clean, versioned APIs and a Flutter frontend.

Four independent Rust services (navigation, media, voice, settings) each expose
a versioned trait-based API and communicate over a shared in-process message
bus. A gateway service bridges that bus to a Flutter app over WebSocket.

---

## Architecture

```
                         ┌──────────────────────────────────────┐
                         │            Flutter app (macOS)         │
                         │   buttons + live state dashboard       │
                         └───────────────▲───────────┬────────────┘
                        ServerEvent JSON  │           │  ClientCommand JSON
                                          │           ▼
                         ┌──────────────────────────────────────┐
                         │          dash-gateway (axum WS)        │
                         │   client JSON  ⇄  bus events           │
                         └───────────────▲───────────┬────────────┘
                                         │           │
                          publish state  │           │  publish commands
                                         │           ▼
    ┌────────────────────────────────────────────────────────────────────────┐
    │                       dash-bus  (tokio broadcast)                         │
    │            every service holds a clone; fan-out to all subscribers        │
    └───▲──────────────▲───────────────▲──────────────▲────────────────────────┘
        │              │               │              │
   ┌────┴────┐   ┌─────┴─────┐   ┌─────┴─────┐   ┌────┴──────┐
   │  voice  │   │   media   │   │    nav    │   │ settings  │
   │  (NLU)  │   │ (playback)│   │(destination)│  │ (kv store)│
   └─────────┘   └───────────┘   └───────────┘   └───────────┘
```

The system is **event-driven**. Nothing calls another service directly; services
only ever publish and subscribe to typed events on the bus. A single flow:

```
user taps "Play"
  → app sends  {"type":"voice","transcript":"play music"}
  → gateway publishes  VoiceCommand{"play music"}  on the bus
  → voice service parses it, publishes  MediaControl{Play}
  → media service plays, publishes  MediaState{playing:true, track:"Highway Star"}
  → gateway forwards it to the app as  {"type":"media_state", ...}
  → app updates the Now Playing card
```

### Events: commands vs. state

The bus payload (`core::EventKind`) is a single `serde`-tagged enum split into
two groups — this split is the backbone of the design:

- **Commands** — a *request* to act, consumed by exactly one owning service:
  `VoiceCommand` → voice, `MediaControl` → media, `SetDestination` → nav,
  `SetSetting` → settings.
- **State** — a *fact* a service announces after acting: `MediaState`,
  `NavState`, `SettingsState`. The gateway relays these to the UI.

---

## Why separate services?

Splitting the domains into independent crates is the point of the project, and
it buys several things:

- **Isolation of failure and change.** Each service owns its state and its error
  type (`thiserror`). A bug or breaking change in nav can't reach into media —
  they share only the `core` vocabulary and the `bus`, never each other.
- **A real API boundary.** Each service is reachable *only* through its versioned
  trait (`v1::MediaApi`, `v1::NavApi`, …) or through bus events. There is no
  shared mutable state to reach around the API.
- **Independent testability.** Every service is unit-tested in isolation (its API
  directly) and integration-tested through the bus, with no other service
  running.
- **Loose coupling via pub/sub.** Because services communicate through broadcast
  events rather than direct calls, a new consumer (like the gateway, or a future
  logging service) can subscribe without any existing service knowing it exists.
- **A clean place for the frontend to attach.** The gateway is the *only* crate
  that knows about WebSockets and JSON; the services stay pure domain logic.

---

## Workspace layout

A Cargo workspace of seven crates:

| Crate           | Kind | Responsibility |
|-----------------|------|----------------|
| `core`          | lib  | Shared vocabulary: `Event`/`EventKind` (the wire schema), `ServiceId`, `CoreError`. No async, no transport. |
| `bus`           | lib  | The pub/sub message bus over `tokio::sync::broadcast`; `Bus` + `Subscription`. |
| `media`         | lib  | Media playback: `v1::MediaApi`, `MediaService`, bus runner. |
| `voice`         | lib  | NLU front door: `v1::VoiceApi`, transcript → `Intent` parser, bus runner. |
| `nav`           | lib  | Navigation: `v1::NavApi`, destination state, bus runner. |
| `settings`      | lib  | Validated key/value store: `v1::SettingsApi`, bus runner. |
| `gateway`       | bin  | axum WebSocket server bridging the bus to the Flutter app. |

The Flutter app lives in `app/` (macOS desktop target).

Each service crate follows the **same template**, so once you've read one you've
read them all:

```
service/
  error.rs    # the service's own ServiceError (thiserror), wraps CoreError
  types.rs    # domain types returned by the API
  api.rs      # pub mod v1 { #[async_trait] pub trait XApi { ... } }
  service.rs  # the concrete impl + unit tests
  runner.rs   # subscribe to the bus, react to commands, publish state + tests
```

### The service API pattern

Every service exposes an `async`, fallible, versioned trait:

```rust
#[async_trait]
pub trait MediaApi: Send + Sync {
    async fn play(&self) -> Result<PlaybackState, ServiceError>;
    async fn pause(&self) -> Result<PlaybackState, ServiceError>;
    // ...
}
```

- **Versioned** (`v1::MediaApi`) so a future `v2` can coexist with `v1` clients.
- **`&self`, not `&mut self`**, so an implementation can be shared behind an
  `Arc` and use interior mutability (a short-lived `std::sync::Mutex` — never
  held across an `.await`).
- **`Result<_, ServiceError>` everywhere** — no panics on bad input.

---

## How the gateway bridges Rust and Flutter

The services speak Rust types on an in-process bus; the Flutter app speaks JSON
over a WebSocket. `dash-gateway` is the single translation layer between them.

For each accepted WebSocket connection the gateway runs two concurrent halves,
joined with `tokio::select!`:

- **Outbound (bus → client):** subscribes to the bus and serializes every
  `Event` to a flattened [`ServerEvent`] JSON frame the UI can render.
- **Inbound (client → bus):** parses each text frame as a [`ClientCommand`] and
  publishes the corresponding **command** event onto the bus. Malformed frames
  are logged and ignored — never fatal to the connection.

The bus does the rest: a command published by the gateway is picked up by the
owning service, which publishes its new state, which the outbound half streams
back to every connected client. The gateway never contains domain logic — it
only moves bytes across the Rust/JSON boundary.

[`ServerEvent`]: crates/gateway/src/protocol.rs
[`ClientCommand`]: crates/gateway/src/protocol.rs
