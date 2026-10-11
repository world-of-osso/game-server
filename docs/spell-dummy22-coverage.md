# Class spell batch22 — 2026-10-11

Base `origin/spell-dummy21` **ce5d03b**. Twelve previously underived roots, prioritizing rotational channels, healing/augmentation and tank cooldowns before DK pet-dependent builders and a soft-cap-only root. One derived numeric contract changes runtime; one existing native contract receives behavioral certification; ten roots receive exact source-only deferrals. [Contract](specs/spell-dummy-scripts.md#twenty-second-batch-native-rewind-non-raid-healing), [runtime](wiki/systems/spell-casting.md#twenty-second-batch-native-rewind-non-raid-healing), [matrix](spell-coverage.md), [previous batch](spell-dummy21-coverage.md).

## Capability matrix

| Root | Done / proof | Missing / exact reason |
|---|---|---|
|363534 Rewind|Consume s4=200 as threefold native healing outside a raid. Party is not raid; caster membership controls it. Native0.4 SP minimum, five ticks, heal cap and overflow retained. RED: three amount/overflow failures, raid control passes.|No timestamped incoming-damage history: s2=5sec/s3=30% cannot reconstruct eligible previous hits or their periodic distribution.376210 optional half-rank remains separate. Minimum healing is existing native behavior, not a history substitute or full Rewind implementation.|
|15407 Mind Flay|Existing native4500ms/750ms gives s4=6 ticks, each300 raw Insanity (18 displayed total). Five completed-root tests certify paired damage/grants, first750ms/no initial payment, wire cancellation, cap/overflow and friendly rejection;5/5 atc799de7.|Dummy s4 is a tooltip count, not an additional six-point grant. Census root retained; no registration-only/no-op removal. Haste/talents/full rotation uncertified. Existing behavior had GREEN evidence immediately; no RED manufactured.|
|33076 Prayer of Mending|Source review: s1=4 jump bound,41635 ward30sec,33110 heal,155793 jump radius30yd, variable208.|No recipient-selection/tie rule or jump-consumption accounting relative to initial application. TDB ward/jump script names do not define the algorithm. No guessed nearest/lowest-health/random jump.|
|98008 Spirit Link Totem|Source review:53006 totem,6sec,98007 ten-percent reduction/10yd,98017 one-second clock; immediate/periodic equal health percentages.|Generic summon28/totem-owned area65 lifecycle absent. Redistribution event attribution/procs unspecified locally; equal percentage is not equal absolute HP. No owner-centered substitute or invented damage/heal transfer logs.|
|108280 Healing Tide Totem|Source review:59764 totem,10sec,2000ms period,114942 radius slot1=40yd, s1=5 soft boundary.|Totem-owned healing lifecycle absent; beyond-five healing curve unspecified. Aura4 index2=100 has no independent tooltip/trigger/variable reference. No hard five-target cap or invented100% bonus.|
|357170 Time Dilation|Source review: s1=50 deferral over8sec;363899 healing-clear condition.|No delayed-damage reservoir/payment ownership or authored cadence/expiry/dispel handling. Zero aura69 does not supply it. No immediate damage reduction or synthetic healing substitute.|
|395152 Ebon Might|Source review: variable769 authors INT*s1/100, s1=8, s3=2 split threshold and optional414878 rank.|No split curve, periodic refresh/snapshot binding, recipient/exclusion rule for18/31+a540 or per-cast duration-extension amount. Variable classset has unmatched parenthesis. No invented ally set or generic own-stat buff.|
|409311 Prescience|Source review: s1=3 crit, s2=25yd untargeted search,410089 native18sec buff/a540 misc5, optional412774 copy.|Which active cooldowns qualify and exact priority/ties are not locally bound;540/wrapper202 and child markers unsupported. No nearest-only selector, a107 substitute or guessed copy proc.|
|86659 Guardian of Ancient Kings|Source review: native aura87 index2=-50 over8sec already supplies reduction; tooltip86657s2 agrees.|Aura4 index1 zero has no trigger/tooltip/variable/options/TDB binding to independent action. Preserve native reduction; do not duplicate it or claim marker closure.|
|85948 Festering Strike|Source review: s3/s4 bound next Scourge Strikes at2–3;275430 servant and native237409 summon row exist.|No persistent next-strike counter/ghoul ownership route; probability distribution across2–3 unspecified. No uniform roll, invented count or obsolete Festering Wounds reinterpretation. Native damage/raw200 Runic Power retained.|
|63560 Dark Transformation|Source review: s1=1 is .1sec Coil/Epidemic extension;344955 supplies ghoul-centered burst.|Permanent DK Ghoul/Monstrosity owner, transformation abilities and extendable aura absent. Self-target zero-trigger Dummy does not identify transformed actor/template/duration. No caster buff or arbitrary pet substitute.|
|2120 Flamestrike|Source review: s2=8 soft boundary, native8yd damage radius, TDB Flame Patch/Hot Streak routes.|No beyond-eight reduction formula. Optional205037/205470/205472 patch is separate. No guessed square-root/linear curve or eight-target hard cap.|

## Census and sources

Frozen baseline **157 roots/117 reviewed/40 underived**, after **157/129/28**: **0 roots/+12 reviewed/-12 underived**. Every root and unimplemented-type count is retained: E3 **111/119**, aura4 **63/71**, aura226 **18/19** IDs/memberships. Twelve reviews are not twelve implementations. Rewind closes only s4; s2/s3 remain. Mind Flay count is understood through native behavior, without removing a field from the upper-bound census.

All13 classes/all40 specs remain represented:630 book IDs/1551 memberships,302 core IDs/355 memberships. Native Mind Flay and partial Rewind add bounded positive proofs; no generic dispatcher is relabelled a named script.

**378 native SQLite rows exactly match** local Mainline12.1.0.69933 CSV columns (zero differences, exact strings/1e-10 numeric tolerance), including referenced durations/ranges/radii/cast times and child rows. Sources: read-only `data/world.db`; `/home/osso/Projects/world-of-osso/game-engine/data/db2/12.1.0.69933/`; local `data/reference/tdb1210/TDB_full_world_1210.26091_2026_09_09.sql` registration tuples. Variables33076→208,363534→775,395152→769; explicit absence for other nine roots. No C++ algorithms, external/CDN sources or DB writes. Only fixture Variance is zeroed; health/power/group membership are ordinary actor inputs.

## Proof ledger

| Revision | Command / result / scope |
|---|---|
|pre-c799de7|One native Mind Flay source-audit test:1/1 GREEN; existing behavior, not RED.|
|c799de7|`cargo test --locked -p server --bin game-server dummy22`:5/5 native Mind Flay tests GREEN, warning-free.|
|66eb38b|`cargo test --locked -p server --bin game-server dummy22_rewind`: three real40-vs120 amount/overflow RED failures; raid control passes. Optional combat-log target corrected before this run, not counted as behavioral RED.|

Rewind GREEN pending. Lead cancelled the originally requested `spell_cast` gate; final proof uses targeted tests plus fmt/check only. No independent/full-server/live/full-rotation acceptance claimed.

Verification uses the inherited immutable local shared-protocol44b2b0f with `--config patch."https://github.com/world-of-osso/shared-protocol.git".shared.path="/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/spell-dummy20-2026-10-11/shared-protocol"`; shared-protocol patch block remains intact. No master merge, deploy, live server or UDP5000.

Durable evidence: `/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/spell-dummy22-2026-10-11/`: source/variable receipts, exact reviewed reasons, frozen before/after sources and census generator/results, behavioral test stdout/stderr. Host-load gate missed for initial source-audit build; later runs honor capacity gate.
