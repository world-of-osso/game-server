# WDP damage-child relationship

Approved2026-10-24: `SIMC_WW_WDP_DAMAGE_CHILDREN`, after duration/admission qualification in the [enabler ledger](ww-wdp-enabler-proof-ledger.md). Scope: once-per-cast451767 at the actual primary target, and158221 at each native effect0 PERIODIC_DUMMY226 tick. Native children own damage coefficients, target cap/reduced-AoE and geometry. No152175 special case in core, no native permission/CD/power grants.

## Source order / absence

CURRENT backup `native-after-wdp-duration-5618c1eea/world.current.1.db` (source/copy SHA in adjacent receipts):152175 effect0 applies aura226, period250ms, TriggerSpell0; effect1=144, TriggerSpell0. Native root duration1000ms. No native trigger/periodic-trigger child edge; a whole-table incoming-trigger scan for152175/158221/451767 is empty. Native children exist:158221 school damage/AP3.1394999,451767 school damage/AP4.5. These amounts are not copied from SimC.

Full local TDB1210 world dump parsed with repository `mysql_dump.py`:4066 `spell_script_names`,220 `spell_linked_spell`,1283 `spell_proc` rows; zero signed-ID matches for root/children in the relevant binding keys. Entire available pinned TC a352b1fa `src` searched with gitignore disabled for IDs and WDP names: exit1, empty stdout/stderr. TC `SpellAuraEffects.cpp1324–1326` leaves PERIODIC_DUMMY to scripts. This is a missing scripted relationship, not evidence that generic DB2 trigger dispatch lost an edge.

Raw rows, TDB archive SHA/full-table counts, TC search receipt and verbatim pinned source: engine diagnostics `ww-parity-2026-10-23/wdp-damage-children-source-packet.json` (and sibling `wdp-damage-children-{native-source,tdb-absence,tc-absence}.json`). Source packet predates the timing decision below; its proposed SimC tick schedule is explicitly superseded, not implemented.

## Pinned source and approved timing difference

SimC76f8118 `engine/class_modules/monk/sc_monk.cpp5824–5827` identifies158221 AoE and451767 single-target children. Lines1888–1894 construct those children;1897–1906 execute the single-target child immediately and manually schedule the AoE child at `i * base_tick_time` while `i <= dot_duration / base_tick_time - 2.0`.

`SIMC_WDP_TICK_SCHEDULE_DIFFERS`: do NOT reproduce that source's0/250/500ms schedule. Parent explicitly chooses actual native aura226 ticks. CURRENT152175 also has Attributes5=512 (`EXTRA_INITIAL_PERIOD`), so P250/D1000 actually schedules0/250/500/750/1000ms including expiry; initial due0 is paid at the next native update (+50ms in this fixture). No manually scheduled class timer or deletion of the authored initial tick. `AuraScripts.on_periodic` is the existing generic periodic-dummy→class-relationship seam. Existing `scripts::on_hit_with_context` is the approved cast-child route; no new cast hook.

## Native RED and shared primary context

At41df5c914 native2RED19.03s/zero-warning407.80s -j2build: actual root CastSuccess/primary/self-aura/P250/D1000 prerequisites pass. Immediate child has0 deliveries vs1; periodic child has[] vs[250,500,750,1000]. Full receipts `native-ww-wdp-children-red-41df5c914/`. Root explicit primary4294967223 differs from its actual aura/hit unit4294967224 at8.95s; the current context hook receives the latter and had no retained original primary.

Parent explicitly approves retaining the request primary in the existing context, not a new hook. Per-cast `SpellInfo.cast_primary_target` stores the original request once at ordinary/costed-trigger/FULL_MASK snapshot boundaries; existing `EffectCtx.primary_target()` exposes it read-only through the same on_hit_with_context signature. No CombatTarget read, target substitution, data mutation or admission change. Existing consumers must be qualified unchanged, including Ret Toll on its source branch. Shared context18eb96bb0 is pushed separately for unchanged existing-consumer qualification; Ret Toll is on its own stack, not imported into this WDP branch.

## Relationship implementation and scoped native GREEN

Class-only WDP registration supplies native158221 through existing `AuraScripts.on_periodic`; each effect0 callback uses normal triggered dispatch at the native aura unit/caster destination. Existing `scripts::on_hit_with_context` registers the root self hit and dispatches native451767 at `ctx.primary_target()` once. No new hook, mutable CombatTarget read, class ID in core timing/dispatch or target/coefficient override. All child target caps/reduced-AoE stay in their existing native path; SimC root-base5 is not copied. Existing Combo Strikes/Hit Combo registries are not expanded by this dispatch-only change; their WDP-specific eligibility/numerical qualification remains separate. Atde1 admitted zero-warning build592.49s; native six PASS/one AoE assertion FAIL27.35s. ST child and five prior WDP controls pass; actual AoE already delivers at sampled[50,250,500,750,1000] while the new fixture incorrectly expected four deliveries. Checked DB2 Attributes5=512 and native `build_effect` index0 initialization before any remedy: this is authored native scheduling, not an extra class callback. Test-only correction adds the initial flag prerequisite and first native frame delay to expected times. Existing names and recipient/damage/expiry assertions stay; expected schedule is explicitly adapted, not claimed unchanged. No production change fromde1. Original missing-child RED remains recorded as[] vs the initial four-time expectation; native dispatch absence was real, while the post-implementation full schedule exposed this test-model omission.

Ret primary-context consumers are independently qualified unchanged:18eb cherry-picked on development8b520b865 as throwawaye293f7f64, `dps_parity_ret_toll`8PASS93.81s/zero warnings. Exact log/proof under engine `ret-parity-2026-10-23/ret-toll-primary-context-18eb96bb0-once-v1.*` read and confirmed. Atb8b4adc0c, corrected native7/7GREEN26.77s after zero-warning57.99s warm build; production remainsde1. Parent then required rebase onto51a28da8a: all12 commits replayed cleanly, every range-diff entry `=`, yielding6f99b6c4f. Generic context18eb→c5a184af2 and child relationshipde1→0bac483e6 retain identical patches. Exact rebase receipt `wdp-rebase-51a28da8a.proof.json`.

On rebased6f99, one admitted -j2 zero-warning build590.48s followed by all seven WDP cases GREEN32.31s. This is the current same-binary WDP proof; pre-rebase results are not silently counted as post-rebase runs. Artifacts `native-ww-wdp-children-rebased-6f99b6c4f/` and `ww-wdp-children-rebased-6f99b6c4f.build.*`.

## Unchanged existing hit-context consumers —6f99b6c4f

One admitted -j2 `--no-run` compilation of `server` and `server-spell-tests` completed1986.16s/zero warnings. Master51 moved the Quake fixtures into the latter package. Each requested existing case then ran exactly once, unchanged, under the same read-only CURRENT snapshot view; no ignored/default-suite substitution and no full-suite pass claim:

| Existing case | Native result |
|---|---|
| `rotation_selection_262_voltaic_blaze_damage_precedes_delayed_flame_shock` | 1PASS6.04s; actual child damage and delayed Flame Shock preserved |
| `dummy9_earthquake_pulses_six_times_at_requested_ground_and_excludes_allies` | 1PASS7.73s; requested ground/recipient exclusion preserved |
| `dummy9_earthquake_overlapping_casts_keep_independent_pulse_clocks` | 1PASS5.93s; independent native pulse clocks preserved |

Exact commands, binary hashes, CURRENT source/copy hashes and full logs: `native-primary-context-consumers-6f99b6c4f/`. Source remained clean at6f99 throughout all three executions. Together with the separately qualified unchanged Ret Toll eight cases above, all identified existing context consumers retain their scoped proof. No consumer test edits or duplicate runs.

## Fresh post-rebase300s measurements

Same6f99 simulation binary; a new read-only CURRENT canonical backup before EACH run. Reference/input/talent/binary/SHA receipts are retained per measurement. Both commands exit0 and run300sim seconds, but neither meets±10%:

| Measurement | Native DPS / reference | Difference | Actual WDP output |
|---|---|---|---|
| ST | 89912.5433 /206443.72 | −56.4469% | Root152175 casts8;158221 hits40/damage490857;451767 hits8/damage191918 |
| Three-target | 113892.0267 /376559.68 | −69.7546% | Root152175 casts3;158221 hits40/damage600556;451767 hits3/damage77522 |

Root152175 itself correctly remains non-damaging; triggered child action counts are0 because FULL_MASK children have no ordinary CastSuccess, while their real hit/damage rows above prove execution. Native runtimes33.52s/22.93s. Exact folders `native-after-wdp-children-6f99b6c4f-1/` and `native-after-wdp-children-6f99b6c4f-3/`; aggregate `wdp-children-6f99b6c4f-comparison-summary.json`. Enabler uptime57.8833%/9.5833%. This is integrated progress, not isolated child-only DPS causality or numerical parity. WDP Combo Strikes/Hit Combo eligibility, broader recipient/cap boundaries and full numerical qualification remain outside this dispatch-only proof.

## Native qualification cases

Two ignored native tests use real literal269 fixtures and stop on the first actual152175 CastSuccess. Single-target case asserts one immediate positive451767 delivery at the root's actual primary, not the self aura unit, and no repeats during subsequent periodic frames. Three-target case asserts one158221 delivery or legitimate native miss per actual eligible target on every native250ms tick through1000ms, including the authored initial tick at the first native update, none manually at cast or after expiry. No child/aura/resource/CD setup grants. The genuine missing-child RED and explicitly corrected initial-tick expectation are recorded above; setup/compile errors are never counted as RED.

Historical5618 actual300s receipts had root casts8ST/3three-target but no158221/451767 rows. Current proof qualifies once-per-cast primary delivery, native periodic scheduling through expiry and the concrete three-recipient fixture; it does not qualify the entire ability's numerics or every recipient/cap boundary. The actual self-aura hit unit remains distinct from the retained requested primary.
