# Retail guild charters

Guild creation uses authentic petitioner NPCs (flag262144) and owned Retail Guild
Charter item5863. Authority and persistence live in `persistence_charters.rs`; the
native client uses Retail registrar/petition controls in both skins.

## What it must do

- [ ] Purchase validates the real open petitioner, guild membership, name,1000c
  realm price, authentic item template and inventory space. Money, item and petition
  commit together; no vendor/world.db row is fabricated.
- [ ] Using the owned charter opens PetitionFrame; offering it delivers actual
  owner/title/signatures to the named online recipient.
- [ ] Signing rejects the owner, guilded/pending-invite players and a second
  character from an already signed account. Four signatures are required by realm
  defaults; the UI presents that same authoritative quota.
- [ ] Only the owner renames or submits. Pending titles do not reserve actual guild names.
- [ ] Creation atomically consumes the charter and petition, reserves the name,
  writes the actual date, gives the owner rank0 and signers the lowest default rank.
- [ ] Purchase, signatures, rename, membership/ranks and consumption survive reopen.
- [ ] Item disposal and joining another guild remove invalid petitions/signatures
  in the same inventory or membership transaction.
- [ ] Legacy guilds with no recorded date remain explicitly unknown, not backdated.

## How it works

- [Guild ranks](guild-ranks.md) — authoritative membership/rank updates.
- Shared-protocol `docs/guild-charters.md` — registrar/petition wire contract.

## Implementation inventory

| File | Responsibility |
|---|---|
| `crates/server/src/persistence_charters.rs` | Durable petition/account/item links and atomic creation |
| `crates/server/src/persistence_inventory.rs` | Item-disposal invalidation inside inventory saves |
| `crates/server/src/persistence_character.rs` | Joining invalidation inside membership writes |
| `crates/server/src/persistence.rs` | New tables and authentic legacy-name initialization |
| `crates/server/src/guild_charters.rs` | Authenticated request actor, live petitioner checks, owner/target routing and founding roster refresh |
| `crates/server/src/inventory.rs` | Owned charter UseItem before spell-use dispatch |
| `crates/server/src/npc_interaction.rs` | Reused live NPC health/faction/range validator |

## Tests asserting this spec

`crates/server/src/persistence_charters_tests.rs`: concrete purchased item/money,
unique signing accounts, pending invites, owner permissions, rename, reopen,
creation/lowest-rank founders, consumption and disposal.

## Known gaps (current cycle)

- [ ] Authenticated RPC/real NPC and bag-use live proof.
- [ ] Native registrar/petition integration.
- [ ] Both-skin authenticated charter creation captures.

## Out of scope

No requested guild-creation behavior is waived.

## Retail sources

- TrinityCore `a352b1fa9017138cc2f1dcfc0a51afaa17391ecf`,
  `src/server/game/Handlers/PetitionsHandler.cpp`: purchase38–119,
  rename188–221, sign223–302, offer322–353, turn-in355–443.
- Pinned `worldserver.conf.dist`: Guild.CharterCost1000 (762), MinPetitionSigns4 (1247).
- Local `content_item`5863: Guild Charter, quality1, one per player/stack,
  pickup-bound; item buy/sell price0. Registrar price comes from realm policy.
- Retail `Blizzard_UIPanels_Game/Mainline/{GuildRegistrarFrame,PetitionFrame}.{lua,xml}`.
