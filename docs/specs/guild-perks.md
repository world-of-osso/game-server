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

- `class_progression::tests::guild_perks_real_membership_grants_rebuilds_revokes_and_reopens_without_touching_class_spells` — real persisted invite acceptance/quit, knowledge, owner messages, spec rebuild and reopen.

## Known gaps (current cycle)

- [x] Real persisted join, changed specialization, quit and fresh-process class-knowledge test GREEN1/1 at serveree29a3bd/protoe50b02c.
- [x] Existing-member login, online forced removal/rejoin and offline removal/reopen controls GREEN2/2 at5dd1cfa9.
- [ ] Four perk effects remain explicitly unsupported/unproven; Guild Mail retains existing same-guild zero-delay behavior.
- [ ] Private real-UDP/native join/leave/relog acceptance and independent consumer gates.

## Out of scope

Guild rewards/reputation eligibility and invitation tabards are separate slices. Missing effect support stays explicit rather than being claimed from a known spell ID.

## Active effect slice: Mount Up

- [ ] Actual78633 grants10% mounted run/flight speed through authored172/211 effects, with strongest-positive nonstacking semantics (TC Unit.cpp8812–8854). Revocation restores the mounted baseline without removing the mount.
- [ ] Authored210 is applied only in its controlled-vehicle flight context.
- [ ] Source Attr4/8 arena/battleground restrictions suppress the passive effect while preserving knowledge and restore it on leaving.
- [ ] Real native mounted/unmounted and guild-removal proof; do not claim effect completion from visible rows or known IDs.

Development proof: actual Golden Gryphon32235 + expert riding225 baseline is run2.0/flight3.2; Mount Up must produce2.2/3.52. Test33ec603a RED reports actual run2.0, after real passive application. Earlier289dbe3b failure was missing fixture SpellStore metadata, not a product RED.
