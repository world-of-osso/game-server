# Pet battle engine

Standalone `server::pet_battle` library, consumed by the separate [wild runtime](../../specs/wild-pet-battles.md). The library itself owns no Bevy plugin, transport, UI, spawning or account journal. Current capability is bounded; see [full property coverage](../../pet-battle-effect-coverage.md), not a blanket Retail parity claim.

## Content

`Catalog::load_world_db` strictly loads ten battle SQLite tables. Missing schemas, invalid types, conflicting known state values and dangling turn/ability/aura references return contextual errors. Identical species4597 speed-state rows are deduplicated; undefined species-state rows (35 in this export) remain in `unresolved_states`. No runtime CSV fallback. Tests copy the ten battle tables from shipped `data/world.db` into isolated in-memory SQLite; no server-local CSV export dependency or mutation of shipped data. No data is committed.

Runtime producer: `scripts/import-pet-battle-tables.py <local-CASC-CSV-directory> <world.db>` atomically replaces the ten tables below, preserving unrelated tables. Original DB2 column names and numeric SQLite affinity are retained: descriptions/LuaName/ParamLabel fields are TEXT, StateMultiplier REAL, other fields INTEGER. The account pet catalog startup loads and retains the turn catalog from the same world.db. The importer requires all ten exports before beginning the database transaction; malformed rows fail explicitly. `scripts/tests/test_pet_battle_tables.py` proves multiline text, signed values, numeric types, idempotence, preservation and invalid/missing export rollback.

Verified 2026-10-10: canonical resolved world.db backed up beside it as `world.db.pre-petbattle-20261010`, then imported. Species3001, toy1166, content_item58185 and spell_effect629424 rows preserved. Current corrected CASC exports contain8086 species-state rows and14975 species-ability rows; the older inventory counts in the historical handoff predate corrected exports.

| DB2 | SQLite |
|---|---|
| BattlePetAbility | battle_pet_ability |
| BattlePetAbilityTurn | battle_pet_ability_turn |
| BattlePetAbilityEffect | battle_pet_ability_effect |
| BattlePetEffectProperties | battle_pet_effect_properties |
| BattlePetState | battle_pet_state |
| BattlePetSpeciesState | battle_pet_species_state |
| BattlePetBreedState | battle_pet_breed_state |
| BattlePetBreedQuality | battle_pet_breed_quality |
| BattlePetSpeciesXAbility | battle_pet_species_x_ability |
| BattlePetAbilityState | battle_pet_ability_state |

### Numeric provenance

Species + breed raw stamina/power/speed states multiplied by DB2 quality multiplier and level. Health `round(raw*quality*level/20)+100`; power/speed `round(raw*quality*level/100)`. Constants20/100 and health offset100 are runtime rules, **not derivable from DB2 alone**. Pinned TrinityCore `a352b1fa` CalculateStats supplies them; its rounding comment remains an upstream uncertainty. Preserve float32 arithmetic: Rare's CSV value.649999976 evaluated in f64 incorrectly makes species39 health1562 instead of float32 result1563. Species39/breed3/Rare/25 fixture proves1563/260/276; identical species4597 speed50 rows yield284, not a summed293.

Retail tooltip `Blizzard_FrameXML/SharedPetBattleTemplates.lua` defines power scaling `points*(1+.05*power)`, variance spread `variance/200`, healing dealt/taken multipliers and floor-formatted output. Blizzard5.3 accuracy article supplies baseline5% critical,1.5x critical damage and+/-10% variance. Current explicit DB2 variance overrides baseline spread. Old Legion emulator's2x critical is not followed.

Local family abilities236..245 describe passives and displayed strong+50%/weak-33%. Current weak factor.67 follows that displayed percentage; exact game-table factor and runtime rounding still need independent Retail proof. Beast is+25% below **half** health, not below25% HP. Aquatic's current description includes outgoing DoT+25%, as well as incoming DoT reduction50%.

### Round pipeline

`Battle::step` consumes immutable state + two decisions + injected `BattleRng`; returns next state and ordered semantic events. Trait draws uniformly0..999999; caller owns seed/replay. No global time or random state. Forfeit ends immediately. Faint replacement requires an explicit swap, does not tick/advance rounds, and requires the living opponent to pass.

Regular decisions are preflighted through every turn and referenced aura graph before RNG. Voluntary swaps precede attacks. Speed determines attack order; ties draw50/50 (legacy runtime reference, not captured Retail evidence). Pass continues committed multi-turn abilities. Faster lethal hits cancel the defeated pet's queued attack. Cooldowns tick for all three pets. Ending occurs when a team has no living pets; both dead teams draw.

Aura states are added/removed without altering their source metadata. Single-turn DoTs tick N times, including the application round. Multi-stage Curse of Doom218 retains its final turn5 despite duration4: four waiting rounds, then impact. Positive periodic63 Wish273, Tranquility254 and Renewing Mists511 follow the current active team pet while preserving the original source power. Photosynthesis268 instead stays on its wearer. Team Decoy333 shares two charges across swaps; recasts restore two, not another independent shield. Metabolic Boost194 refresh does not duplicate speed state. Control immunity uses Critter's broader root protection separately from Resilient's stun/sleep/polymorph protection. Time Stop600 removes aura927 and invokes event5 to grant Resilient924.

Nine families have CPU behavioral coverage: Beast, Flying, Critter, Magic, Mechanical, Undead, Humanoid, Aquatic and Dragonkin. Humanoid heal percentage comes from Recovery726/property53, not a new constant. Elemental negative-weather immunity now executes and is tested for Darkness and Scorched Earth, while beneficial Moonlight/Sunlight modifiers remain available.

### Effect expansion — 2026-10-10

Round-start ability procs establish first-strike priority before ordering; equal priorities use ordinary speed/tie RNG. Round-end procs stay bound to the original actor, even after a forced swap or completed continuation. Forced swaps cancel the displaced pet's queued attack. Damage records last-hit/round-condition states for drains and conditional effects.

Weather is one global aura, not six independently ticking copies. Its exact applied state deltas include all bench pets and record Elemental exemptions, so replacement/expiry cannot subtract skipped negative states. Sunlight403 adjusts the published maximum-health cap from an unmodified base; no healing is invented on application. Source-powered periodic heals use separate source and recipient modifiers. Scorched Earth171 ticks the two current front pets once each; split-team damage divides the pool among living enemies.

Current data distinguishes Vicious Fang156/Huge Fang930 ramp-on-hit from other ramp-on-use descriptions. Tail Sweep122 picks first/last conditional branches. Unknown kill routing is rejected rather than treating every135 row as self-kill. Source fields and per-property tests are linked in the [coverage report](../../pet-battle-effect-coverage.md).

### Effect continuation — petbattle-effects2

Read-only hosted-map spawn/species joins rank99 wild ability candidates; the [coverage report](../../pet-battle-effect-coverage.md) owns exact counts and remaining blockers. New paths include last-hit healing, lower-health/speed/state bonuses, charged attacks, guaranteed accuracy, typed bench heals and nonlethal attacks. Three-row team attacks follow the actual active slot instead of assuming slot0. Cyclone189 is enemy-team-bound through swaps and object-reset cleanup; Stench527/1010 addresses each enemy separately. Leech Life383 alone runs its healing/state event2 immediately after its successful damage row; generic reactive dispatch remains excluded.

### Lockdown continuation — petbattle-effects3

Nevermore522 and Spell Gulp1858 use property168 to apply a two-round enemy-pet aura738/1859. Its event9/property129 locks the wearer's next used ability for five rounds; subsequent49 removes that wearer's aura, not the original caster's. Local Retail tooltip Lua names `PROC_ON_ABILITY`; its dynamically fetched parameters are DB2 labels, not an OverrideIndex interpreter. Only these two wearer-owned callbacks execute, after the first turn of an actual ability use. Passes/swaps do not consume them; already committed multi-turn attacks continue. Exact Retail callback/expiry timing remains uncaptured.

Mind Games2390 is the choosing pet's own detriment:117 uses one-based slots1/2 and nine-round LockDuration. Its aura174 reference is not a stun application. `Pet.lockouts` stores three independent slot clocks alongside ordinary cooldowns; round-start ticking includes bench pets. Existing wild/PvP snapshot projection publishes actual lockdown and disables that slot, rather than reporting zero. This does not certify the complete Addius replacement/restoration encounter.

### Fade continuation — 2026-10-16

Current70391 has105 referenced property IDs, including new387; canonical's older26 phase-controller rows were not modified. The [current census](../../pet-battle-effect-coverage.md#current70391-continuation--2026-10-16) owns exact deltas. Only Fade1084 establishes random self-swap107: its round-start dodge1083 ends before its round-end reserve choice. Other107 routing remains rejected. Three actual unsupported107 RED cases become81/81 targeted GREEN; current admission1749/2829 (61.82%),993 abilities. Both-skin private5961 real-pointer/log/model captures prove bounded inherited HUD projection; no all-row or exact Retail timing claim.

### Two-way walls — 2026-10-16

Only the seven [source-qualified150 pairs](../../pet-battle-effect-coverage.md#two-way-walls150--bounded-proof2026-10-16) admit:326/434/443/464/480/2397 are team-owned persistent charged objects whose two direct-hit blocks are shared between both sides.123/126 are conditional metadata admissions for those six auras only. Existing refresh, periodic bypass and object cleanup paths are reused. Real-ID directional/expiry/swap/refresh/periodic/Tidal controls pass88; row admission1749→1756/2829 (61.82→62.07%),993→1006 ability graphs. Both-skin private5961 pointer proof covers Ice Barrier479's real health/aura/cooldown/log projection. Onyx438/439, multiple-wall interception order and exact Retail block feedback remain named gaps, not a blanket property150 parity claim.

### Bench resurrection112 — 2026-10-16

[Bounded2347/2348](../../pet-battle-effect-coverage.md#bench-resurrection112--implementation2026-10-16) restore dead inactive allies to5% maximum HP, never heal living allies, clear dead/old undead-expiry metadata and retain passive usage. The owned-team loader derives Is_Dead from persisted health; a real redb close/reopen boundary reproduces the missing-state defect. RED3+1→GREEN4 and catalog1760/2829 rows,1008 graphs are recorded. Both-skin private5961 actual2347 cast revives two persisted-dead reserves to78/1563, then pointer swap and Metal Fist147 win prove a revived reserve usable. Eight fullHD originals inspected;2348 native/VFX/full Retail timing remain separate. No generic property111 routing or client simulation added.

### Same-species damage370 — 2026-10-16

[Bounded Dragon's Call2283](../../pet-battle-effect-coverage.md#same-species-dragons-call370--implementation2026-10-16) uses literal `Pet.species: u32` passed from owned/wild Identity; unknown species fail explicitly. Additional matching roster pets increase base points before the existing direct-damage pipeline, excluding caster/enemies and never inferring species from family. The literal source has no alive filter; fainted roster identity is retained. Real owned-team RED reproduces unsupported370; GREEN6 passes,1761/2829 rows/1009 graphs. Both-skin private5961 physical2283 with two matching reserves deals85/77 (boosted75–92 range), updates HP/cooldown3 and retains strict asset mode/tripwire0; four fullHD originals inspected. Final gates pass; a later pure test-fixture extraction passes its focused matrix rerun without repeating unaffected scopes. Test-only roots and independent Retail dead-slot/timing certification remain excluded.

### Armageddon1025 — 2026-10-16

[Bounded source and proof](../../pet-battle-effect-coverage.md#armageddon1025--bounded-proof2026-10-16). Existing ordinary damage routes once to each living enemy; the final self-kill suppresses Mechanical Failsafe even when attacks miss or the final enemy faints. Allied reserves remain unchanged. Exact kill tuple and root stay bounded; no new opcode/generic sacrifice rule. Real route-rejection RED1→GREEN8 covers18 active-slot/team combinations and a real authored level4 Rascal-Bot owned loadout. Admission1776/2829 rows/1013 graphs. Both-skin strict native casts deal61/58, caller321→0 without revival, force a real reserve selection and permit its actual attack; eight fullHD originals inspected. Native one-enemy and three-enemy test scopes remain distinct. Final lib118/wild-PvP26/UI12/check/fmt pass on fixed protocol6b066803; temporary source pin configuration restored and detached worktree removed afterward.

### Miss-only self outcomes194 — 2026-10-16

[Bounded762/989 source and proof](../../pet-battle-effect-coverage.md#miss-only-self-outcomes194--implementation2026-10-16). The guard inspects only miss events produced during the current turn. Blocked attacks and another pet's earlier miss do not open the branch. The following159 selects the actual caller for existing stun/heal machinery. Stench selectors and generic failure/callback semantics are unchanged. RED2→GREEN8 passes; both-skin private5961 actual self-stun and59/68 healing, ten inspected fullHD originals and canonical strict readback pass. Final integrated lib111/wild-PvP25/UI12/check/fmt pass; exact proof/native heads and source limits live at the linked coverage entry.

### Refuge223 — 2026-10-16

[Bounded1344](../../pet-battle-effect-coverage.md#refuge223--implementation2026-10-16) keeps self26's six-round policy and routes the two other slots only to friendly Critters. A shared explicit-wearer aura function preserves original caster and existing live/charge/refresh semantics; no caster relabeling or shared team shield. Actual unsupported223 RED observed; other selectors and complete Retail timing remain gated. GREEN9/9 and1764/2829 rows/1010 graphs pass. Both-skin private5961 physically blocks caller then Critter reserve independently at892/892, while non-Critter978 takes28/26 damage; strict snapshots/canonical readback pass and eight fullHD originals are inspected. Final CPU gates pending.

### Lock-On77 — 2026-10-17

[Bounded301/517/300](../../pet-battle-effect-coverage.md#lock-on77--implementation2026-10-17) uses a target-worn one-round lock before ordinary direct damage. Existing75/76 self charges retain their wearer/duration; generic aura provenance/refresh unchanged. Unsupported77 RED5→GREEN6, existing Rocket4/Wind-Up1 regressions and1779/2829 rows/1017 graphs pass. Authored Tonk owned/wild-PvP29 and both strict native skins pass first-use enemy lock/no damage, next blast145/143/mark removal; six fullHD originals inspected/all saved phases two framed models. Actual canonical3-mesh loader/4files+fragment hashes pass. Final lib132/check1m02/no warnings/bothfmt pass; prior unchanged UI12 retained. GM722 and cross-caster/full timing/VFX certification excluded.

### Life Exchange67 — 2026-10-17

[Bounded277/493](../../pet-battle-effect-coverage.md#life-exchange67--implementation2026-10-17) directly equalizes active HP from pre-effect values, caps maximum health and preserves reserves/misses. Property67 names Accuracy, not Points; no attack-power scaling or new damage/heal callback policy. Unsupported67 RED3→GREEN4 and1778/2829 rows/1016 graphs; authored Tormented Spirit owned/wild-PvP28 passes. Its strict canonical loader succeeds but Modern rejects its tall scale1.0 model's viewport framing before any cast/capture. No camera/scale workaround; switch legal native subject to Sprite Darter87 level15, whose scale0.3 and complete existing canonical closure are authenticated. Sprite replacement owned test passes1/1; both strict native skins physically produce shared98/108 and subsequent Scratch28/30 leaves70/78, preserving injury across runs. Four fullHD originals inspected, all four saved phases have two framed models; canonical Sprite loader2meshes/6 existing hashes pass. Final e68015ba/shared1911f73e lib126/check1m25/no warnings/bothfmt pass; unaffected wild/PvP27 plus replacement Sprite1/1 and unchanged UI12 proof retained. Integer averaging and feedback/proc classification remain unverified against independent Retail runtime.

### Launch Rocket75 — 2026-10-17

[Bounded293/294](../../pet-battle-effect-coverage.md#launch-rocket75--implementation2026-10-17) admits exact effect508 and its ready-object aura. Existing charged-damage machinery creates the caster's rocket on first use, launches typed damage on second use and consumes the ready aura even on a miss. Real unsupported75 RED3→GREEN4; setup/delayed launch/re-setup, target dodge, three defender families and malformed-tuple rejection covered. Census1777/2829 rows/1015 graphs. Authored level20 Lil' Bling owned-team proof passes in wild/PvP27; both actual strict skins physically set up294 without enemy damage, then launch/expire294 and win at enemy0. Six fullHD originals inspected, two framed models throughout; native capped147 damage is distinct from pure formula proof. Final lib122/check55.43s/bothfmt pass on a1e5346f/shared523d4f95; wild/PvP27 and prior unchanged UI12 scopes retained. Initial duplicate caller `--locked` failed before compilation; corrected check passed, both attempts retained. Object-sharing and full Retail timing/VFX are not certified.

### Limits

Handler references are **not** executable or all-row behavioral coverage. The [matrix](../../pet-battle-effect-coverage.md) records recursive admission separately.45 unsupported referenced IDs (including undefined sentinel0), unknown OverrideIndex/target routing, unadmitted object/special-stat auras and proc events1..4/9/12 (except bounded Leech Life383 and Nevermore/Spell Gulp) remain rejected. Control-duration ordering, exact damage rounding/weak game-table factor and several proc timings remain uncaptured. The separate wild consumer has bounded wire/UI integration; this is not a full Retail parity claim.

## Wild runtime consumer — verified 2026-10-10
`crates/server/src/wild_pet_battles.rs` wraps the immutable turn library with authenticated account loadouts, wild creature claims, seeded RNG, trap decisions and persisted rewards. It reads species metadata/display overrides/visuals and pet XP thresholds already imported into canonical world.db; no reimport is needed for the current branch. [Wild contract](../../specs/wild-pet-battles.md) owns requirements and exact acceptance.

Terminal state must be sent before End on CollectionChannel: otherwise lethal-round health remains at the client's previous snapshot. The production test now uses a Lightyear HostClient sender/receiver and asserts delivered final HP/End, then reopens account storage. Native private5592 proof separately covers actual defeated0HP wins and captures in both explicitly verified skins. Full server workspace passes; no unsupported effect substitution or blanket semantics claim.

## Sources
- [Wild runtime spec](../../specs/wild-pet-battles.md), `crates/server/src/wild_pet_battles_tests.rs` and [native acceptance](../../../../game-engine/docs/specs/wild-pet-battles.md#acceptance--2026-10-10).

- [Contract](../../specs/pet-battle-engine.md), [104-property coverage and reproduction](../../pet-battle-effect-coverage.md).
- Local-CASC DB2 build12.1.0.69933 in game-engine/data/db2; no Blizzard CDN/table downloads.
- Local Retail `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/Blizzard_FrameXML/SharedPetBattleTemplates.lua` AttackBonus/StandardDamage/StandardHealing; `Blizzard_PetBattleUI/Shared/Blizzard_PetBattleUI.lua` speed hints/ability cooldown presentation.
- TrinityCore a352b1fa `src/server/game/BattlePets/BattlePetMgr.cpp`, CalculateStats.
- Blizzard news10055295, **Pet Battle Accuracy Changes in Patch5.3**, official article/search-indexed archived text.
- LegionCore snapshot9222216e0dda0708ae53e6e6f6b07db7d65ba46f `PetBattleAbilityEffect.cpp` opcode/implicit-target names; `PetBattle.cpp` ties; legacy reference only, not copied implementation or Retail proof. Saved in game-engine/data/diagnostics/petbattle-engine-20261010.

## See Also

- [[world-data]] — static SQLite boundary.
- [[toys]] — account collections remain outside this turn library.

### Level-based maximum health — 2026-10-17

Actual caller level1..25 now enters existing Internal_InitialLevel17 through `Pet::new`; owned/wild creation supplies Identity.level, never stat-derived/default level. Only577/610 state2=500 and1990=1000 are admitted. Add raw state2 × actual level /100 before the existing max-health percentage/floor path, preserving current-HP clamp and original base cap at `Encounter::owned_health()`. Aura deltas already handle application, refresh and expiry; no parallel stat or persistence system. [Coverage/source/proof boundary](../../pet-battle-effect-coverage.md#level-based-maximum-health--implementation2026-10-17) owns exact results: gate426 precedes both-skin legal Perky Pug250/7 maximum464→499/Healthy9→464 native proof, with two framed models in every accepted phase and unchanged geometry. All ten fixture tables reconcile to active70448; [fresh census](../../pet-battle-census-70448.json)1797/2829/1023 does not rename historical70391. Independent Retail percentage ordering remains open.

### Starfall and Moon Tears — 2026-10-17

Only258/979 route their first three damage rows across the enemy active/other slots and their next three source-powered heal rows across living allies before existing Moonlight596. Dead reserves stay dead and individual offensive misses do not cancel independent healing. A scoped recipient helper restores the prior healing target, preserving existing family-filtered ally healing; no new team-selector interpreter. [Source and exact proof boundary](../../pet-battle-effect-coverage.md#starfall-and-moon-tears--implementation2026-10-17) distinguishes genuine core/owned RED, passing slot/cap/miss cases and the corrected application-round weather fixture's focused1/1 GREEN (existing timing preserved). Lead gate438 precedes both-skin legal Cosmos1972/1 native258: Rabbit147→138,9 healing×3, both reserve bars increase and Moonlight8. All four saved fullHD phases have two framed models/strict tripwire0 and unchanged geometry. Exact artifact/PNG/proof limits remain in the coverage report;979 native remains unproved.

### Rapid Fire — 2026-10-17

Only774 repeats active/other-enemy recipient order for its second authored three-row wave. Existing typed damage, individual accuracy, dead-pet handling and deferred faint replacement remain unchanged, so a first-wave faint never redirects wave2.2064/OverrideIndex1 remains rejected;920 is separately bounded below. [Source and exact proof boundary](../../pet-battle-effect-coverage.md#rapid-fire--implementation2026-10-17) records real core4/owned1 RED→six named GREEN cases and legal Peddlefeet122/10; integrated69653970 lib150/main34/check23.64s/no warnings/fmt pass and fresh reconciled70448 census1817/2829=64.23%/1026 graphs. Lead gate453 precedes both-skin legal Peddlefeet122/10 native774:27+19/Rabbit101 and36critical+20/Rabbit91, caller653 with ordinary capped recovery. All four fullHD phases have two framed models/strict tripwire0; geometry91/101 only enemy HP fill/timed floats. Coverage report owns exact frozen master-artifact/source/PNG/proof boundaries; native miss/first-wave-faint/enemy reserves remain unproved.

### Magma Wave — 2026-10-17

Only319 joins the existing enemy-team damage registry. Three authored7-point rows use ordinary typed damage and per-recipient accuracy/dead-pet behavior, followed by existing property85 state69 reset removing objects from both teams. No opcode, aura or general routing redesign. [Exact source/proof boundary](../../pet-battle-effect-coverage.md#magma-wave--implementation2026-10-17) records actual core4/owned1 routingRED, all9active/individualmisses/deadreserve/bothteamobjects/allmisscleanup/nonobjectpreservation GREEN, and legalMoltenHatchling428/4power47/15Elemental-v-Critter damage. Integrated5ee64433lib158/main36PASS; fresh reconciled70448 census1828/2829=64.62%/1028graphs (+5/+1/no removals), historical70391/other rejections unchanged. Check/fmt and native-after-landing remain pending.

### Primal Cry — 2026-10-17

Only920 maps its three damage/Speed Reduction154 row pairs to each enemy active/other slot. Its failure chain resets at each damage-row pair start: a miss/dead target skips that pair's slow without suppressing later enemies. Historical failed-OR behavior remains unchanged for every other ability, and774's wave mapping is preserved. Existing aura refresh/clocks and source ownership handle25% slow without stacking. [Exact source/proof boundary](../../pet-battle-effect-coverage.md#primal-cry--implementation2026-10-17) distinguishes unsupported-routing RED, unrelated moving-protocol compile failure, actual chain-scope miss/dead failures and corrected core4/owned1 GREEN. Legal Meadowstomper1446/2; integrated5cd78051 lib154/main35 pass,049a40c6 packagecheck3m08/no warnings/workspacefmt pass, clean shared2a1ed035 before/after. Fresh reconciled70448 census1823/2829=64.44%/1027graphs (+6/+1/no removals); historical70391 unchanged. Gate465 lands154+57 before both-skin legal Meadowstomper1446/2 physical920:28damage/Rabbit119,154slow3rounds/CD3; own211→182→152 after29/30Scratch. Four fullHD phases have two framed models/ExtractedOnly/tripwire0, geometry93/105 only retained fills/timed floats, canonical4hashes+links/1meshloader pass. Coverage report owns exact nativeb87013ad/cf1c4d6f7/a566 schema-alignment/artifact/PNG boundaries; native reserves/misses/refresh/expiry remain unproved, reserve/miss/refresh CPU controls are separate, and full Retail timing/VFX remains uncertified.
