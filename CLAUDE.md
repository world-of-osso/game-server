# game-server

WoW-inspired multiplayer game server using headless Bevy as ECS runtime + lightyear for networking. Server is authoritative; clients predict locally and reconcile with server state.

See ARCHITECTURE.md for full design.

## Structure

```
crates/
  shared/    — Components, types, game formulas. Used by both client and server.
  server/    — Server binary. Headless Bevy with lightyear server plugin.
  client/    — Client stub. Will link to wow-engine later.
```

## Reference Repos

- `~/Repos/azerothcore` — C++ WoW 3.3.5a server (architecture reference for zone/map threading, combat systems)
- `~/Repos/rustbolt-core` — Rust WoW server attempt (early stage, structure reference)
- `~/Projects/wow/wow-engine` — Sibling Bevy 0.18 project (client renderer)

## Dev

- `cargo run -p server` — Start server (binds to 0.0.0.0:5000)
- `cargo test` — Run all tests
- `./run-tests.sh` — cargo test + clippy
- Edition 2024, rust-version 1.89
- Bevy 0.18, lightyear 0.26, redb 3, bitcode 0.6
