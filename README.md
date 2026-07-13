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
