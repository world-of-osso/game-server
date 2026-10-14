# Guild tabards

Guild tabard registration follows TrinityCore a352b1fa `Guild.cpp:51,749–753,1511–1528` and Retail Mainline `TabardFrame.lua`. [Guilds](../wiki/systems/guilds.md) owns membership.

## What it must do

- [ ] Existing guilds migrate to no registered design without changing bank contents, ranks, rights or allowances. A registered design survives reopen.
- [ ] An authenticated living player must retain a valid interaction with a guild tabard vendor. Only the guild leader may register a design.
- [ ] Each successful registration charges100000c from personal gold. Insufficient funds and rejected registration change neither gold nor design. Charge and design persist atomically.
- [ ] Colors are validated against installed GuildColorBackground/Border/Emblem rows; selectable designs use installed GuildEmblem and GuildTabard texture data.
- [ ] Guild snapshots, character-selection roster entries and equipped guild tabards expose the authoritative appearance, including subsequent design changes. Registration never rewrites character customization or owned equipment.

## How it works

- [Guild lifecycle](../wiki/systems/guilds.md).

## Implementation inventory

- `crates/server/src/persistence_guild_tabards.rs`: optional design storage and atomic payment. Additive table migration leaves all legacy GuildData layouts intact.
- `crates/server/src/guild_tabards.rs`: authenticated vendor requests, source catalog validation, membership-design replication and replies.
- `crates/server/src/guild.rs`: snapshot appearance and online member broadcasts.
- `crates/server/src/auth.rs`, `admin_handlers.rs`: persisted guild design in character-selection roster entries.

## Tests asserting this spec

- `crates/server/src/persistence_guild_tabards_tests.rs`: database reopen, legacy guild preservation, leader authority, exact/repeated payment and rejection state.
- `crates/server/src/npc_interaction/guild_tabard_tests.rs`: real Jondor5130 gossip/service, host-client requests, catalog/auth/target/range rejection and successful replicated design/payment.

## Known gaps (current cycle)

- [ ] Vendor request routing and source catalog validation.
- [ ] Appearance snapshots/replication and native TabardFrame/character rendering.

## Out of scope

- Personal tabards (Retail210469); this feature is guild registration.
