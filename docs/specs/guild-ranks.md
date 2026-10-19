# Player-managed guild ranks

Retail guild rank controls live in `crates/server/src/guild_ranks.rs`. The server
owns authority and persistence; see [banks](../wiki/systems/banks.md) for data flow.
Wire contract: shared-protocol `docs/guild-ranks.md`.

## What it must do

### Rank controls
- [x] Only Guild Master creates, removes, renames, reorders or configures ranks.
- [x] There are 2–10 ranks, highest first. Rank 0 is immutable. Occupied ranks cannot be removed.
- [x] Reordering preserves member rank identity; removing empty ranks renumbers lower positions.
- [x] Adding ranks preserves members with an implicit lowest rank.
- [x] Promote/demote requires its flag and a lower-ranked target in the same guild. Promotion never reaches the actor's rank; rank 0 never changes.
- [ ] Names are nonempty, at most 15 characters, without control characters.

### Online member zone
- [x] Guild member snapshots carry an optional actual zone label only for online members, resolved through existing GroundMap area and AreaTable zone authority and real `area_table.AreaName_lang` data. Missing position/map/area/name and offline members produce None; no map-name or ZoneN substitute.
- Area names use the existing loaded AreaTable; installing the guild plugin must not open a separate database.
- [x] New guild queries and existing state refresh paths recompute online labels, preserve guild membership/note privacy, and never persist the presentation label as player position authority.
- Source: TC a352b1fa `Guild.cpp:1331–1352` AreaID; Mainline `CommunitiesMemberList.lua:115–120,1285–1297`, `GuildRoster.lua:115`.
- Test: `guild_online_zone_native_query_uses_parent_area_refreshes_and_preserves_privacy`, actual HostClient query/reply consumed during Update. Behavioral RED preceded implementation; Targeted GREEN, retained140/0/3ignored and coordinated gz689 passed. Exact landed builds passed accepted two-member both-skin real-zone/refresh/disconnect native proof; engine `docs/wiki/systems/guilds.md#online-member-zone` owns artifact/PNG provenance and proof limits.
- Protocol/server/engine land together. Zone sorting, offline timestamps and Club/mobile presence are excluded.

### Direct member-rank assignment
- [x] One authenticated `SetMemberRank { character_name, rank }` assigns the target directly to an existing zero-based rank in one rank/member transaction; no chained step requests. The actor/guild come from the connection and persisted membership, never the packet.
- [x] Choose Demote when destination>current, Promote otherwise, matching pinned TC `Guild.cpp:1894–1930`. Recheck the corresponding right, reject self with the existing InvalidName refusal, and retain strict hierarchy: target and destination must both be below actor; rank0/out-of-range/cross-guild targets refuse without writes. Authorized unchanged destination leaves state unchanged; the source UI does not send current selection.
- [x] Normal recipient snapshots/ChangedGuilds refresh follow the existing handler. Current rank settings/other members remain unchanged; assigned state survives redb reopen.
- [x] Direct TC `SendGuildRanksUpdate`→`Member::ChangeRank` has no lifecycle-log call. Direct assignments append no synthetic step events; explicit Promote/Demote commands keep their existing logging.
- Named exclusions: `GUILD_RANK_ASSIGNMENT_AUTHENTICATOR_GAP`, `GUILD_RANK_ASSIGNMENT_LEADERSHIP_GAP` (transfer/disband), `GUILD_RANK_ASSIGNMENT_CONFIG_EXCLUDED` (existing creation/reordering unchanged). No authenticator eligibility fields are fabricated. Protocol/server/client land together; native proof is post-landing.

### Social permissions
- [x] Guild/officer chat requires separate listening and speaking rights; history records only authorized recipients.
- [x] Chat membership decisions use authoritative persisted guild IDs when components are stale.
- [ ] Invitations, MOTD, guild information and officer-note editing/viewing require their rank flags.
- [x] Leave removes a non-master's membership; a Guild Master receives ERR_GUILD_LEADER_LEAVE.
- [x] Remove-member requires the removal flag and a strictly lower-ranked same-guild target.
- [x] Removal atomically deletes membership, guild-local rank, note and pending invitation; daily bank usage remains. Online player components and recipient rosters refresh, including the removed member's empty roster. Rejoining starts at the lowest rank.

### Invitation lifecycle
- [x] Invitations reject already-guilded, already-invited, missing/offline and self targets with Retail GlobalStrings; invitation rights come from persisted actor membership.
- [x] Online invitees receive GuildInvitation; a persisted pending invitation is delivered on world entry. Decline consumes it.
- [x] Immediate and persisted delivery carry the inviting guild's real optional registered tabard. Missing designs remain absent; invitation storage is not populated with invented emblems. Real request/paid registration/reopened App+redb controls pass for both optional states (TC Guild.cpp1727–1738; Mainline GuildInviteFrame.lua11). Identical-binary actual UDP/native proof adds four invitations/four physical declines, both optional states and both skins; four full-HD originals inspected. [Provenance and exact proof boundaries](../wiki/systems/guild-lifecycle.md#invitation-appearance-delivery).
- [x] Acceptance cannot replace an existing guild; membership, initial lowest rank and invitation consumption commit in one redb transaction. Accepted membership survives reopen; online member rank views refresh.

### Bank permissions
- [x] Tab view/deposit and stacks/day are per rank/tab. Zero denies withdrawals; rank 0 alone is unlimited.
- [x] Gold withdrawal and repair share a copper/day allowance; extra withdrawals are refused.
- [x] Every bank request reloads the current rank. Hidden tab contents/text/logs are not disclosed.
- [x] Updated rights are effective for an already-open vault, and open vault views refresh after edits.

- [x] Hiding a tab preserves its configured deposit flag and daily stack limit; the action/view gate still refuses access.

### Authority and durability
- [x] Rank list and affected member positions commit together and survive database reopen.
- [x] Stored bank balances, tabs, limits, logs and repair permissions survive layout upgrades.
- [x] Social and monetary wire masks use pinned TC `a352b1fa` GuildRankRights values. Layout 3 translates each old layout-2 compact right independently; mixed officer permissions remain mixed and do not acquire newly modeled public-note rights. Layout 0/1 retain their existing grants; repeated startup never remaps already-migrated rights.
- [x] Every guild load restores all modeled Guild Master social rights, including newly modeled rights on already-deployed layout3, without changing other ranks or bank booleans/allowances (TC a352b1fa Guild.cpp262–264,318–321). Existing layout2 migration and layout3 two-reopen tests pass. Guild filter57 cases pass; the remaining raid-history case passes separately with its required read-only SITE_DATA_WORLD_COPY input (58 covered total).
- [ ] Only authenticated in-world connections can request controls; the actor comes from the connection, never the packet.
- [ ] Success broadcasts recipient-specific state; errors use typed Retail GlobalStrings refusals.

## How it works

- [Banks](../wiki/systems/banks.md#player-rank-controls) — rank transactions, layout migration, vault refresh and chat decisions.

## Implementation inventory

| File | Role |
|---|---|
| `crates/server/src/guild_ranks.rs` | Rank requests, authority and member mapping |
| `crates/server/src/guild.rs` | Plugin registration and profile permission checks |
| `crates/server/src/persistence_banks.rs` | Rank storage, atomic writes and layout migration |
| `crates/server/src/guild_bank.rs` | Current-rank bank enforcement and recipient views |
| `crates/server/src/messaging.rs` | Guild/officer chat and invitation permission checks |

## Tests asserting this spec

- `crates/server/src/guild_ranks_tests.rs` — GM controls, hierarchy/flags, bounds, immutable leader, rank identity and restart round-trip.
- `crates/server/src/guild_bank_tests.rs` — actual player packets, open-vault edits, item/gold limits and hidden logs.
- `crates/server/src/persistence_banks_tests.rs` — previous layouts and preserved limits.
- `crates/server/src/guild_rights_migration_tests.rs` — persisted single/mixed/empty/full layout-2 masks, enforcement, bank fields/member positions, reopen and idempotent startup.
- `crates/server/src/messaging.rs` tests — guild/officer recipients and chat history.
- `crates/server/src/merchant_guild_repair_tests.rs` — shared repair/gold allowance.

## Known gaps (current cycle)

- [ ] Client native settings UI and transport hookup are tracked in game-engine `docs/specs/guild-ranks.md`.
- [ ] Unchecked contract bullets have code but lack specific passing behavioral proof.

## Out of scope

Authenticator-only ranks and Discord permissions are not among the requested existing-feature flags. Member removal itself has no existing player feature; its rank flag is represented.

## Retail sources

Root: `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/`.

- `Blizzard_GuildControlUI/Blizzard_GuildControlUI.lua:4,536–620`: max 10, minimum 2, occupied-rank deletion and shifts excluding Guild Master.
- `Blizzard_GuildControlUI/Blizzard_GuildControlUI.xml:7`: 15-letter rank names.
- `Blizzard_GuildControlUI/Blizzard_GuildControlUI.lua:137–238,420–459`: tab view/deposit/stacks controls, flags and shared gold/repair allowance.
- `Blizzard_Communities/GuildRoster.lua:137–141`: strict promotion/demotion hierarchy.
- TrinityCore a352b1fa `Guild.cpp:262–264,318–321`: Guild Master LoadFromDB restores GR_RIGHT_ALL; SetRights forces it, preventing loss of leader rights. Other rank masks are not normalized.
- `game-engine/data/GlobalStrings.csv`: Retail refusal strings.
