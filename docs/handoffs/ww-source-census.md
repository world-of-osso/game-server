# Windwalker source census (in progress)

Verified2026-10-23. Native DB2/TC precede SimC76f8118; no parity claim. Shared native-book helper belongs to14752. `follower_spec_kit` membership below is **not** proof of native KnownSpells membership; selected reference traits are installed separately by `dps_parity::inputs::install_traits`.

## Missing historical reference channels

| Spell / source | Native data / decision status | Test / gap |
|---|---|---|
| Tiger Palm100780; TC Unit.cpp6869–6891/9973–10000, Player.cpp8180 | Native AP coefficient0.9; castable; coefficient input omits weapon AP | Pushed0cef21dfc genuine RED91vs358–377→native GREEN at4b9d2be73; enabled afterward, parent gate pending |
| Rising Sun Kick107428→185099; DB2 trigger + TC `spell_monk_rising_sun_kick` | Native damage AP coefficient1.43799996; same generic missing weapon AP | Shared fix must cover native child; no fresh qualified damage proof |
| Mastery: Combo Strikes115636; SimC `sc_monk.cpp:142–156,358–373` | Native aura4/coeff2.3287499, no damage multiplier or child trigger. Native `windwalker.rs` contains Hit Combo/Combo Breaker/Dance, not Combo Strikes | Spec-specific missing scripted-policy candidate; full TC monk source has no Combo Strikes script; not yet implemented or behavior-proven |
| Rushing Wind Kick467307→468179; SimC `sc_monk.cpp:5838–5839` | Native trigger64 and child school damage/AP1.79750001 exist. Selected talent1250566 is aura4/60;467307 absent from published spec269 action rows | Missing APL action/book-family binding; need exact acquired native book after14752 helper. No fresh cast proof |
| Spinning Crane Kick101546→107270; DB2 aura23 trigger | Native channel/child AP0.100000001; published action requires≥3enemies. Selected default ST APL uses Dance/Unbroken Rhythm conditions instead | ST missing APL branch, not absent damage data. Three-target recipient behavior unproven here |
| Dual Threat451823→451839; SimC `sc_monk.cpp:2038–2106` | Driver451823 applied by starter kit and selected talents; native aura4/30, proc mask4/chance100 but no TriggerSpell. Child451839 AP5.81256008 exists | Missing alternating eligible-autoattack replacement producer; TC monk source has no Dual Threat script. Need cited policy qualification, not a fabricated unconditional proc |
| Shado Flurry451250 / ordinary Flurry450617; driver450615; SimC `sc_monk.cpp:663–781,5156–5166` | Native school-damage child coefficients0.519999981/0.600000024 exist. Driver450615 is aura4; no native trigger child. Selected Shado1262612 has native25/8 dummy points | Missing charge/energy producer and release policy; no native runtime proof, no heuristic substitute |
| Guillotine1306604; item driver1291728; SimC `unique_gear_midnight.cpp:6326–6374` | Item-proc damage, **not a Monk rotation spell**. Native child is school damage with zero AP/SP/base; SimC supplies item-scaled driver effect1 and missing-health multiplier from effect2 | Item-effect implementation gap; preserve in matched-gear parity ledger, do not tune Monk AP to absorb it |

## Named policy pending qualification

`WW_COMBO_STRIKES_SIMC_POLICY`: pinned TC entire `src/` has no115636/Combo Strikes binding or generic special case. Aura4 handler `SpellAuraEffects.cpp:4836+` supplies no115636 behavior. Native115636 effect0 supplies2.3287499 per raw mastery point, but no applicability mask, trigger or damage modifier. SimC `sc_monk.cpp:142–156` applies its native mastery value to explicitly mastery-eligible actions while Combo Strikes is active; activation/order must be ported from its358–373 source. Keep values DB2-owned; cite any missing eligibility/activation as SimC policy. Ignored `dps_parity_windwalker_combo_strikes_first_cast_and_repeat_without_hit_combo` follows the genuine weapon-AP RED/GREEN: level10 native Bo, actual Tiger Palm→Provoke→Tiger Palm admission, native raw mastery/coefficient, exact production pre-crit/pre-defense amount comparison. It asserts first-action mastery without learned Hit Combo, utility preservation, and repeat removal without rerolling legal hit defenses. Genuine runtime RED at40447ae57: first83.754=repeated83.754; native18.63% requires99.35737,11.13s/exit101. Separate implementation adds ordinary-success state independent of Hit Combo and native raw-points/coefficient damage consumption for the published kit's damage IDs. GREEN pending; only Tiger Palm first/utility/repeat is currently under test, not full per-ability qualification.

## Literal APL boundary

Published kit is a short priority list: Zenith→FoF→Strike of the Windlord→RSK→Touch of Death→SCK(≥3)→Blackout Kick→Tiger Palm(Chi deficit≥2), with a no-repeat group. This is not the default APL embedded in `profiles/MID2/MID2_Monk_Windwalker.simc` (`default_st`, `multitarget`, `opener`, `zenith`, `fallback`, racials/items/external buffs). No current literal-port claim follows from positive casts or DPS.

Selected trait export has79active entries:78 spell definitions and one selection node101232/entry125046 (TraitDefinitionID0, subtree65). The selection node is not an absent native spell. Full per-selected-spell execution matrix remains open.

## Frozen native baseline before WW policy

At40447ae57 (weapon AP plus Mutilate regression fix and simulator pet-plugin fix), read-only frozen DB21ba10a0… and300s native dummy31146: ST23785.6633 vs206443.72 (−88.4784%);3T23260.4667 vs376559.68 (−93.8229%). Both sustained-clock assertions pass17.8s each. ST casts TP46/BOK204/ToD1/SotW1/Zenith5; FoF/RSK/SCK/RWK absent. Damage includes native SotW children once. No literal APL/native book completeness claim; missing consumables, race, Omnium and three redirected-base-stat gear options remain explicit report gaps. Immutable native1/3 receipts and binary/command provenance live underengine diagnostics`native-before-ww-policy/`.

## Source inventory artifact

Engine `data/diagnostics/ww-parity-2026-10-23/269-source-census-v1.json` (SHA256 `6e8464b9c0a16f4aba97199036e9bf93424775dbdca6ac6f1d4e9811c675b6af`) preserves108 IDs:78 selected spell definitions union IDs from the frozen ST reference action report. Each row contains raw DB2 effects/coefficients, selected entry identity and published kit intervals; all current runtime statuses are explicitly `UNPROVEN_CURRENT_REVISION`. This inventory does not cover every unreachable default-APL action and is not the final execution matrix.

## Sources

- Native70448 `spell_effect`, `spell_aura_options`, `trait_node_entry`, `trait_definition`, `follower_spec_kit` read-only SQL; canonical database unchanged.
- Pinned public TC `src/server/scripts/Spells/spell_monk.cpp`, SHA256 `7f85246ca7e0a773e88370c21b7c207537068678d50417c7d4e5a0394a507a92`.
- Frozen SimC76f8118 checkout `/home/osso/.worktrees/simc-76f8118`.
- [Weapon AP proof ledger](ww-weapon-ap-proof-ledger.md).
