# Feral source census (in progress)

Verified2026-10-23. Follower Ralia Dreamchaser: entry232800/spec103/class11/form768. Native DB2 and public TrinityCore a352b1fa precede SimC76f8118. No current parity claim.

## Frozen evidence

Private SQLite backup of engine `data/server-world/world.db`: SHA256 `51455391e7d4962589cc66d239bf77b2dbf3f4372ec8daee80c092f1e7989d9b`. Canonical database is never written. Evidence directory: engine `data/diagnostics/feral-parity-2026-10-23/`; `db2-spell-census.json` retains full35spell effects/auras/powers/PPM rows, `oracle-factor-census.json` retains printed factors and attribution, `tdb-druid-script-bindings.json` and `tdb-feral-proc-rows.json` retain matching SQL rows. This census is not proof of actual execution.

Authoritative references in engine `data/diagnostics/dps-parity-2026-10-21/`: `103.reference.single.*` ST191666.531726 DPS; `103.reference-fixed.three.*` three actual recipients247748.824618 DPS. Their ±10% bands are172499.878553–210833.184899 and222973.942157–272523.707080. Unsuffixed `103.reference.three.*` is explicitly invalid: Patchwerk clears the authored adds event. Both valid references use300s fixed time, same MID2 profile, optimal_raid=0 and seed70448. Both references reproduced exactly in this evidence directory. Read-only exact-master9d13ac19c native300s baseline: ST56378.653333 (−70.5850%),3T62475.256667 (−74.7828%); each sustained-clock test PASS. `source-view.proof.json` records the passing startup-loader inode proof; `baseline-master.{1,3}.proof.json` records exact binary/hash/environment and bubblewrap mount view.

## Channel/source coverage

| Reference ST channel | Native data / source | Qualification / gap |
|---|---|---|
| Ferocious Bite22568,15.64% | Native effect0 AP1.452, effect1PowerBurn62/max25; base Energy25 then up to4optional CP beyond required1. TDB binds `spell_dru_ferocious_bite`; TC druid974–1018 scales effect0 by1+min(post-cost Energy,maxExtra)/maxExtra, then burns extra Energy. Incarnation102543 effect1 only adjusts maxExtra if an actual aura effect exists. | No native Bite damage hook found; generic PowerBurn consumes resources but cannot supply this damage factor. Exact ordinary launch→damage→burn behavioral RED committed, not yet executed. No fabricated damage scalar. |
| Rip1079,13.94% | Native periodic effect0 AP0.3113, matching oracle printed a_mod. Native `auras/refresh.rs` already handles Rip pandemic. TC druid2080–2114 binds a legacy Player-only amount addition of1%AP×CP plus optional idol amounts. | Source divergence: Ralia retains Follower/NPC identity, not Player; do not port legacy Player-only addition onto follower or replace native coefficient. CP snapshot/duration and real periodic damage still need native proof. |
| Unseen Slash1263890,10.41% | Oracle actual formula1263890/AP0.657; parent Unseen Predator1263660 has real label218 modifiers and proc trigger1263962. | No native1263890 dispatch found. Follow trigger chain/DB2/TDB before any SimC-only dispatch policy; no unconditional fabricated proc. |
| Rake1822→155722,10.19% | Native initial AP0.291 matches oracle; effect2 triggers155722. TDB `spell_dru_rake`, TC druid2040–2077 captures stealth before impact. Existing `scripts/batch7.rs` and `auras/scripts.rs` freeze named bleed stealth bonus; `auras/spell_mods.rs` separately snapshots Tiger's Fury5217 periodic modifier. | Existing mechanism, not a new missing snapshot claim. `dummy12_rake_bleed_snapshots_stealth_bonus_across_later_rank_removal` already covers rank+stealth removal. Fresh matched-profile factors/periodic crit remain unqualified. |
| Feral Frenzy274837→274838,7.83% | Native parent Dummy5/aura226 with no child trigger; child274838 directAP0.163 and periodicAP0.3267 match oracle. No TC Feral Frenzy script or TDB binding found. | Native parent has no dispatch script. A named SimC scheduling/child policy is needed only after proving no structured producer; DB2 retains coefficients/timing data. |

Other major ST shares: Shred5221 7.14%, cat melee6.56%, Bloodseeker Vines439531 6.54%, Lunar Inspiration155625 5.51%, item Guillotine1306604 4.21%. Three-target top channels: Rip23.89%, Rake14.33%, Lunar Inspiration7.98%, Vines7.71%, Bursting Growth6.60%. Do not absorb missing channels into coefficients.

Mastery77493 already uses generic native Attr8/raw mastery scaling for followers (`auras/mastery.rs`); missing literal rotation does not establish a mastery defect. The35spell census has one TDB proc override,48484/Infected Wounds; newer Feral proc chains need an expanded child census rather than an assumption that absent rows mean zero probability.

## Baseline decomposition

`baseline-master-per-channel.json` combines native Rake1822+155722 and reference same-ID Convoke variants without dropping damage. ST native/reference channel damage deltas: Bite−77.2%, Rip−68.5%, Rake−69.3%, Shred+53.7%, Frenzy/Unseen/Vines/Moonfire all−100%. Three-target Rip−89.1%, Rake−98.6%, Swipe+364.6%. Overperforming filler alongside absent channels rules out an undifferentiated global multiplier.

`per-slot-base-stat-diff.json` reconstructs the native source formula using ItemSparse, ItemBonus, explicit item levels, RandPropPoints and the actual generated multiplier tables. All10nonredirect slots match reference gear. Five ignored `redirected_base_stats` options explain base rating deltas(native−reference): head271528→271875 haste−62/vers+62; shoulders271526→268246 crit+5/mastery−5; chest271531→268235 crit+131/haste−69/mastery−62; hands271529→268234 vers+106/crit−48/mastery−58; legs271527→268225 haste+74/crit+69/mastery−143. Total crit+157/haste−57/mastery−268/vers+168. Shared equipment owner has the evidence; no Feral coefficient correction follows.

Native base ratings1076crit/738haste/959mastery/275vers plus7crit/7haste/32mastery gems exactly reconstruct the observed1083/745/991/275. Reference base919/795/1227/107 plus23/23/197/0 gives942/818/1424/107. Remaining consumable gap16crit/16haste/165mastery is routed to the shared consumable owner. Mastery rating mismatch is not a base-mastery defect.

The native fixture casts Incarnation102543 three times despite its absence from the exact selected Feral reference traits. [Generic action-knowledge census](action-knowledge-census.md) records all11profiles before enabling the controller knowledge fix; visible/parent bridges must remain admissible.

## Literal boundary

The exact pinned profile requires target_if Boolean selectors and action-relative dot/snapshot facts, active_dot, hero_tree, previous-GCD facts, pool_resource/for_next, max_energy and use_items/off-GCD options. Shared owners receive these requests through the lead; Feral owns identity bindings, Berserk/Incarnation aliases and snapshot source attribution. Dungeon rotation is unchanged; only explicit parity Fixture encounters may capture native follower decisions.

SimC `sc_druid.cpp11216–11323` maps clearcasting to native135700, Apex proc to391882 and Sudden Ambush to1244483 through selected talent triggers; `13595–13617` maps bs_inc to Incarnation when selected, otherwise Berserk. Native TraitDefinition overrides, not guessed action replacement IDs, remain authoritative.

## Sources

- Public pinned SimC `engine/class_modules/sc_druid.cpp`, `profiles/MID2/MID2_Druid_Feral.simc`; profile SHA2569f419caddee5e8cae94856a05b27bd276b390c6ac6c83d4d6449b1eccd093797.
- Public pinned TC `/home/osso/.worktrees/logs/achievements-criteria-2026-10-12/tc-source-a352b1fa/TrinityCore-a352b1fa/src/server/scripts/Spells/spell_druid.cpp`.
- TDB `spell_proc.sql`/`spell_script_names.sql` from engine `data/diagnostics/spell-dummy7-2026-10-10/`.
- [Spell oracle](../wiki/reference/simc-spell-oracle.md), [shared literal machinery](../wiki/systems/simc-apl.md), [DPS parity contract](../specs/dps-parity.md).
