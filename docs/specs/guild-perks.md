# Guild perks

Mainline installed `GuildPerkSpells` membership grants/removal, following TrinityCore a352b1fa `Guild.cpp:2322,3002`. Definitions and real spell metadata come from active DB2 exports, not a hard-coded perk list. [Guild systems](../wiki/systems/guild-lifecycle.md).

## What it must do

- [x] Actual guild members know the five installed perk spells; nonmembers do not receive them. Joining, quitting/removal and login reconcile the authoritative membership.
- [x] Owner learn/unlearn messages reflect those changes without removing unrelated class/profession knowledge. Ordinary class/spec rebuilds preserve guild grants without transient unlearning.
- [x] Fresh processes preserve the correct membership-derived knowledge after leave; never persist a stale guild entitlement as permanent class knowledge.
- [x] Forced/offline removal also revokes guild knowledge and remains revoked after relog.
- [x] Report actual effect coverage separately from catalog/known-spell availability; no invented buffs, mail acceleration, reputation or guild-chest authority.

## How it works

- [Guild systems](../wiki/systems/guild-lifecycle.md).

## Implementation inventory

| File | Role |
|---|---|
| `crates/server/src/class_progression.rs` | Baseline/PvP/guild knowledge composition and rebuild preservation |
| `crates/server/src/guild_perks.rs` | Installed catalog and membership-derived owner learn/unlearn updates |
| `crates/server/src/main.rs` | Required startup catalog load |

## Tests asserting this spec

- `spell_cast::charge_snapshot_tests::guild_hasty_hearth_...` — four actual-spell controls cover isolated30/15/30-minute recovery, consumed-clock preservation, and Human458619 mixed25/10/25-minute queued uses. Ten existing rate and three snapshot controls also pass.

- `auras::mount_tests::guild_mount_up_...` — three real-data controls cover ground/flight grants and revocation, stronger Crusader Aura without stacking/swim leakage, and installed battleground/arena map suppression/restoration.

- `class_progression::tests::guild_perks_real_membership_grants_rebuilds_revokes_and_reopens_without_touching_class_spells` — real persisted invite acceptance/quit, knowledge, owner messages, spec rebuild and reopen.

## Known gaps (current cycle)

- [x] Real persisted join, changed specialization, quit and fresh-process class-knowledge test GREEN1/1 at serveree29a3bd/protoe50b02c.
- [x] Existing-member login, online forced removal/rejoin and offline removal/reopen controls GREEN2/2 at5dd1cfa9.
- [x] Supported Mount Up172/211 and map suppression landed;210 remains a named blocked context.
- [x] Hasty Hearth is separately qualified below. Guild Mail has a real same-guild zero-delay control.
- [ ] Mobile Banking remains source-blocked: installed83958 is script effect77 without an authored trigger;88304/88306 summon206602/206603 for300000ms, but pinned TC supplies no dispatch binding. Existing CharacterReputation already represents authenticated faction1168; no new reputation table is needed. The installed tooltip requires Friendly chest users, not explicitly Friendly casters. A contemporary fork's cast-time guard does not establish Mainline policy; see the [authority audit](../wiki/systems/guild-perks.md#mobile-and-resurrection-authority-audit--2026-10-17).
- [ ] Quick/Dead83950 remains source-blocked: installed aura340 carries50% but TC a352b1fa SpellAuraEffects.cpp412 uses HandleNULL; SpellAuraDefines.h427 names a guild-member resurrector whereas the installed tooltip is unqualified. Its second effect is dummy3/+10% dead movement, without a pinned script. Recipient/caster/guild, health calculation and dead-speed handling are not fabricated from catalog knowledge.
- [x] Bounded catalog/membership/UI slice landed; actual UDP invitation/removal/rejoin/quit and native fresh-process owner readback passed.

## Out of scope

Guild rewards/reputation eligibility and invitation tabards are separate slices. Missing effect support stays explicit rather than being claimed from a known spell ID.

## Active effect slice: Hasty Hearth

- [x] Actual Hearthstone8690 consumes charge category2309:1800000ms without83944,900000ms with its authored453/-900000ms passive; revocation restores future uses.
- [x] Joining or leaving does not retime an already-consumed Hearthstone charge (TC SpellAuraEffects.cpp525 and SpellHistory.cpp834–853).
- [x] Preserve actual Human458619: two charges,25-minute nonmember/10-minute guild consumption. Mixed queued uses retain their own durations through join/removal; no racial knowledge is removed to simplify the fixture.
- [x] Actual native membership/cast/charge receipts establish both-skin behavior: identical binaries, four physical6948/8690 casts, two charges25/10-minute recovery, four actual invitation/removal cycles, exact114 non-guild IDs retained. Twelve inspected full-HD images establish bag/item/cast context; clock values come from native owner receipts, not rendered timer labels. Pending-clock persistence across restart is not claimed.

## Supported effect slice: Mount Up

- [x] Actual78633 grants10% mounted run/flight speed through authored172/211 effects, with strongest-positive nonstacking semantics (TC Unit.cpp8812–8854). Revocation restores the mounted baseline without removing the mount.
- [ ] **Blocked by explicit lead decision:**210 applies only to player-controlled NPC flight (TC Unit.cpp8833–8844). The local authored vehicle runtime is ground IoC Demolishers; no flying context is fabricated or applied to the mounted player. This is not part of the supported172/211 completion claim.
- [x] Source Attr4/8 arena/battleground restrictions suppress the passive effect while preserving knowledge and restore it on leaving.
- [x] Real native mounted/unmounted movement and guild-removal proof passes both skins: seven coherent actual samples each. Ten inspected mounted originals selected; four wall/tree-occluded removed-ground/unmounted frames are not claimed as pixel proof.

