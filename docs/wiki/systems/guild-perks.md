# Guild perks

[Contract](../../specs/guild-perks.md). Shared `load_guild_perks` reads active GuildPerkSpells, SpellName and SpellMisc exports; server startup requires the real catalog. This is not the Trading Post Perks tables or a fabricated reward list.

## Knowledge and membership

`KnownSpells` contains three vectors: baseline learned IDs, PvP replacement/base pairs, and membership-derived guild IDs. Its iterator combines them without duplicate IDs. Guild IDs are not written into professions or persisted class progression. Class/spec evaluation preserves the guild vector so an ordinary rebuild does not transiently unlearn perks.

`GuildPerksPlugin` observes changed membership/knowledge and removed membership components after Update mutations. It replaces only the guild vector and publishes actual owner SpellsLearned/SpellsUnlearned deltas through the existing TalentChannel. The existing known-spell passive-aura reconciliation consumes the combined knowledge; listing/learning alone does not prove a perk effect works.

## Proof boundary

The real persisted invite-acceptance test reproduced zero perk grants at server64fe5764. Implementation is committed before its focused GREEN run. The test also asserts no unrelated class losses, no perk unlearning during a real spec change, actual quit revocation and fresh-database-process knowledge. Private UDP/native proof and individual effect coverage remain open. Client source list/catalog coverage is tracked in the [client contract](../../../../game-engine/docs/specs/guild-perks.md).

## Sources

Installed Mainline GuildPerks.lua/XML; active GuildPerkSpells rows2/7/10/14/16. TrinityCore a352b1fa Guild.cpp2322 grants the perk store's spells on guild login;3002 removes them on leaving/removal.
