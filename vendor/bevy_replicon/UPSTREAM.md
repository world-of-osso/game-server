# bevy_replicon

- Version: `0.41.1`
- Crates.io checksum: `a25105650eb1b27c6ada888e06bb38ff7e326a94c7c5b4d75fb5aee50c5ae45e`
- Upstream commit: `60e3c5ea164385b1b95932542f949d123c3db5c4`
- License: MIT OR Apache-2.0

Vendored because Lightyear 0.28 delegates server replication traversal to Bevy Replicon. The local patch replaces steady-state all-replicated-entity mapping/change scans with global and sender-targeted dirty candidates, while preserving Replicon's component registry, visibility filters, insertion/removal/despawn actions, serialization, priority, mutation ACK tracking, and resend behavior. New senders still receive one full candidate set. Test-only diagnostics count candidate and entity visits.

The default feature set omits `world_serialization`: nothing here uses Replicon world serialization, and it pulled `bevy_asset`/`bevy_camera`/`bevy_image`/`bevy_mesh`/`bevy_window` into every Lightyear consumer, including the headless simulator. Lightyear 0.28 requests Replicon defaults, so this vendored default is the only switch.
