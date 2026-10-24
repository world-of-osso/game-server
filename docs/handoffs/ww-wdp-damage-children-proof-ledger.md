# WDP damage-child relationship

Approved2026-10-24: `SIMC_WW_WDP_DAMAGE_CHILDREN`, after duration/admission qualification in the [enabler ledger](ww-wdp-enabler-proof-ledger.md). Scope: once-per-cast451767 at the actual primary target, and158221 at each native effect0 PERIODIC_DUMMY226 tick. Native children own damage coefficients, target cap/reduced-AoE and geometry. No152175 special case in core, no native permission/CD/power grants.

## Source order / absence

CURRENT backup `native-after-wdp-duration-5618c1eea/world.current.1.db` (source/copy SHA in adjacent receipts):152175 effect0 applies aura226, period250ms, TriggerSpell0; effect1=144, TriggerSpell0. Native root duration1000ms. No native trigger/periodic-trigger child edge; a whole-table incoming-trigger scan for152175/158221/451767 is empty. Native children exist:158221 school damage/AP3.1394999,451767 school damage/AP4.5. These amounts are not copied from SimC.

Full local TDB1210 world dump parsed with repository `mysql_dump.py`:4066 `spell_script_names`,220 `spell_linked_spell`,1283 `spell_proc` rows; zero signed-ID matches for root/children in the relevant binding keys. Entire available pinned TC a352b1fa `src` searched with gitignore disabled for IDs and WDP names: exit1, empty stdout/stderr. TC `SpellAuraEffects.cpp1324–1326` leaves PERIODIC_DUMMY to scripts. This is a missing scripted relationship, not evidence that generic DB2 trigger dispatch lost an edge.

Raw rows, TDB archive SHA/full-table counts, TC search receipt and verbatim pinned source: engine diagnostics `ww-parity-2026-10-23/wdp-damage-children-source-packet.json` (and sibling `wdp-damage-children-{native-source,tdb-absence,tc-absence}.json`). Source packet predates the timing decision below; its proposed SimC tick schedule is explicitly superseded, not implemented.

## Pinned source and approved timing difference

SimC76f8118 `engine/class_modules/monk/sc_monk.cpp5824–5827` identifies158221 AoE and451767 single-target children. Lines1888–1894 construct those children;1897–1906 execute the single-target child immediately and manually schedule the AoE child at `i * base_tick_time` while `i <= dot_duration / base_tick_time - 2.0`.

`SIMC_WDP_TICK_SCHEDULE_DIFFERS`: do NOT reproduce that source's0/250/500ms schedule. Parent explicitly chooses actual native aura226 ticks from P250/D1000:250/500/750/1000ms, including the native expiry boundary. No invented tick zero or manually scheduled class timer. `AuraScripts.on_periodic` is the existing generic periodic-dummy→class-relationship seam. Existing `scripts::on_hit_with_context` is the approved cast-child route; no new cast hook.

## Native qualification pending

Two ignored native tests use real literal269 fixtures and stop on the first actual152175 CastSuccess. Single-target case asserts one immediate positive451767 delivery at the root's actual primary, not the self aura unit, and no repeats during subsequent periodic frames. Three-target case asserts one158221 delivery or legitimate native miss per actual eligible target on every native250ms tick through1000ms, none at tick zero or after expiry. No child/aura/resource/CD setup grants. Native RED not yet run; setup/compile errors will not count as RED.

Existing5618 actual300s receipts have root casts8ST/3three-target but zero root damage and no158221/451767 rows. Full native child dispatch/damage/recipient and numerical parity remain unqualified. First cast's native self-aura/explicit-primary distinction must be checked before treating a context hook's actual hit unit as the requested primary.
