# Guild lifecycle history

Server-authoritative guild lifecycle events are distinct from [bank logs](banks.md). The [contract](../../specs/guild-event-log.md) tracks the ongoing slice; query and atomic producer wiring exist; backend acceptance4/4, actual UDP transport1/1 and both-skin native/server-process restart acceptance pass.

## Storage and query

`guild_event_log.rs` reads additive guild-keyed JSON histories without changing existing guild/rank bitcode records. Startup initializes the table; corrupt stored history fails explicitly rather than becoming an empty log. An absent guild history is genuinely empty.

`guild_ranks.rs` routes `QueryEventLog` through its authenticated in-world connection, using that actor's persisted membership to select history. Replies include current server time, oldest-to-newest events and a typed refusal for nonmembers. The request cannot select another guild. No data is reconstructed from administrative membership setup.

## Producer boundary

Pending invitation creation/acceptance, explicit step Promote/Demote requests and remove/quit append before their existing redb transaction commits. Charter registration appends actual founder joins with the supplied creation timestamp in its membership/charter-consumption transaction. Administrative set-membership does not reconstruct history. Names are captured from real character records; rank labels are captured before later renames can rewrite their meaning.

The ledger retains the latest100 entries in insertion order, including tied timestamps. Callers retain ownership of commit/abort; appending never opens or commits another transaction. Mainline reverses the returned order for display. Behavioral RED reproduced all six successful actions leaving an empty log; targeted4/4 GREEN atc317745f/protocol4fad8c7 covers all six real mutations, refused actions, no administrative backfill, event-time labels after rank renames, actual founder joins at the creation timestamp,103→100 tied-order retention and transaction abort/durable reopen. Verified2026-10-15 at engine27661fa5e/server8038e839/protocol4fad8c7: actual UDP preserves six kinds/order/refusal; real native Officer mutations and ordinary UDP target acceptance/quit/read/refusal/privacy pass in both skins. A hundred physical rank changes produce genuine100-row overflow; a fresh server/client process preserves every raw event and timestamp with identical binary SHA256 receipts. Eleven1920×1080 PNGs inspected; trial logs, history and artifact receipts: `/home/osso/.worktrees/logs/guild-lifecycle-2026-10-15-run4/`. Refreshed-master integration gates remain separate.

## Direct member-rank assignment

[Authority contract/exclusions](../../specs/guild-ranks.md#direct-member-rank-assignment): `SetMemberRank` uses the authenticated actor and current persisted membership, resolving the real same-guild target. It chooses a direction right from requested/current order, rejects self, preserves project strict hierarchy and validates the destination before changing the local member-position vector. The existing `save_guild_ranks` commits ranks/positions together and normal ChangedGuilds/recipient snapshots refresh the result. The shared pure `rank_has_right` check keeps Guild Master and bit semantics identical for direct assignments, existing steps and note/other social checks.

Pinned TC `Guild.cpp:1894–1930`→`SendGuildRanksUpdate`→`Member::ChangeRank:589–601` broadcasts/persists direct rank updates without calling its event log. This distinct direct request therefore passes no event to the transaction; it does not fabricate intermediate promotions/demotions. The existing six-kind step-command history is unchanged. UI display indices1..N are mapped to the wire0..N−1 contract by the client, never adjusted again by the server. Authenticator-backed assignment, leadership/disband and rank-configuration changes remain the contract's named exclusions. Actual backend RED2/2 showed the newly declared request still refused legitimate GM/multirank and promote-only requests; the earlier missing snapshot import was fixture compilation, not RED. Remaining GREEN/transport/joint gate/native proof is separate.

## Invitation appearance delivery

`guild_invites::load_pending_invitation` composes the persisted invitation identity with the guild's current real registered `GuildTabardDesign`. Both immediate delivery in messaging and world-entry redelivery call it; existing invitation storage remains unchanged. Missing designs remain `None`, not invented registrations. TC a352b1fa Guild.cpp1727–1738 and installed Mainline GuildInviteFrame.lua11 establish the data flow; [contract](../../specs/guild-ranks.md#invitation-lifecycle).

Real request/paid-registration/closed-App-and-redb-reopen controls cover both optional states and both delivery paths. Independent368 passes the server89 targeted controls and protocol gate. Actual private5891/native runs `text-fixed-no-design-1792235618` and `text-fixed-registered-1792237819` add four real officer invitations/four physical declines in both skins with identical server/engine/officer SHA256 receipts; native receipts retain the real guild/inviter and optional design. The native runs prove immediate delivery and registered-design readback from a fresh server, not a separate native persisted-invitation replay.

Four inspected1920×1080 originals are under `/syncthing/AgentShared/2026-10-15/guild-completion/{modern,forever}-invitation-{no-design,registered}-pending.png`; full receipts and source/binary manifests live at `/home/osso/.worktrees/logs/guild-invitation-tabards-2026-10-16-run1/`. Registration setup validates the installed catalog, finances one authored tabard fee in the private fixture and calls the real durable paid transaction. No earned-gold, vendor-interaction, achievement/reputation warning or expiry/Escape claim is made.

## Sources

- [Guild event-log contract](../../specs/guild-event-log.md) — source attribution and acceptance ledger.
- `crates/server/src/guild_event_log.rs`, `guild_ranks.rs`, `persistence.rs` — current query/table boundaries.

## See Also

- [Banks](banks.md) — separate per-tab item/money history and guild authority.

## Online member zone

`guild_online_zone.rs` extends the existing online-member query parameter with the existing GroundMap/AreaTable resources and a startup-loaded read-only zone-name catalog. Each existing guild snapshot path resolves only same-guild online actors' actual map/position to area, then the existing flagged immediate-parent zone, then its real AreaName_lang. Missing authority yields None. The label is not persisted or used as movement authority; offline state and recipient-filtered officer notes stay unchanged. No new per-frame mirror, sort, map-name substitute or network request. Protocol/consumer land together. Actual HostClient fixture delivers its reply into an Update observer before Lightyear's Last cleanup; earlier out-of-frame receiver inspection was a fixture error, not product RED. See [contract](../../specs/guild-ranks.md#online-member-zone). GREEN/gate/native pending.
