# Windwalker source census (in progress)

Verified2026-10-23. Native DB2/TC precede SimC76f8118; no parity claim. Shared native-book helper belongs to14752. `follower_spec_kit` membership below is **not** proof of native KnownSpells membership; selected reference traits are installed separately by `dps_parity::inputs::install_traits`.

## Missing historical reference channels

| Spell / source | Native data / decision status | Test / gap |
|---|---|---|
| Tiger Palm100780; TC Unit.cpp6869–6891/9973–10000, Player.cpp8180 | Native AP coefficient0.9; castable; coefficient input omits weapon AP | Pushed0cef21dfc genuine RED91vs358–377→native GREEN at4b9d2be73; enabled afterward, parent gate pending |
| Rising Sun Kick107428→185099; DB2 trigger + TC `spell_monk_rising_sun_kick` | Native damage AP coefficient1.43799996; same generic missing weapon AP | Shared fix must cover native child; no fresh qualified damage proof |
| Mastery: Combo Strikes115636; SimC `sc_monk.cpp:142–156,358–373` | Native aura4/coeff2.3287499; named first/ordinary-repeat state and published-kit damage consumer at3286b0463 | Native TP→Provoke→TP without Hit Combo genuine RED→GREEN1/1,10.04s; other actions/channel snapshots remain unqualified |
| Rushing Wind Kick467307→468179; SimC `monk/sc_monk.cpp:1143–1205,1408–1414` | Native trigger64/child AP1.79750001; selected1250566 aura4/60 and1250554 aura332→467307 exist | Missing landed-BOK/active-Combo-Breaker60% producer, RSK reset and1250554 override; missing APL admission/override consumption. Not merely a missing damage definition; no fresh native cast proof |
| Spinning Crane Kick101546→107270; DB2 aura23 trigger | Native channel/child AP0.100000001; published action requires≥3enemies. Selected default ST APL uses Dance/Unbroken Rhythm conditions instead | ST missing APL branch, not absent damage data. Three-target recipient behavior unproven here |
| Dual Threat451823→451839; SimC `sc_monk.cpp:2038–2106` | Driver451823 applied by starter kit and selected talents; native aura4/30, proc mask4/chance100 but no TriggerSpell. Child451839 AP5.81256008 exists | Missing alternating eligible-autoattack replacement producer; TC monk source has no Dual Threat script. Need cited policy qualification, not a fabricated unconditional proc |
| Shado Flurry451250 / ordinary Flurry450617; driver450615; SimC `monk/sc_monk.cpp:663–781,1825,2066,2270,5148–5175` | Native child AP0.519999981/0.600000024;450615 effect0/1 grant1/2 charges for1H/2H,451021 native cap30; selected1262612 supplies25/8 markers | Missing landed-auto/DT charge producer and FoF release (paired children at150ms spacing). No energy-spend heuristic follows from this pinned source. Stand Ready/Wisdom releases separate and unqualified |
| Guillotine1306604; item driver1291728; SimC `unique_gear_midnight.cpp:6326–6374` | Item-proc damage, **not a Monk rotation spell**. Native child is school damage with zero AP/SP/base; SimC supplies item-scaled driver effect1 and missing-health multiplier from effect2 | Item-effect implementation gap; preserve in matched-gear parity ledger, do not tune Monk AP to absorb it |

## Named policy pending qualification

`WW_COMBO_STRIKES_SIMC_POLICY`: pinned TC entire `src/` has no115636/Combo Strikes binding or generic special case. Aura4 handler `SpellAuraEffects.cpp:4836+` supplies no115636 behavior. Native115636 effect0 supplies2.3287499 per raw mastery point, but no applicability mask, trigger or damage modifier. SimC `sc_monk.cpp:142–156` applies its native mastery value to explicitly mastery-eligible actions while Combo Strikes is active; activation/order must be ported from its358–373 source. Keep values DB2-owned; cite any missing eligibility/activation as SimC policy. Ignored `dps_parity_windwalker_combo_strikes_first_cast_and_repeat_without_hit_combo` follows the genuine weapon-AP RED/GREEN: level10 native Bo, actual Tiger Palm→Provoke→Tiger Palm admission, native raw mastery/coefficient, exact production pre-crit/pre-defense amount comparison. It asserts first-action mastery without learned Hit Combo, utility preservation, and repeat removal without rerolling legal hit defenses. Genuine runtime RED at40447ae57: first83.754=repeated83.754; native18.63% requires99.35737,11.13s/exit101. Separate implementation adds ordinary-success state independent of Hit Combo and native raw-points/coefficient damage consumption for the published kit's damage IDs. At3286b0463 the same ignored test passes1/1 in10.04s (compile6m50s),exit0,zero compiler warnings; enabled afterward. Only Tiger Palm first/utility/repeat is qualified, not full per-ability/channel snapshot behavior.

## Literal APL boundary

The frozen DB actually orders Zenith→SCK(≥3)→BOK→RSK→FoF→ToD→SotW→TP, with no no-repeat/resource predicates on attack rows (same-priority attacks sort by spell ID). Its109 exact source rows are preserved in`native-before-ww-policy/269.frozen-kit-rows.json`; BOK204/205 casts starve later RSK/FoF. A previously described newer published priority/no-repeat policy does not describe this frozen input. This is not the default APL embedded in `profiles/MID2/MID2_Monk_Windwalker.simc` (`default_st`, `multitarget`, `opener`, `zenith`, `fallback`, racials/items/external buffs). No current literal-port claim follows from positive casts or DPS.

Selected trait export has79active entries:78 spell definitions and one selection node101232/entry125046 (TraitDefinitionID0, subtree65). The selection node is not an absent native spell. Full per-selected-spell execution matrix remains open.

## Frozen native baseline before WW policy

At40447ae57 (weapon AP plus Mutilate regression fix and simulator pet-plugin fix), read-only frozen DB21ba10a0… and300s native dummy31146: ST23785.6633 vs206443.72 (−88.4784%);3T23260.4667 vs376559.68 (−93.8229%). Both sustained-clock assertions pass17.8s each. ST casts TP46/BOK204/ToD1/SotW1/Zenith5; FoF/RSK/SCK/RWK absent. Damage includes native SotW children once. No literal APL/native book completeness claim; missing consumables, race, Omnium and three redirected-base-stat gear options remain explicit report gaps. Immutable native1/3 receipts and binary/command provenance live underengine diagnostics`native-before-ww-policy/`.

## Native energy-regeneration divergence

Frozen WW publishes11Energy/s. Native`spell_cast/powers.rs` applies DB2 base rates, aura percentage and flat terms; pinned TC`Entities/Unit/StatSystem.cpp850–916` likewise applies no haste and explicitly leaves haste-to-power-regeneration update as a TODO. Preserve TC precedence: lower Tiger Palm cadence alone is not proof of a missing native haste multiplier. No energy tuning or new haste policy approved.

## Post-Combo native receipt

Production3286b0463, same immutable frozen DB/gear/traits/source view: ST30118.0967 (−85.4110%),3T29369.6333 (−92.2005%). Both300s sustained assertions pass. Immutable outputs and exact commands/binary SHA live inengine diagnostics`native-after-combo-strikes/`. Test activation9fe73189e changes no body or production behavior; no redundant rerun.

## Literal missing-cooldown policy

Lead approved `SIMC_MISSING_COOLDOWN_PLACEHOLDER` for exact WW tokens only: `cooldown.whirling_dragon.remains`=0 (never started) and `cooldown.invoke_power_infusion_0.duration`=`timespan::min` in seconds when no external PI cooldown exists. Pinned`player/player.cpp12257–12269` creates those placeholder objects with duration/base_duration=`timespan::min`; `sim/cooldown.hpp113–114` starts readiness at−3600s. Do not correct the typo to`whirling_dragon_punch`, and never turn arbitrary unknown operands into zero. Generic native bindings remain strict. Wrapper implementation/behavior proof pending literal-driver RED admission.

## Source inventory artifact

Engine `data/diagnostics/ww-parity-2026-10-23/269-source-census-v1.json` (SHA256 `6e8464b9c0a16f4aba97199036e9bf93424775dbdca6ac6f1d4e9811c675b6af`) preserves108 IDs:78 selected spell definitions union IDs from the frozen ST reference action report. Each row contains raw DB2 effects/coefficients, selected entry identity and published kit intervals; all current runtime statuses are explicitly `UNPROVEN_CURRENT_REVISION`. This inventory does not cover every unreachable default-APL action and is not the final execution matrix. New`269-apl-source-census-v1.json` enumerates132literal rows/10lists/33actions/82operands with every binding UNPROVEN. `269-missing-channels-census-v2.json` (SHAc83af19f5841e439d5fd27cd2b917c3f73e9bc766eb3365f1a96b9c42942ff5d) preserves exact native rows plus pinned producer/release source anchors for RWK, Dual Threat, SCK and Shado Flurry; it is source evidence, not runtime qualification. `269-apl-trait-bindings-source-v1.json` resolves all20APL talent aliases through pinned generated trait entries, with each native TraitDefinition/SpellID identity matching. `269-apl-buff-bindings-source-v1.json` resolves all14buff aliases to native identities/source anchors. `269-item-apl-source-census-v1.json` records actual equipped item effects:270175 has on-use1297761 (20s category/90s recovery), whose zero MOD_STAT bases require1297760's item-scaled values;270173 is passive Guillotine. Neither item producer is qualified, and their missing behavior must not be absorbed into Monk coefficients.

## Sources

- Native70448 `spell_effect`, `spell_aura_options`, `trait_node_entry`, `trait_definition`, `follower_spec_kit` read-only SQL; canonical database unchanged.
- Pinned public TC `src/server/scripts/Spells/spell_monk.cpp`, SHA256 `7f85246ca7e0a773e88370c21b7c207537068678d50417c7d4e5a0394a507a92`.
- Frozen SimC76f8118 checkout `/home/osso/.worktrees/simc-76f8118`.
- [Weapon AP proof ledger](ww-weapon-ap-proof-ledger.md).
