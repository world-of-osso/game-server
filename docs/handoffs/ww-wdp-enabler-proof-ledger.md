# Native WDP enabler producer

Current child dispatch and post-rebase300s results are in the [WDP damage-child ledger](ww-wdp-damage-children-proof-ledger.md). Measurements below retain their historical enabler/duration-only revisions; later child execution supersedes the former missing-child observation.

## Current approved duration override — 2026-10-24

`SIMC_WW_WDP_ENABLER_DURATION` supersedes the initial base-duration-only policy below. DB2196742 DurationIndex36 remains1000ms in the native store. Only the existing selected-owner producer now supplies a per-trigger duration snapshot: minimum actual RSK/FoF remaining cooldown plus1500ms multiplied by native melee-haste time multiplier. SimC integer timespan multiplication truncates the scaled grace before addition (`engine/util/timespan.hpp132–135`); native cooldown seconds are converted back to their millisecond clock. Nonpositive overlap does not produce; an already active owned enabler does not refresh (`NONE`). No152175 admission/gate changes.

Pinned SimC76f8118 `engine/class_modules/monk/sc_monk.cpp4552–4572`, verbatim:

```cpp
struct whirling_dragon_punch_buff_t : monk_buff_t<>
{
  whirling_dragon_punch_buff_t( monk_t *player )
    : monk_buff_t( player, "whirling_dragon_punch", player->talent.windwalker.whirling_dragon_punch_buff )
  {
    // current measured value for grace period
    // partial testing as of 01/08/2024 dd/mm/yy
    base_buff_duration = 1500_ms;
    set_refresh_behavior( buff_refresh_behavior::NONE );
  }

  bool trigger( int, double, double, timespan_t ) override
  {
    timespan_t buff_duration =
        std::min( p().cooldown.rising_sun_kick->remains(), p().cooldown.fists_of_fury->remains() );

    if ( buff_duration > 0_ms )
      return monk_buff_t::trigger( -1, DEFAULT_VALUE(), -1.0,
                                   ( base_buff_duration * p().composite_melee_haste() ) + buff_duration );

    return false;
```

Before override, first real window at88732aa80 implies7.95s creation/8.95s expiry. All active samples8.00–8.90 have native GCD remaining equal to enabler remaining (.95→.05), WDP known/resolved/CD0, no active CastState and pinned STrow43 condition true; both enemy/self raw admissions return OnGlobalCooldown. At8.95 GCD0 but aura expired, raw native CasterAuraState; actual chosen action is BOK. Post-GCD target/cost checks were masked, not claimed passed. Exact trace and compact timeline: `native-ww-wdp-first-window-88732aa80/`.

Native duration test dd50178b1 genuinely fails10.15s: actual1000 vs source-derived7265ms, after actual native overlapping cooldowns/derived haste. Artifact `native-ww-wdp-duration-red-dd50178b1/`. Existing positive producer test retains native cast/overlap assertions but now captures first duration>1000 and asserts native store base remains1000; the new test owns exact source arithmetic. This is an explicit assertion adaptation to the newly approved requirement, not unchanged old1000ms qualification. At5618c1eea, zero-warning admitted -j2 build461.87s; same-binary five native cases GREEN29.45s (duration formula, adapted positive/store-base, both prior negative controls, first-window trace). Production/shared native gates remain unchanged outside the existing producer.

| First-window boundary | Native observation / proof |
|---|---|
| Duration | Actual7265ms =6318ms remaining-RSK minimum +947ms truncated hasted grace; first observed8.00, expiry15.215, implied creation7.95 |
| First free GCD8.95 | Enabler still6.265s; WDP cooldown0, both raw enemy/self admissions ready, Prepared/row gate true, STrow43 condition true; actual source dispatches152175 |
| Later refusals | After that successful cast, native OnCooldown; during later FoF channel native InProgress; after aura expiry CasterAuraState. No masked post-GCD target/cost refusal appears at first dispatch |
| Refresh | Aura143 retains duration7265/expiry15.215 through145 active-frame samples, including next actual source FoF13.95; no refresh observed |
| Numerical / damage exclusion | Actual300s below: root casts now8ST/3three-target, but root damage0 and source damage children158221/451767 absent. Duration/admission qualification, not WDP damage qualification |

Exact CURRENT-backup/input/binary/runtime/trace receipts: `native-ww-wdp-duration-green-5618c1eea/`; compact `first-window-summary.json` supplements the full trace. Fresh CURRENT backup before EACH same-binary300s: ST97359.4833 vs206443.72 (−52.8397%), three-target105538.6233 vs376559.68 (−71.9729%), exit0 and300sim seconds. Runtime27.92s/30.96s (wrapper28.20s/31.12s). Enabler uptime61.9833%/9.3667%; source-native152175 CastSuccess counts8/3. Exact per-measurement SHA/proofs and comparison: `native-after-wdp-duration-5618c1eea/`. Both remain outside±10%; no isolated causal DPS claim.

## Initial base-duration-only qualification (historical)

Approved2026-10-24 policy: `SIMC_WW_WDP_ENABLER_PRODUCER`. Scope: actual RSK107428 or FoF113656 root landing while both native cooldowns remain>0 applies native196742 via generic aura path. Selected WDP owner's source buff eligibility only. Duration comes from CURRENT DB2 (1000ms); no152175 special case or native CasterAuraSpell gate change.

## Authority / absence

Pinned TC a352b1fa spell_monk.cpp has no Whirling Dragon Punch/152175/196742 binding. CURRENT native152175 requires CasterAuraSpell196742;196742 effect0 aura226 has no TriggerSpell. TDB spell_proc has no152175/196742 rows. Existingf2ST/3T receipts have no196742 uptime or152175casts. Raw read-only data/receipt packet: engine `ww-parity-2026-10-23/wdp-native-enabler-source-packet.json`.

SimC76f8118 sc_monk.cpp1138 (RSK execute),1826 (FoF execute),4552–4572 (WDP buff) supplies missing producer/overlapping-CD eligibility. Buff creation at6416–6417 is conditional on selected WDP talent. `SIMC_WW_WDP_GRACE_DURATION_DIFF`: SimC's manually measured1500ms grace plus minimum remaining cooldown differs from DB2 DurationIndex36/1000ms. DB2 wins: apply its generic1000ms aura only, no copied1500 or extended CD-derived timer.

## RED/GREEN scope

Ignored `literal_ww_native_selected_wdp_earns_its_native_cooldown_window` runs the actual selected269 literal fixture, requires real native RSK/FoF CastSuccess and overlapping native cooldowns, then asserts earned owned196742 duration1000. No aura/cast/power/cooldown grants. Atd174 zero-warning -j2 build555.29s; initial runtime fails17.92s before producer assertion because merged native TTD now requires explicit FixtureTarget lifetime/role. This is setup failure, not RED. Fixture now installs the same declared lifetimes as the measurement caller before decisions; no cast/aura/power/CD changes. Atc4f2e3212 fixed-lifetime native case reaches genuine RED14.46s/exit101: selected owner, actual RSK/FoF successes and both positive cooldowns pass; owned196742 duration is None instead of Some1000. Zero-warning -j2 warm build30.83s; CURRENT private backup/input/binary/archive receipts at `native-ww-wdp-enabler-red-c4f2e3212/`. Producer implementation follows only this assertion RED. Damage152175/children/recipients are outside this enabler-only change; numerical band remains open. Fresh CURRENT-backup EACH300s ST/3T required after qualification.

## Implementation boundary

Separate `ww-wdp-enabler` branch atop c4; `ww-monk-parity` remains pinnedc4 for ww1081 gate. Existing root OnHit hooks107428/113656 test actual selected152175 owner and native cooldown readers for both>0, then cast native196742 through the generic triggered aura path. No152175 hook, gate override, forced cooldown state or explicit duration assignment. `SIMC_WW_WDP_GRACE_DURATION_DIFF` remains1000ms native vs SimC measured1500ms plus remaining-CD extension. Added actual landed-RSK/FoF-ready and unselected-owner/overlapping-CD negative controls. At013 native2GREEN/1setupFAIL15.80s after zero-warning402.82s compile: positive earns actual1000ms and actual landed-RSK/FoF-ready negative passes. Removing WDP's point invalidated later native20-point gates, so the unselected case correctly stops before its producer assertion. Native node101207 is a real choice between125014/392983 Windlord and134452/152175 WDP; declare the other choice with the same point spend before native progression, never bypass validation. Ate072282c0 corrected native alternate-choice/unselected-owner case passes14.99s; zero-warning -j2 warm build36.84s. Production code b275 is unchanged between013's two passing cases ande072's corrected case; three unique boundaries qualified across those binaries, not a claim of a new3case same-binary rerun. Exact artifacts `native-ww-wdp-enabler-green-013169adb/` and `native-ww-wdp-control-green-e072282c0/`.

## First-window diagnostic request (historical, before duration approval)

Parent next scope: trace the first actual196742 window against every unmodified pinned WDP source row, controller knowledge/resolve, Prepared readiness/row gates, raw native enemy/self admissions, actual cooldown/GCD/CastState/Moving and chosen source action per frame. Test-only `wdp_window_tests` follows exact production cancel→sample→select once→dispatch→App update ordering; diagnostics never cast WDP or waive a gate. Native1000ms stays unchanged; if its overlap alone explains the gap, parent owns policy decision. No native generic cause asserted before trace.

## Fresh integrated measurements

Samee072 binary, a separate fresh CURRENT canonical backup before EACH actual300s measurement: ST89384.0867 vs206443.72 (−56.7029%);3T106899.63 vs376559.68 (−71.6115%), both exit0/300sim seconds. Runtime32.13s/29.27s. Exact source/copy SHA/input/binary/command receipts: `native-after-wdp-enabler-e072282c0/`. Native196742 uptime15%ST/1.3333%three-target; no152175 casts or WDP damage rows. This proves the enabler boundary only, not native WDP dispatch/damage or numerical parity. Other integration changes since f2 mean no isolated enabler-only DPS causal claim.
