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

## Relationship implementation pending native GREEN

Class-only WDP registration supplies native158221 through existing `AuraScripts.on_periodic`; each effect0 callback uses normal triggered dispatch at the native aura unit/caster destination. Existing `scripts::on_hit_with_context` registers the root self hit and dispatches native451767 at `ctx.primary_target()` once. No new hook, mutable CombatTarget read, class ID in core timing/dispatch or target/coefficient override. All child target caps/reduced-AoE stay in their existing native path; SimC root-base5 is not copied. Existing Combo Strikes/Hit Combo registries are not expanded by this dispatch-only change; their WDP-specific eligibility/numerical qualification remains separate. Atde1 admitted zero-warning build592.49s; native six PASS/one AoE assertion FAIL27.35s. ST child and five prior WDP controls pass; actual AoE already delivers at sampled[50,250,500,750,1000] while the new fixture incorrectly expected four deliveries. Checked DB2 Attributes5=512 and native `build_effect` index0 initialization before any remedy: this is authored native scheduling, not an extra class callback. Test-only correction adds the initial flag prerequisite and first native frame delay to expected times. Existing names and recipient/damage/expiry assertions stay; expected schedule is explicitly adapted, not claimed unchanged. No production change fromde1. Original missing-child RED remains recorded as[] vs the initial four-time expectation; native dispatch absence was real, while the post-implementation full schedule exposed this test-model omission.

Ret primary-context consumers are independently qualified unchanged:18eb cherry-picked on development8b520b865 as throwawaye293f7f64, `dps_parity_ret_toll`8PASS93.81s/zero warnings. Exact log/proof under engine `ret-parity-2026-10-23/ret-toll-primary-context-18eb96bb0-once-v1.*` read and confirmed. WDP corrected GREEN, other existing context consumers and fresh ST/3T pending.

## Native qualification cases

Two ignored native tests use real literal269 fixtures and stop on the first actual152175 CastSuccess. Single-target case asserts one immediate positive451767 delivery at the root's actual primary, not the self aura unit, and no repeats during subsequent periodic frames. Three-target case asserts one158221 delivery or legitimate native miss per actual eligible target on every native250ms tick through1000ms, including the authored initial tick at the first native update, none manually at cast or after expiry. No child/aura/resource/CD setup grants. Native RED not yet run; setup/compile errors will not count as RED.

Existing5618 actual300s receipts have root casts8ST/3three-target but zero root damage and no158221/451767 rows. Full native child dispatch/damage/recipient and numerical parity remain unqualified. First cast's native self-aura/explicit-primary distinction must be checked before treating a context hook's actual hit unit as the requested primary.
