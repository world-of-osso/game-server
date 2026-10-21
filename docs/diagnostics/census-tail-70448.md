# Class census tail — 70448

Clean-room reference: TrinityCore a352b1fa. Baseline evidence: engine `data/diagnostics/class-census-70448-2026-10-21/{by-cause,fixture-delta,per-spec-matrix}.md`;7071PASS/756FAIL of7827 memberships. Dispatch certification is not full-ability parity.

## Named gaps

| Cause | Roots | Disposition and evidence |
|---|---|---|
|285 LINKED_2 nonzero|371905,388491,388511,392221,471492,427775|Existing source conflict retained by lead decision2026-10-21. [Prior native matrix](../class-completeness2.md#zero-amount-linked-auras--batch25). Do not copy parent positive BP0 over70448 negative child values; do not invent sign or scaling rules.|
|483 SUPPRESS_TRANSFORMS|164862 Flap,188499 Blade Dance|**AURA_483_TC_NULL_NYI**. TC `SpellAuraEffects.cpp:555` dispatches HandleNULL; `SpellAuraDefines.h:570` explicitly marks NYI. Complete `src/server` search finds only those declarations, no consumer. Five memberships remain partial; no marker handler or invented transform behavior.|

## Active slice

Aura94 INTERRUPT_REGEN: native17057 Bear Form, index1 zero amount. TC `Entities/Unit/StatSystem.cpp:931–937` selects combat regeneration while aura exists without changing actual combat status. Focused ignored RED: `census_tail_94_native_bear_form_retains_idle_rage_until_removed`.
