# Guild perks

[Contract](../../specs/guild-perks.md). Shared `load_guild_perks` reads active GuildPerkSpells, SpellName and SpellMisc exports; server startup requires the real catalog. This is not the Trading Post Perks tables or a fabricated reward list.

## Knowledge and membership

`KnownSpells` contains three vectors: baseline learned IDs, PvP replacement/base pairs, and membership-derived guild IDs. Its iterator combines them without duplicate IDs. Guild IDs are not written into professions or persisted class progression. Class/spec evaluation preserves the guild vector so an ordinary rebuild does not transiently unlearn perks.

`GuildPerksPlugin` observes changed membership/knowledge and removed membership components after Update mutations. It replaces only the guild vector and publishes actual owner SpellsLearned/SpellsUnlearned deltas through the existing TalentChannel. The existing known-spell passive-aura reconciliation consumes the combined knowledge; listing/learning alone does not prove a perk effect works.

## Proof boundary

Persisted invitation/quit, changed specialization, existing-member login, forced/offline removal and reopen controls pass. Actual private UDP invitation acceptance→officer removal→rejoin→quit produces exact perk learn/unlearn deltas while retaining class knowledge; the bounded catalog/membership/UI slice landed. Client native five-tooltip/chat/caret and identical-binary fresh-process owner readback also pass. Individual effects remain separate from these knowledge proofs.

## Supported Mount Up:172/211

Actual78633 carries nonstacking run172 and flight211 at10%. The movement accumulator takes the strongest positive nonstacking amount and chooses the larger of that multiplier and the existing stacking multiplier, then applies the mounted speed aura. Thus expert-riding Golden Gryphon32235 changes run2→2.2 and flight3.2→3.52; actual Crusader Aura32223 at20% wins with2.4/3.84 rather than multiplying both bonuses. These two modifiers do not enter the existing swim normalization path. Revoking the guild-derived spell removes its passive and restores the ordinary mounted rates without unmounting.

Passive synchronization also observes actual InMap changes. Required installed MapStore metadata interprets Attr8 NOT_IN_BATTLEGROUND and Attr4 NOT_IN_ARENA_OR_RATED_BATTLEGROUND for the supported battleground/arena map types. Knowledge is retained; only an ineligible passive application is removed, then restored when the character returns to an allowed map. No missing-map fallback or spell-ID override is added.

Aura210 is **explicitly blocked**, per lead decision: TC Unit.cpp8833–8844 applies it to player-controlled NPC flight, not the mounted player. The local authored vehicle path is ground IoC Demolishers, where the perk is excluded by battleground rules. No flying vehicle or extra mounted multiplier is fabricated.

Real-data RED33ec603a recorded run2 instead of2.2; REDc0f608da recorded the passive surviving a move to installed WSG489. GREEN3/3 at6f27ebd6 covers the rates, Crusader nonstacking and WSG489/NagrandArena559→continent0 restoration. The earlier fixture-only missing SpellStore78633 failure is not a product RED. A functional both-skin native run measured real replicated run14→15.4→14 and flight22.4→24.64→22.4 through actual invitation acceptance/removal; final outdoor capture review remains separate.

Sources: pinned TC Unit.cpp8812–8854, SpellInfo.cpp2218, Spell.cpp7158 and Player.cpp26509; actual installed SpellEffect and Map rows.

## Sources

Installed Mainline GuildPerks.lua/XML; active GuildPerkSpells rows2/7/10/14/16. TrinityCore a352b1fa Guild.cpp2322 grants the perk store's spells on guild login;3002 removes them on leaving/removal.
