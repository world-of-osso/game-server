# Spell Casting

## Scourge Strike owned-plague eruption — 2026-10-17

Native55090 effect1 supplies35%; a named effect0 consumer erupts pre-existing owned191587/1240996 through actual1241167/1241171, then retains the existing missing-Virulent application. Stored aura amount contains AP/base snapshot but not periodic damage modifiers. Shared read-only `damage_before_mitigation` applies the unchanged ordinary periodic numeric stages before rounding/crit/mitigation; native children exclude caster percent-done via Attr6=0x20000000 while retaining their own target-taken stage. This intentionally retains the non-bug source target multiplier, never SimC's optional division. No periodic clock/tick execution, consumption, refresh, generic trigger or hit-policy change.

Source: authenticated70448/read-only world.db25 effect rows; SimC76f8118 `sc_death_knight.cpp12502–12661/15372–15373`, `dot.cpp875`, `action.cpp1450`, `action_state.hpp96`. Evidence is engine `data/diagnostics/class-coverage-b-2026-10-17/unholy-*`; [contract](../../specs/class-completeness.md#unholy-scourge-strike-eruption-active70448), [coverage/proof](../../class-completeness.md#unholy-scourge-eruption-preparation-2026-10-17). Before named eruption implementation, shared extraction34 existing damage/absorb/crit/versatility controls pass; native ten-case RED is6FAIL/4PASS. Native9cacd3193 ten-case GREEN35.62s/0ignored/0warnings follows genuine six-failure RED. Integrated59c68472e check/fmt pass. The unrelated master spawn-fixture compile break is repaired by ownerefd627da1/lead511; integrated24b98c2ac native10+Scourge/Epidemic2 pass12/12 in53.31s,0ignored/0warnings,fmtPASS8.83s. Retain32 disjoint extraction controls and unchanged production bin-check proof; no broad replay. Lead integration gate remains OPEN. Spread/tie rules, Scourging/hero/set, Dread death/single-host, ghoul readiness/ownership and full legal/UDP/spec parity remain open.

[Health aura-state qualification](../investigations/health-aura-state-admission.md) preserves the exact source update/check boundary and open passive/ownership seams. The implemented aura-spell checks below are not health-state admission or full-trigger target validation.

## Dead-caster native channel coverage — 2026-10-17

Native55224 Archivist's Scan supplies cast-time row195=1100ms, duration row31=8000ms, Attr1 channel bit0x4, Attr0 AllowCastWhileDead, and a real zero-amount aura12 target stun. It has no power or target-condition rows in the owned source snapshot. This is a distinct prepare→channel lifetime boundary, not another direct cast. New concrete fixtures use actual5 and DeathPlugin-produced Dead, retain the native cast/channel clocks and require actual channel state plus an ordinary target Frostbolt refused as Stunned. The full-mask5 call used to kill a caster during its channel is the real generic-CC-exempt bool API, not an edited flag.

Measured coverage: natural completion while still dead, actual death interrupting a living-start channel, and successful Ghost release interrupting an already-dead channel, all removing the target aura. Baseline9c13da6b already passes3/3 in21.80s (196.412s wrapper, no warnings), so this slice changes tests/docs only and claims no invented RED or production fix. Masterc23d4110 is integrated as433e8ae9 with unchanged protocole9b80b8; its dispatcher adds124 pull_towards/Omor work, not these E6/E1 or channel/death/Repop paths. The three unchanged baseline proofs and prior runtime221 remain retained within their scopes; no rerun solely for the merge. Workspace/all-target check passes37.538s/0warnings; fmt and main manual Rust readability pass. Medium tests/docs-only work uses the main-thread gate, with no new independent verification claim. No raw health/mana/stat/native flags/rows/clocks/RNG/budget changes, fabricated aura or learned Archivist ability claim. Receipts `data/class-completeness6-2026-10-12/dead-caster-channel-{goal,source,proof-ledger}.json`.

## Dead-start cast progression and death interruption — 2026-10-17

The source-qualified distinction below is now wired at cast progress: eligible dead-start casts can advance, but observing a living-to-dead transition still interrupts. Existing server-only `PendingCastSpell` now contains the native `SpellInfo` plus one `caster_was_alive` boolean, captured by the existing cast-state constructor and updated on progress. This is an observation of actor life, not a native spell flag, new wire field or trigger-origin registry. Missing snapshots cannot positively qualify a dead-start exemption; there is no alternate metadata lookup. Movement rules and ordinary completion revalidation remain active.

NPC death owners now interrupt through existing `cancel_cast`: after CombatStop/before JustDied, at natural corpse removal, and on forced non-summon despawn. A dead NPC with DeathState but no Corpse does not tick spell events (`Creature::Update` DEAD branch). `Creature.cpp:430–439` is important: RemoveCorpse calls setDeathState(DEAD), which interrupts existing casts again; absence of later Unit updates alone is not the removal contract. Player death still uses the existing deferred transition and sampled progress boundary; no new claim of synchronous/sub-frame player death ordering or revive/redeath parity.

Genuine RED7791f1dc is3pass/4fail6.07s (378.572s wrapper, cold dependency rebuild): actual Death Touch5 and DeathPlugin produce PlayerDeathState::Dead, but active35058 and passive53008 bars cancel on the first progress tick; NPC death leaves a cast present before the kill call returns; post-corpse DEAD incorrectly cancels a newly queued event rather than leaving it unadvanced. Living35058 real damage, player death during a flagged bar, and natural NPC corpse-removal interruption controls pass. No raw Health/Mana/stat/native rows/flags/clocks/RNG/resource budgets are changed. Player identity74,001 is fixture identity, not a live account or injected life state.

Native35058 retains1000ms, RadiusB13=10yd, its original0.3 variance and no power/condition rows; actual damage is observable on the original fixture target. Native53008 retains its passive3000ms and non-death-persistent modifier aura, so completion must publish CastSuccess without retaining that aura. Natural zero corpse delay is not extended to force an NPC completion fixture. Native metadata fixtures do not certify learned Nether Explosion/Glyph abilities. Initial7/7 GREEN passes12.10s atf012cfa3; a forced-despawn control is added without separate RED. The first merged attempt fails compilation only because new master channel-destination consumers still use tuple `.0`; fdb8e0ec adapts them to `.spell`, without changing destinations. This is not behavior RED.

Independent inspection identified two additional phase boundaries. Actual successful ReleaseSpirit with native graveyards reaches Ghost but retains its bar (genuine RED1dfc8bc3); failed release preserves the bar as required. Native SmartLifetime corpse removal is separately reproduced with actual `zones::spawn_creature` and source Brown Ram guid20004147/entry12372, original300s respawn and native corpse delay: c889ee74 leaves the bar present after despawn (genuine RED). Initial Erland1978 is absent from active world spawns, so that earlier fixture error is not RED; no replacement row or clock was fabricated. Successful open-world release now cancels; the dungeon release closure cancels before teleport/revival. SmartLifetime despawn also cancels through the existing API. `Player.cpp:4287` BuildPlayerRepop calls setDeathState(DEAD); the source ghost health1 is not a reason to change this project's health budget. Respawn-continuity suggestions were not adopted: source compatibility-mode Respawn does not interrupt a newly stored DEAD-phase event, unlike the default destroyed-object mode. Full respawn parity remains unqualified.

Final8bcb9970 integrates master6c0cadc9 through1ed63098 with clean-FF landed protocole9b80b8; no authored wire changes. Current221/0/0 passes100.09s (329.193s wrapper, no warnings): eleven new cases plus205 prior and five named integration controls (unexempt queue death, Adarogg death/evade, merged channel destination, Stockade release and authored Smart despawn). Workspace/all-target check passes122.640s/0warnings; fmt passes. Independent read-only artifact/source/wiring/readability repair verification passes, distinct from main-run command proofs. Pure eligibility and effectful tick/result resolution are separated; life observation writes only when its value changes. Remaining small test/placement/style suggestions are deferred, not behavior findings.

Proof boundaries remain explicit: dungeon cancellation before teleport/revival is source/code wiring, not an independently failing active-bar fixture; the Stockade test exercises its maintained far-transfer/revival path. Failed release retaining Dead/the bar is this project's existing error contract, not whole TC Repop parity (source builds Ghost before its later graveyard lookup). The earlier compile-only merged attempt and missing-Erland fixture are invalid behavioral evidence and retained as such. Receipts `data/class-completeness6-2026-10-12/dead-cast-progress-{goal,source,proof-ledger}.json` and prior source qualification.

## Ordinary dead-caster admission and execution — 2026-10-17

Both `validate_start` and `execute_cast` unconditionally returned CasterDead. Pinned `Spell.cpp:5748–5749` exempts ordinary passive spells and Attr0 AllowCastWhileDead; `SharedDefines.h:442,459` supplies0x40 and0x00800000. The two boundaries now share one checker using existing `auras::is_passive` and the raw attribute, preserving every later caster/recipient/resource/CC check. Bool-triggered/no-aura-origin casts retain their distinct exemption below; no trigger-origin type or new flag storage is added.

Actual Death Touch5 kills the present NPC without direct health writes. Native15007 Resurrection Sickness is active with AllowCastWhileDead, while native199324 Divine Vision is passive without that bit; both have native AllowAuraWhileDead. Genuine REDcbf6e350 is2pass/3fail7.28s (76.171s wrapper): ordinary15007, ordinary199324 and direct15007 execution all return CasterDead. Living15007 application and unexempt dead116 refusal pass. Positive cases must observe actual stored native auras while the caster remains dead, not empty admission or marker injection. Mergede075b0cf integrates master42a7ab51 and clean-FF landed protocol1911f73. Current205/0/0 passes44.11s (139.238s wrapper): five new plus200 prior. Existing unexempt Slam queued-death control separately passes1/1 in5.71s (6.776s wrapper); native1464 Attr0=327696 has neither exemption. That legacy queue fixture retains its direct Health setup and is not an additional authentic-death or flagged-progress proof. Workspace/all-target check passes19.966s/0warnings; fmt and independent read-only artifact/source/wiring/readability pass. Main confirms actual authored diff and byte-identical RED fixture. A formatting hypothesis is falsified by actual fmt0; optional boolean-style/constant/lookup nits deferred. Receipts `data/class-completeness6-2026-10-12/ordinary-dead-caster-{goal,source,proof-ledger}.json`.

Both positive spells are instant. Their eligibility/application does not certify progression of non-instant casts from a dead actor. At qualification basis2184c2d0, `advance_casts` canceled every dead actor; subsequent bounded progression work and its limits are documented above. These fixtures are not legal Divine Vision knowledge, whole PvP talent/Sickness effects, self-resurrection94 or passive learn/gain/loss proof. No raw Health/Mana/stat/native rows/flags/clocks/RNG/budgets are changed.

### Non-instant progression qualification — 2026-10-17

Five independent read-only probes were reconciled against the pinned files at runtime basis2184c2d0. **No non-instant runtime acceptance follows from this qualification alone; subsequent bounded work is documented above.** Ordinary instant205 plus the unexempt queue control remain unchanged proof, not cast-bar coverage.

| Case | Pinned source boundary | Current proof |
|---|---|---|
| Already-dead player or creature CORPSE starts an eligible cast | Player945/Creature822 run Unit/events; Spell4240–4276 has no caster-alive progress gate and completion3759 rechecks5748 | Divergence at2184c2d0; subsequent bounded implementation above |
| Living caster dies while PREPARING or channeling | Unit9178/9187–9197 interrupts current non-melee spells at death before aura/control cleanup; flags/passive do not exempt that interruption | Existing health-poll cancellation is not a model of this distinct transition |
| Launched projectile's caster dies | InterruptNonMeleeSpells(false) skips generic LAUNCHED state | Separate lifetime boundary, no new behavioral proof |
| Creature is DEAD rather than CORPSE | Creature776–817 does not call Unit::Update | Do not extend CORPSE event progression to despawn/respawn/DEAD |
| Ordinary instant active/passive exceptions | Native15007/199324 and direct execution | Existing205+1 proofs retained; no rerun after docs-only work |

`Spell.h:683` defines `IsInterruptable()` as `!m_executedCurrently`: the death-interrupt exception prevents reentrant cancellation during execution, not an exemption based on the client's interruptible cast-bar attribute. The newly cached pinned header has URL/SHA receipts. `Unit.cpp:11447–11458` runs kill/death procs before the death transition; simply letting every flagged dead cast progress would lose source interruption of a cast that began alive. Conversely, moving every cancellation to a delayed player transition risks canceling a new already-dead cast. Integration must preserve the actual ordering, not replace the health poll with another assumption.

Literal runtime state at qualification basis2184c2d0 was `CastState` plus server-only `PendingCastSpell(SpellInfo)`; neither recorded a life transition. The later shape change is documented above. The maintained runner patches shared-protocol-s2 explicitly, not the canonical sibling claimed by one explorer. No new wire/state type or remediation was adopted during this read-only step.

Native candidate35058 Nether Explosion has1000ms, AllowCastWhileDead, E2 and selectors22/15. RadiusA is0 but RadiusB is13; inspecting only A would incorrectly reject its source area. Its actual radius, costs, target conditions and observable hit outcome must be authenticated before behavioral RED. Native53008 supplies a passive3000ms modifier but lacks death-persistent aura eligibility;55224 has the channel bit, so neither is interchangeable with a direct active cast. No flag/cast-time/clock/RNG/resource edits or NoOp markers are authorized to make a fixture work.

Sources: TCa352b1fa `Unit.cpp:3158–3184,3255–3263,9178–9218,11447–11458`, `Spell.h:683`, `Spell.cpp:3759,4240–4276,5748`, `Player.cpp:920–950`, `Creature.cpp:749–835`, `Object.cpp:245`, `SpellInfo.cpp:783–825,4003–4018`; current `spell_cast/mod.rs` and death producers. Receipts: `data/class-completeness6-2026-10-12/cast-while-dead-progress-{goal,candidates,native-inputs,qualification}.json`, plus `tc-a352b1fa/Spell.h.receipt.json`.

## Creature bool-trigger dead-caster admission — 2026-10-17

`creature_cast(..., triggered=true)` unconditionally rejected a dead caster. Pinned `Spell.cpp:5748–5749` instead exempts `IsTriggered() && !m_triggeredByAuraSpell`; `SpellDefines.h:498,523` gives the bool constructor FULL_MASK with default null TriggeringAura, and `Spell.cpp:3455–3459` only records an aura origin when one is supplied. This bool API has no aura-origin argument, so its living-caster gate was not native compatibility.

Actual Death Touch5 kills the present NPC without direct health writes. Genuine REDe214ccf5 is2pass/2fail10.64s (116.157s wrapper): dead bool-triggered116 incorrectly returns CasterDead, and dead bool-triggered205406 reports CasterDead instead of its still-required214502 CasterAuraState. Living bool-triggered116 and dead ordinary116 controls pass. The unconditional gate is removed; all caster/target aura checks, recipient validation, cost/time/CC bypass and depth1 remain unchanged. Native116's228597 child is loaded, but these fixtures prove admission, not full Frostbolt flight/damage from a dead actor. Merged1a33d5c9 integrates master1e782e6e with unchanged landed protocol523d4f9. Current200/0/0 passes72.31s (148.091s wrapper): four new plus196 prior, including actual reflection, all metadata/cost/queue controls and repaired resurrection fixtures. Workspace/all-target check passes15.734s/0warnings; fmt and independent read-only artifact/source/wiring/anti-pattern/readability pass. Main confirms authored diff and byte-identical RED fixture. A reviewer formatting hypothesis is falsified by actual fmt0; optional comment wording deferred. Main also confirms `Object.cpp:2258` passes only args.TriggeringAura into prepare, and `Spell.cpp:9646–9657` SetTriggeringSpell does not populate it. Receipts `data/class-completeness6-2026-10-12/creature-trigger-dead-caster-{goal,proof-ledger}.json`. Ordinary passive/AllowCastWhileDead and aura-origin trigger policy remain separate.

## Normal explicit AllowDeadTarget admission — 2026-10-17

The maintained `validate_target` used `target_alive == resurrects`, treating every non-resurrection spell as living-only. Native9454 Freeze has AllowDeadTarget, independent of the aura's missing AllowAuraWhileDead. Actual Death Touch5 leaves a present corpse; player request, ordinary creature and triggered creature entry all incorrectly rejected9454 with InvalidTarget. Genuine RED24687a16 is2pass/3fail12.93s (164.091s wrapper), with actual living Freeze stun and corpse Flame Shock/no-payment controls passing. No raw health/mana/stat/row/flag/clock/RNG/budget mutations.

For a living unit, the validator retains resurrection rejection. For a dead unit, it now uses the already-qualified `SpellInfo::allows_dead_target` predicate (`SpellInfo.cpp:1842–1857,2455`), then keeps map/type/range/LOS and all later aura/resource checks. Aura persistence still follows `Unit.cpp:3525` and existing `auras/mod.rs:865`; successful dead-target Freeze must neither apply an aura nor revive the unit. Queued callers share this validator, but native64 parents of9454 are absent, so no fabricated queue chain is claimed. Native resurrection2006/2008/7328/20484/61999/95750/361227 has raw corpse-ally Targets32768; maintained resurrection controls must remain passing. The first merged run passes192 and fails three legacy offer/decline/effect172 tests: both hand-built7328 fixtures omitted native corpse-target metadata. They now load actual Redemption7328, retaining its range/cast-time/cost/flags; only the pre-existing synthetic172 acceptance variant replaces its effect list and remains explicitly non-native. No production bypass or budget edit. The first corrected fixture run passes4 and refuses3 with AffectingCombat: the common rage-preserving harness starts in combat, unlike the source Redemption precondition. Setting only the fixture caster out of combat matches the already-passing native ghost test. Corrected3/3 pass5.37s (41.801s wrapper); four prior fixture controls remain valid, and all192 prior passes are retained without195 replay. Added native2008 living-friendly rejection control passes1/1 in46.85s (210.900s wrapper), with no separate RED claim. Production ef1b8636 is unchanged; current962a3a34 integrates masterb2855010 throughc468c3d2 and uses clean-FF landed protocol523d4f9. Workspace/all-target check passes19.945s/0warnings; fmt and independent artifact/source/wiring/readability pass, including a bounded confirmation of the fixture repair. Main confirms actual diffs: the five native feature tests and all legacy restore/offer/decline/timeout assertions are unchanged; only native2008's separate control was appended. Legacy resurrection target Health/Mana setup and synthetic172 were pre-existing unit-test inputs, not newly authored native ability proof. Optional name/comment/dedup suggestions remain deferred. Receipts `data/class-completeness6-2026-10-12/normal-allow-dead-target-{goal,proof-ledger}.json` and prior full-trigger source rows.

## Shared full-trigger dead-recipient admission — 2026-10-17

`plan_triggered_spell` bypassed all explicit unit-life checks. Native Death Touch5 leaves a present corpse; a subsequent shared full-trigger188389 admitted and applied Flame Shock. Source `SpellInfo.cpp:1842–1857,2455` checks `IsAllowingDeadTarget`: Attr2 bit1, raw Targets corpse-enemy0x200/unit-dead0x400/corpse-ally0x8000, or active-effect corpse-object selectors93/119/121. The shared route now applies this unit-life predicate before target aura checks/planning. It does not classify non-unit objects without Health as dead units.

Genuine RED7551ec5f includes two Flame Shock failures and a passing living/cost control. The fourth test initially demanded Freeze9454 persist on a corpse, incorrectly conflating AllowDeadTarget with AllowAuraWhileDead. Corrected bcd4bde9 retains successful admission but asserts no aura and no resurrection; focused positive1/1 passes11.24s before production changes. Existing `auras/mod.rs` and pinned `Unit.cpp:3525` (`_CreateAuraApplication`) explicitly reject that persistence; the distinct script `AddAura` path is not this spell application. Both genuine negative fixtures remain unchanged, and the current entire test file equals pre-production bcd4bde9. Merged2a90014e integrates masteraae714c5 with unchanged landed protocol6b06680. Current183/0/0 passes52.75s (300.182s wrapper): four new plus165 retained and5 creature-script/9 actual missile tests, including reflected Frostbolt228597 hitting its mage. Workspace/all-target check passes182.777s/0warnings; fmt and independent read-only artifact/source/wiring/readability pass. Main confirms actual authored changes and fixture integrity. Raw target-flag/corpse-selector predicate branches have primary-source/code proof, not separate admission fixtures. Receipts `data/class-completeness6-2026-10-12/full-trigger-dead-target-{goal,source,proof-ledger}.json`.

A wholesale normal `validate_target` graft was rejected during independent source qualification: a reflected Frostbolt's parent plan redirects to its mage, then effect32 calls the shared helper for native228597 (selector6). Ordinary relation validation would reject that current self-hit. This change leaves that relation bypass and all range/LOS/auto-self/actor-death behavior untouched; it is not whole source FULL_MASK compatibility. Source `Spell.cpp:3488–3492` only exempts BAD_TARGETS for special IGNORE_TARGET_CHECK, never TARGETS_DEAD. Native9454 is an internal metadata control, not a learned/class-legal spell claim.

## Creature-script triggered aura metadata — 2026-10-17

The distinct `creature_cast(..., triggered=true)` branch already validated recipients but skipped every caster/target aura metadata checker, unlike the shared full-trigger and queued routes. Native bool-true UnitAI casts use FULL_MASK0x0007FFFF; its generic CC exemption is not IGNORE_CASTER_AURASTATE0x00200000 or IGNORE_TARGET_CHECK0x00100000 (`Spell.cpp:5873–5889`, `SpellInfo.cpp:2507–2517`, `SpellDefines.h:293–302`).

The shortcut now checks caster metadata before its existing recipient validation, then target metadata on the admitted unit, before success/launch. Cost/time/cooldown/generic-CC exemptions and depth1 proc suppression are unchanged. Genuine RED0d74fa86 is2pass/6fail9.38s (119.618s wrapper): required caster205406→214502, excluded caster185313↔1784, excluded caster type449285/853, excluded target130→79636, required target type1293610/39 and excluded target type440490/485 all admitted incorrectly. Valid/clear native producer matrix and a stunned NPC's unrestricted116 Frostbolt control pass. Merged ce17d43e integrates master d1957002 with unchanged landed protocol6b06680. Current165/0/0 passes49.88s (145.446s wrapper): eight new plus157 prior controls. Five additional `creature_scripts::tests` controls pass11.65s (12.615s wrapper); an initial filename filter did not select those tests and is not their proof. Workspace/all-target check passes10.323s/0warnings; fmt passes. Independent read-only artifact/wiring/source/readability inspection passes; main confirms the actual diff and byte-identical RED test fixture. Reviewer's named boss spells119973/8148/120384/119425/371652/395029/265991 have no raw restriction rows in the owned native DB, so these new predicates do not change their admission (code/data qualification, not a boss-suite run). Test helper/naming suggestions are deferred; no behavior counterexample. Receipts `data/class-completeness6-2026-10-12/creature-trigger-aura-{goal,source,proof-ledger}.json`.

Producers are actual214502/1784/79636/642 casts plus an ordinary level-eligible Paladin853; the Stealth158185/158188/98877 closure is retained. Fixtures qualify NPC/internal metadata eligibility, not legal NPC spellbooks/player learning, cloud408 amounts, highlight UI/dummy scripts, complete Headlock/state1 or whole ability effects. Four raw states, required target notification1221044 and required caster types remain open; destination-missile costed paths, other implicit/auto-self cases and whole flag parity remain separate.

## Excluded caster aura-type admission — 2026-10-17

`check_caster` consumes `ExcludeCasterAuraType` after existing caster spell references, using the actual caster's active effect types, not amount sums. Native449285 Digestive Spittle excludes type12; actual level-eligible Paladin853 Hammer of Justice supplies a zero-amount type12 stun. Source metadata checks at `Spell.cpp:5881–5889` precede generic `CheckCasterAuras` at6113; ordinary admission now keeps263/form/caster metadata before generic CC refusal. This is that relative priority, not certification of every CheckCast error ordering.

Genuine RED9acb0858 is1pass/2fail17.32s (148.900s wrapper): ordinary NPC incorrectly reports Stunned instead of source CasterAuraState/detail, and full-trigger incorrectly admits the spell. Actual cancellation and native six-second expiry controls pass. Full-mask IGNORE_CASTER_AURAS is distinct from IGNORE_CASTER_AURASTATE, so generic CC bypass does not exempt this field. A recipient-only type control is added without an independent RED claim. Master9b7151e4 is integrated1155dd73 with unchanged landed protocol6b06680. Current merged157/0/0 passes98.03s (288.452s wrapper, no warnings): four new plus153 adjacent, including128 prior and25 polymorph/warrior-stun controls. Workspace/all-target check passes18.988s/0warnings; fmt and independent read-only artifact/wiring/source-agreement/readability pass. Receipts `data/class-completeness6-2026-10-12/excluded-caster-type-{goal,source,proof-ledger}.json`.

This is native internal/NPC metadata eligibility, not a player-learnable Digestive Spittle or whole ability claim. Ordinary/full-trigger admission and lifetime are exercised; completion and queued routes share the checker but lack an independent type-specific midcast fixture (stun normally interrupts). The separate creature-script triggered route is addressed above; the original shared-helper proof alone did not cover it. Six of twelve raw restriction fields now have bounded consumers. Required CasterAuraType is zero throughout native70391 and remains unimplemented;439683's excluded26 is retained as raw/generic field input, not an implemented254 effect or qualified positive cast.

## Queued effect64 recipient admission — 2026-10-16

Native full-mask child flags do not suppress explicit target admission. `SpellInfo.cpp:2455` refuses a dead recipient unless `IsAllowingDeadTarget` permits it;188389 has Attr2=16384 (not ALLOW_DEAD_TARGET bit1), Targets0 and unit selectors. The queue previously discarded missing entities but accepted a present corpse, then spent30 mana and applied Flame Shock: actual native470411 queues188389, actual Death Touch5 kills the NPC before dispatch. Genuine RED7bb23c5d is0pass/2fail33.06s (160.191s wrapper), including9970 mana versus10000 and an observed corpse aura. No health/mana/stat/clock/native-row/RNG edits.

The cost-retaining child route now uses ordinary `validate_target(..., Sight::Start)` after caster aura checks and before target aura checks/payment. Subsequent aura checks and planning use the validator's returned recipient; they do not validate an ally auto-self recipient and then act on a different captured unit. Self-only selectors with a captured unit return that same unit unchanged. Cost/item checks precede hit planning, then payment/launch. Generic full-trigger/reflection helpers and32/140 timing remain unchanged. Master53a59762 is integrated911d6395; final recipient-result fix9c01c758 and landed protocol6b06680 pass merged128/0/0 in27.84s (314.078s wrapper, no warnings): two new and126 prior controls, including living-recipient cost phases and all producer/queue positives. Workspace/all-target check passes39.710s/0warnings; fmt passes. Independent read-only artifact/wiring/readability inspection identified the return-value integration risk; main corrected it and verified the current128/check proofs. A stale-PvP-bracket protocol compiler attempt is retained as invalid proof, not RED. Receipt `data/class-completeness6-2026-10-12/queued-trigger64-target-{goal,source,proof-ledger}.json`.

This proves a dead-but-present recipient and no child payment/effect, distinct from prior entity disappearance. Caster death, unconverted allow-dead metadata, other implicit targets and whole flag parity remain separate; range/LOS/other existing recipient predicates have shared-validator/code proof here, not independent new fixtures. Pinned `Spell.cpp:6098–6102` explicitly checks triggered-spell range and `5996–6005` checks LOS; their numeric geometry parity is not broadened by this slice.

## Queued effect64 payment phases — 2026-10-16

`SpellEffects.cpp:692–721` removes IGNORE_POWER_COST/IGNORE_REAGENT_COST from full-mask child casts. Native470411 Flame Shock triggers188389: both have0.300000012% base-mana rows, but parent Attr6=0x20 is DoNotConsumeResources. Native TakePower/TakeReagents skip that bit while CheckPower still runs (`Spell.cpp:5451,5618,6104–6109`). Simply charging the child would double-charge the previously paying parent.

Four actual native fixtures change no mana/base stats, native rows, clocks or RNG. The existing controlled budget is10000: parent incorrectly pays30 immediately; after one ordinary50ms step, baseline regeneration gives9975 while the child is incorrectly free. Genuine RED0b0e8d01 is1pass/3fail23.12s (685.288s cold rebuild): parent-before-dispatch9970≠10000, live-child9975≠9970 and vanished-target9975≠10000. The general full-trigger cost exemption control passes. This is source-qualified payment ordering, not a shaman legal-build/base-mana claim.

Normal and cost-retaining queued casts now share a payment boundary honoring the bit; queued children plan/check aura eligibility and affordability/items before payment, snapshot resource-dependent values and launch without cooldown/GCD. The general full-trigger helper remains cost-exempt. No child spell-ID exception, native-bit rewrite or resource-budget manipulation. Current master9eb0b706 is integrated756e8352 with landed protocol5e4af62. Merged126/0/0 passes58.89s (358.280s wrapper, no warnings): four new cases plus122 adjacent, including retained queue/parent positives and charge/category/summon lifetime controls. A prior compile-only failure used stale concentration protocol; its log is retained, not behavioral RED. Workspace/all-target check passes92.274s/0warnings; fmt and independent read-only artifact/wiring/readability pass. Four new cases exercise mana phases and actual pre-dispatch target disappearance; lack-of-power and existing-but-invalid child planning retain source/code proof rather than a separate behavioral fixture. Receipts `data/class-completeness6-2026-10-12/queued-trigger64-cost-{goal,source-candidate,proof-ledger}.json`.

Actual rune/health/reagent payment fixtures, effect32 cost-path parity and full flag/launch-phase compatibility remain separate.

## Queued effect64 and required-caster trigger integration — 2026-10-16

Source `SpellEffects.cpp:594–723` schedules effect64 via caster events using native `MiscValue`; `EventProcessor.cpp:37–75` removes and executes one due callback before the next. Rust previously invoked64 synchronously inside the parent's effect loop, before parent aura publication. Native0ms298992→181448 and1ms412677→412680 both reproduce an inline child:7777e02e RED0pass/2fail11.81s,100.507s wrapper.

Effect64 now queues due callbacks in ordered millisecond buckets with FIFO order inside a bucket. The existing virtual clock is read, not changed. FixedUpdate dispatch follows cast completions, due hits and queued starts, before the aura update chain; a callback is removed before invoking it, so nested due-zero work cannot recurse into its parent callback. Missing entities drop the callback; existing trigger depth remains authoritative. Effect32 still uses its previous synchronous path. The same required/excluded caster checker now gates triggered casts before the explicit-target checker. All three exact required-caster fixtures retained from the blocked branch are restored, including normal channel cancellation before removal. Current master2d7faa55 is integrated063d89d4. The merged106 batch passes105, with only a new due-order test incorrectly observing an NPC's owner-only aura log. Test-only68e1ce54 observes the actual player owner's receipts and passes1/0/0 in11.00s (81.136s wrapper), confirming native0ms-before1ms order despite opposite parent-call order. The105 unchanged passes include every restored required-caster and Stack Timer positive, two genuine queue RED cases, linked/reentrant, shapeshift, missile and adjacent controls; no unchanged106 rerun. Workspace/all-target check passes223.375s/0warnings; fmt and independent read-only artifact/wiring/readability pass. No parent marker, early aura activation or requirement bypass.

Natural50ms fixture steps prove0/1ms non-inline delivery and parent visibility, not1ms wall-clock resolution. Native effect64 power/reagent retention is handled by the payment section above. Its actual rune/health/reagent fixtures and general LAUNCH-vs-HIT phase placement remain explicit gaps; this is not full flag/event-processor parity. Receipts `data/class-completeness6-2026-10-12/queued-trigger64-{goal,proof-ledger}.json` plus prior source qualification/blocked-branch receipts.

## Triggered excluded-caster aura eligibility — 2026-10-16

Triggered launch consumes active `ExcludeCasterAuraSpell` through a predicate shared with ordinary `check_caster`, before the existing explicit-target checker. Full-mask source flags do not bypass caster references (`Spell.cpp:5873–5889`, `SpellDefines.h:293–302`). Native146481 Fissure Shock triggers146480 Dark Ascendancy, which excludes146543 on its caster. Real ordinary self casts supply that identity; target-only identity and actual removal permit the child. Exact negative/positive fixtures are retained from fddf075c's genuine excluded-caster RED and matching pass; no unchanged RED rerun. Master3dc58b66 integrated92b12946 with landed protocol8f38d11 passes77/0/0 in45.36s (173.943s wrapper): two exact retained fixtures plus75 prior controls. Main verified fixture/helper/test bodies match the original RED byte-for-byte. Workspace/all-target check passes198.441s/0warnings; fmt and independent read-only artifact/wiring/readability verification pass. Receipts `data/class-completeness6-2026-10-12/triggered-excluded-caster-aura-{goal,proof-ledger}.json` and `triggered-caster-aura-source.json`.

### Historical required-caster attempt and source correction

Applying the complete caster checker on branch `class-triggered-caster-aura` (69ab93ab/7c54719d) passes negative identity checks but fails four positive native chains:298992→181448 and three412677→412680 controls (76pass/4fail). That old branch is **not landable**; the queued64 integration above supersedes it and restores its positive native fixtures. At the failed revision, Rust invoked effect64 synchronously before publishing parent auras.

Five independent source slices exposed a correction to the initial registration hypothesis: native effect64 runs at LAUNCH/LAUNCH_TARGET and **queues** the child using `MiscValue` milliseconds, not an inline hit-phase cast (`SpellEffects.cpp:594–723`). These native parents use0ms/1ms. The parent hit finishes application/activation before a queued zero-delay callback can execute; `EventProcessor::Update` removes each event, runs it to completion, then processes the next due event (`EventProcessor.cpp:37–75`, `EventProcessor.h:96–103`). A callback queued during the current callback is non-reentrant; a positive1ms delay waits until a subsequent time advance. Native application registration and effect activation really are distinct, but that distinction is **not** proof that these effect64 chains require a new identity-only aura store. Two explorers conflated64 with inline hit triggers; their staging recommendation was rejected. Source-compatible trigger scheduling supplies the bounded fix above; cross-actor ordering and full event-processor parity remain unqualified.

Do not weaken the positive fixtures, pre-grant markers, globally activate parent effects early or bypass pending requirements. The blocked branch retains its tests/proofs. This independent exclusion slice does not certify required-trigger parity or current-resolution parent identity. Research receipts: `data/class-completeness6-2026-10-12/aura-registration-qualification-{goal,result}.json`; newly cached pinned EventProcessor files include SHA receipts.

No native rows, clock/RNG/resource budgets, caster-type/state/health logic or whole-spell/transform gameplay changed. None auto-self, implicit/area targets and full flag compatibility remain separate.

## Triggered explicit-target aura eligibility — 2026-10-16

The source `TRIGGERED_FULL_MASK=0x0007FFFF` excludes both IGNORE_TARGET_CHECK0x00100000 and IGNORE_CASTER_AURASTATE0x00200000 (`SpellDefines.h:293–302`). Even IGNORE_TARGET_CHECK only overrides BAD_TARGETS, not TARGET_AURASTATE (`Spell.cpp:3488–3492`). Earlier bypass controls document previous server behavior, not source compatibility.

Triggered launch now invokes the existing target aura-spell/type checker on its explicit unit before planning/launch. The existing relation bypass remains separate; no full flag redesign or caster-trigger enforcement is included. Native412677 Aura Stack supplies its own aura then triggers412680 Stack Timer, whose native target exclusion forbids itself. Actual repeated normal parent casts incorrectly refreshed the180-second child: genuine REDe52a1cc7 is2pass/1fail11.95s, with expiry180.05 incorrectly becoming182.2; removal/foreign-unit controls pass. Merged masterfeae7a69 as9a8cb85c passes75/0/0 in25.73s (153.491s wrapper): three new,66 prior and six Ysera controls. Workspace/all-target check passes26.990s/0warnings; fmt and independent read-only artifact/wiring/readability verification pass. The existing direct bypass test is renamed and its target refusal follows source; its caster bypass remains an explicitly open gap. Receipts `data/class-completeness6-2026-10-12/triggered-target-aura-{goal,source,proof-ledger}.json`.

Missing explicit/auto-self resolution, caster-trigger references, states and whole flag parity remain open. This proves child metadata eligibility/lifetime only, not complete Aura Stack gameplay or UI.

## Native target aura-type admission — 2026-10-16

Ordinary resolved-unit admission/completion now consumes `TargetAuraType` and `ExcludeTargetAuraType` after the existing aura-spell references. The target's actual `ActiveAuras.has_type` result supplies presence, without amount interpretation, source-ID exceptions or borrowing another unit's auras. Failure retains the source TargetAuraState/detail contract; type checks are outside vehicle/owner state exemptions in the reference (`SpellInfo.cpp:2513–2517`).

Native1293603/1293610 require69/39;440490/448026 exclude485. Normal server-scripted casts exercise the passive/internal highlight rows rather than pretending they are client-learnable abilities. Real Power Word: Shield/Divine Shield casts supply the types, including caster/other-unit mismatch and eight-second removal/expiry controls. Genuine RED410436a0 is2pass/5fail6.21s (189.060s wrapper); positive shield/caster-only controls pass. Production33db2c7c is integrated with master0bf173f4 as2db43289, preserving authoritative target LOS and aura eligibility. Seven new plus59 adjacent cases pass66/0/0 in28.63s (249.693s wrapper); standalone GREEN was deliberately batched here. Workspace/all-target check passes51.794s/0warnings; fmt and independent read-only artifact/readability verification pass. Completion and player-route coverage remain source/shared-wiring proof rather than separate native fixtures. [Contract](../../specs/class-completeness.md); receipts `data/class-completeness6-2026-10-12/target-aura-types-{goal,source,proof-ledger}.json`.

This is metadata eligibility, not the indicators' UI/dummy implementation, actual Wrecking/Shattering Throw, Headlock/state1 completeness or aura485's own effect semantics. No handler was added to label an unknown effect as working. All four explicit fixture rows are instant; completion reachability has shared caller/source evidence, not a fabricated cast-time fixture. Caster types, other implicit/area targets, target-required references, health states and full-trigger target validation remain separate.

## Native required caster aura-spell admission — 2026-10-16

`aura_spell_restrictions::check_caster` now checks nonzero native `CasterAuraSpell` before the existing excluded-caster reference. Existing ordinary admission/completion callers therefore require the actual caster's active spell identity; a different unit's aura does not qualify. Missing state uses the existing CasterAuraState/detail failure and full-trigger helper bypass is unchanged. The internal module was renamed from exclusions to restrictions; no compatibility path or protocol change.

Native205065 Void Torrent requires194249 Voidform from the maintained228260 producer. Native205406 Sheilun's Gift requires214502: both actual two-second cloud/gift casts, a positive heal and loss of the cloud between admission/completion exercise the boundary without invented cloud amounts or edited cast times. Genuine REDa7999400 is3pass/6fail in9.05s, including wrong-recipient, expiry and ordinary NPC cases; native positive/full-trigger controls pass. Production4097f639 initially passes23/24 controls; the new midcast fixture incorrectly required an empty cooldown inbox after valid admission. A focused diagnostic observed exactly205406/category133/1500ms/is_gcd=true, matching native start recovery and `start_gcd`. Test7ff318a6 now distinguishes admission's empty inbox from completion's exact retained GCD packet, with the same source failure/no launch/no heal assertions; production GCD behavior is unchanged. Master97b9882b is integrated3933939f with landed protocolb701f32; corrected nine plus43 adjacent cases pass52/0/0 in19.79s (cold rebuild416.895s, no warnings). Current workspace/all-target check passes132.981s/0warnings, and fmt/manual changed-Rust readability pass. Only documentation follows those proofs; no unchanged rerun. Canonical aura-restrictions table publication is still lead-owned and absent at the final read-only check; the owned slot alone was populated. [Contract](../../specs/class-completeness.md); receipts `data/class-completeness6-2026-10-12/required-caster-aura-{goal,source,proof-ledger}.json`.

This consumes one more of the twelve raw fields at the existing ordinary routes, not complete Void Torrent/Sheilun cloud stacks, aura408 or legal-build acceptance. Target-required references remain separate: native322109 Touch of Death requires1221044 Notification, while the existing health/damage consumer and fixtures have no authenticated notification producer policy. No direct native trigger parent for1221044 was found in the owned70391 DB. Do not enable that target gate by injecting a marker, guessing on-demand health production or weakening the existing health-admission tests. Four states/four types, health timing/uint64 semantics and other implicit targets remain open.

## Native excluded aura-spell admission — 2026-10-16

The raw [aura restriction metadata](spell-info.md#native-aura-restrictions-metadata--2026-10-16) previously reached no cast checks. Ordinary `validate_start`/`execute_cast` now consume `ExcludeCasterAuraSpell` on the actual caster and `ExcludeTargetAuraSpell` on the resolved explicit/auto-self target, before payment/launch. Both use existing active aura identity, not amounts or a spell-ID exception. Refusal carries exact source `SPELL_FAILED_CASTER_AURASTATE`/`SPELL_FAILED_TARGET_AURASTATE` through the existing InvalidTarget/detail wire pattern; no protocol change. Completion refusal retains the already-started GCD; existing full-trigger helper bypass is unchanged.

Native70391 Stealth1784↔Shadow Dance185313 and Slow Fall130→No Feather Fall79636 provide both field consumers. Cutlass Slash214794's actual450ms cast and Blood Frenzy214789 reproduce a blocker gained between admission and completion, with removal-before-completion as a control. Genuine REDc40d6f42 is4pass/5fail in12.90s, including ordinary NPC and selected-ally cases; productiona44ea1f7 passes37/0/0 in8.88s (nine new and28 native/form/casting/loader/Stealth-child controls). Rogue fixtures retain source158185/158188/98877 children, rather than only the excluded parent record. Master9aca3c1a is integrated as6db2db50; current merged37/0/0 passes7.71s, workspace/all-target check passes27.129s/0warnings, and fmt/manual changed-Rust readability pass. Protocol5405246 is unchanged. Target-midcast refusal, charge preservation and GCD-retention details have source/code wiring evidence rather than a separate native lifetime fixture; the actual450ms fixture exercises caster exclusion. No native amounts, rows, RNG, resources or clocks changed. [Contract](../../specs/class-completeness.md); receipts `data/class-completeness6-2026-10-12/excluded-aura-spells-{goal,source,proof-ledger}.json`.

The required-caster reference is consumed separately above; target-required spell refs, aura types, health/other states (including1784's separate excluded state12), implicit/area targets and full trigger-flag semantics remain open. Native61988/65219 marker definitions are absent locally and were not fabricated; the fixtures are not complete Stealth/Shadow Dance, legal-build or fresh-native acceptance. Canonical source-table publication remains a loader deployment prerequisite.

## Rain of Fire dynamic-object visibility — 2026-10-17

Actual86636 persistent objects now carry shared `DynamicObjectInfo` and existing Position through ordinary Replicate/NetworkVisibility. Existing same-copy100yd interest owns visibility, and actual DynamicObject expiry/caster removal owns despawn; neither server damage nor duration changes. Source70448 Effect88312 is effect27/periodicDamage3 with destination53/enemy28, Radius8=5 and Duration105=9s. Overheat86633 remains unit damage, not a ground area.

The real Lord Overheat/interest RED reaches a genuine object with correct5yd/9s geometry but no replication; caster-removal control passes before the change. Tests retain actual boss casts and area damage, distinguish fixture position changes from native input, and exercise other-copy exclusion. Postmerge server270.2s GREEN2/2 and shared wire1/1 accompany engine real-UDP21/native projection4 GREEN; affected server check29.25s/fmt pass without warnings. Position changes in the server fixture are not native movement-input proof. Joint consumer gate and fresh actual-client escape remain pending. [Contract](../../specs/rain-of-fire-visibility.md).

## Self hits alongside destination missiles (2026-10-17)

TC a352b1fa `Spell.cpp:2486–2521,4096–4138` keeps caster-targeted hits at zero delay even when other effects have a traveling destination. `missiles::hit_delays` preserves that separation; unitless effects land at the destination delay, not alongside the early self hit. Native Recuperate1231411 combines a self trigger1231418 with destination effect179 (`CREATE_AREATRIGGER`, properties38187) and Speed10: delaying the self trigger broke its immediate aura application and full-health termination tests. Shared aura-stack slots are unchanged. This fix addresses hit timing only; these tests do not certify area-trigger behavior or visuals.

## Native damage prerequisite fixtures — 2026-10-16

Charred Warblades heal-lifecycle and Warrior Rampage child-damage tests require actual landed melee hits. Their ordinary hostile NPC has native3% dodge (`spell_cast/hit.rs`), so asserting positive damage from every unqualified weapon hit was stochastic. At master379c2c6b the nine tests first passed, then unchanged Rampage failed on201364; a128-hit diagnostic failed on Chaos201428 at hit42 with an actual `Miss(Dodge)` event. Auras cannot accumulate Fire damage from a dodged hit.

These damage-only fixtures now use existing `harness_with_native_enemy_dodge_reduction`: actual Sunwell Radiance45769 effect1 applies native aura251/-20 enemy dodge. No spell attributes, production evade guard, kit, stat formula or hit resolver change. Dedicated avoidance tests retain the ordinary harness. The bounded repeated-hit regression asserts positive source-derived heal input while retaining actual combat logs on failure; it does not retry missed hits or fabricate damage. Named regression results live in the follower master-RED proof ledger; no full-suite acceptance follows from these focused tests.

## Readiness clock isolation — 2026-10-15

`history_now` reads Bevy `Time<Real>` for SpellHistory deadlines, owner snapshots, charge recovery, queue GCD admission, school lockouts, rate changes, resets and ready-time credit. Generic `now`, aura time, cast/channel delta and missile clocks remain unchanged. No global simulation clamp change. [Contract](../../specs/class-completeness.md#readiness-elapsed-time).

Original native-equal-budget2 server frames averaged358–790ms; Bevy0.19's default virtual clock cap retains at most250ms/frame. Native local timers advance independently, so received charges still toggle0/1 but readiness can precede server admission. Actual70391 categories require12000ms Overpower and9000ms Throw Glaive. A real60-second AutomaticTime/400ms-frame HostClient lifetime RED rejects an Overpower recast after its recovery plus two real seconds. At710339dc the unchanged60-second lifetime passes, including actual Overpower/Throw Glaive recovery and owner updates. At18182f8a, a separate real10-second/800ms-frame native Fury Slam1464 GCD test passes with sourced137050e8 cost removal, not injected Rage. The67 rate, restoration, ready-credit, category, queue, reset and school-lockout controls pass. This is owner-message/lifetime proof, not fresh native/UDP, rendered/audio or full-class acceptance; independent lead acceptance remains open. Evidence: `data/class-completeness6-2026-10-12/native-readiness-{active-source,clock-observations,proof-ledger}.json`.

## Warrior effect-hit stuns

`warrior_stuns` runs the pinned Shockwave46968/Storm Bolt107570 effect0 SCHOOL_DAMAGE hooks before the default damage handler, after target/effect immunity filtering. Shockwave casts132168 with ordinary validation except cast-in-progress and suppressed owner errors; Storm Bolt casts132169 fully triggered. Child missile speed/duration remain native. The separate Storm Bolt destination-area filter keeps only its explicit target without caster-owned436162, and otherwise retains native area selection. Other effects and unbound Dummy fields remain separate gaps. Source: TC a352b1fa `spell_warrior.cpp:1792–1856`, `Spell.cpp:5719–5734`; [contract](../../specs/class-completeness.md).

## Twenty-second batch: native Rewind non-raid healing

Rewind363534's native effect0 already scales each periodic heal by0.4 SP. SpellMisc's EXTRA_INITIAL_PERIOD,4sec duration and1000ms period yield five ticks, matching local description variable775's minimum. Its previously skipped s4=200 is now consumed by a named healing multiplier: GroupRegistry's caster membership distinguishes raids from parties/solo, without changing recipient selection, native amount, clock, rounding or overflow. This does not implement s2/s3's historical-damage component. [Contract](../../specs/spell-dummy-scripts.md#twenty-second-batch-native-rewind-non-raid-healing), [sources/proof/census](../../spell-dummy22-coverage.md).

Mind Flay15407's native4500ms/750ms clock and300-raw aura24 payment already fulfill the s4=6 total-count field. Behavioral tests certify cadence, wire cancellation, cap/overflow and rejected friendly casts. No additional six-point grant, registration-only no-op or production Mind Flay change.

Server-authoritative spell casting in `crates/server/src/spell_cast/`. `SpellCastPlugin` validates `SpellCastIntent`, runs cast bars and channels, spends power, starts GCD/cooldowns/charges, selects implicit targets and resolves effects from [[spell-info]] records. Power pools regenerate from the DB2 `power_type` table and replicate as `UnitPowers`. Aura effects, spell modifiers and aura stat modifiers come from [[auras]].

## Execute effect64 script override — 2026-10-17

Generic effect64 remains asynchronous. Pinned TC `spell_warrior.cpp:779–801` explicitly prevents its default handler for Execute163201 effect0 and directly casts the authored260798 damage child. That native override must run before the existing parent refund/survivor check; native Speed0 alone does not make a generic queued effect synchronous. The wide rebased test exposed real unchanged target health immediately after Execute. The spell-script handler now intercepts only this sourced effect, retaining child admission/cost checks and parent item level. Existing native rank/no-rank/survivor/lethal assertions are unchanged. At71c0be65 with landed protocola56688e, the complete failed-case recovery cohort plus queued-trigger controls passes29/29 in34.22s/209879ms total. Final four-filter run atf2be0f8b passes1606 server cases,0failures,17ignored in1474.24s, plus two single-case targets (exit0/1624361ms); workspace/all-target check passes164019ms with zero warnings, fmt/diff/manual Rust readability pass. The owned world DB remains exact published SHA318fda959a62c487c58d2ce1bf0e67b741eca2a9c3f080d189084c32b4644806. Existing ignored source/navigation/private-fixture cases remain open; this is not fresh UDP, rendered, full-ability/loadout or all13/40 acceptance. The same wide run exposed six obsolete inline-effect fixtures: Chaos Strike second hit is300ms, Rampage later hits300/900ms, Obliterate10ms children precede20ms children, and Rake's bleed must apply before testing later rank removal. Their waits/order now use those unchanged native delays and assert early non-delivery. Two achievement fixtures must use a fresh eligible61781 recipient because its native target-aura exclusion forbids refreshing an already transformed unit; the repeated-target negative and persisted two distinct successful hits remain checked. This does not certify all Execute custom-argument, Sudden Death or absorb semantics.

## Queued effect64 captured destinations — 2026-10-18

Pinned TC `SpellEffects.cpp:652–722` constructs `SpellCastTargets`, copies the parent's destination at677–678 when the child requires it, then captures targets and itemLevel by value in the delayed callback. The server queue already retained its selected Entity and item level, but omitted the per-cast `SpellInfo.cast_destination`. A fresh child therefore lost the supplied cursor before target87 planning. Existing native [Akunda279076→279077](toys.md#akunda-source-child-and-visible-trap-2026-10-16) reproduces exactly: actual learned-use/cast/cooking fixture spawns fire at casterx−8949.95 rather than supplied cursorx−8947.95 (0PASS/1FAIL/40.11s/209208ms), before any fixture/assertion change.

`QueuedTrigger.destination` now stores that original optional3D point at enqueue; the costed child helper restores it and item level immediately after loading the child, before transforms, eligibility, costs and planning. Absence stays absent: no caster-position, effect-destination or guessed-coordinate substitute is introduced. The selected explicit Entity, existence/admission checks, due ordering, depth, resource phases and item-level snapshot remain unchanged. The native Execute override uses the same costed-context entry, preserving its direct source-script timing. Other TC arguments not represented by this API are not claimed as implemented. Unchanged Akunda passes1/1 in55.42s/226272ms atae042920f. Fresh-master1a06890ec integration6114bad56 passes the complete requested queued_trigger/power_drain/toys:: cohort205/205,0failed,0ignored in493.85s/614864ms. Fmt passes; final check remains pending. No imported effects/flags/clocks/RNG/budgets or clamp code changes, and no whole-ability/loadout/rendered/fresh-UDP/full13/40 claim.

[Contract](../../specs/class-completeness.md); receipts `data/class-completeness6-2026-10-12/queued64-captured-targets-{goal,proof-ledger}.json`.

## Queued64 child affordability qualification — 2026-10-17

`queued_trigger_cost_tests.rs` adds a native-only fixture for the previously source-only child NoPower boundary: queue470411, retain the natural10000mana, then execute1000 unchanged enemy Soul Rend3405 casts against both the queued caster and a same-class control before the next normal frame. Both encounter actors retain their original health; the control proves the exact normal-regeneration amount without a queued child. Confirm native188389 affordability returns `NotEnoughPower(Mana)` and the queued callback applies no aura, consumes no mana and sends no suppressed cast error. The distinct full-trigger helper must still apply188389 to that same living recipient without paying its power cost. Normal regeneration remains enabled; its first-frame amount is observed, not overridden. At019005d6 the first baseline already passes17/17 in54.47s/289.908s total: new NoPower case, four prior cost cases, six drain cases, five ordinary dead-caster cases and one Mobile Banking completion-refusal control. The caster's post-frame mana equals the naturally drained regen control and remains below30; no hard-coded regen amount is asserted. No new production fix or invented RED. That historical baseline predates the landed ExpectedStat correction and is not current acceptance. On rebased9129a861, four cost controls and the achievement regression pass, but this fixture fails explicitly because its generic NPC lacks UnitLevel. The fixture now normally summons native Skul10393 and loads the real creature/ExpectedStat resources; no level, resource budget, spell row or clock is fabricated. The corrected affordability/cost cohort passes5/5 in32.29s/160052ms, then the complete failed-case recovery cohort passes29/29 at71c0be65. Final integrated acceptance is recorded in [Execute effect64 script override](#execute-effect64-script-override--2026-10-17).

[Contract](../../specs/class-completeness.md); source and planned ordering: `data/class-completeness6-2026-10-12/queued-trigger64-no-power-{goal,proof-ledger}.json`. This branch integrates the prior owned queued64 stack with native effect8; fresh current master still lacked queued64 when effect8 was published. Internal bool-triggered producers do not certify legal Soul Rend/Flame Shock loadouts, whole abilities or full13/40 acceptance.

## Native power drain8 — 2026-10-17

`effects.rs::power_drain` implements pinned TC a352b1fa `SpellEffects.cpp1146–1180`: target must be alive and its `PowerPools.order` primary must match MiscValue. A stored secondary/legacy Mana component alone is insufficient. The amount path now includes ExpectedStat and the two correlated CalcValue/handler variance stages; the original117-based source-parity claim is withdrawn. For zero-coefficient effect8, TC CalcBaseValue evaluates PlayerMana ExpectedStat × BasePoints/100, or CreatureAutoAttackDps when native ScalesWithCreatureLevel overrides it. Zero ScalingClass, absent SpellScaling and ContentTuning0 do not disable that else branch. Use real caster UnitLevel; missing level/data must fail explicitly. CalcValue then applies base variance before point modifiers and rounds effect8; EffectPowerDrain adds supported done bonuses and reuses the same signed variance before taken/truncation. These stages must not draw independent random values. No health, armor, crit or absorb resolution runs for the drain.

`change_power` supplies the actual removal after the zero floor. Only a different caster receives that amount times native Amplitude (caster spell-mod operation27), truncated and passed through ordinary energize clamping/logging. Native3405 amplitude0 grants nothing. At real fixture UnitLevel10, native PlayerMana1000 makes3405 drain100 and46453 round177.514801 to178 before100SP, yielding278;10922−39×278 leaves80. Neither the old117 nor the intermediate raw-base118 is the full native result. Native16094 retains ScalesWithCreatureLevel: Level10 AutoAttackDps17.0521317 gives rounded base284 before its correlated variance. The fixture initializes UnitLevel from its already configured Level10, loads actual expected_stat and does not modify actor budgets. Imported16094 retains its independent damage and slow effects. Tests use naturally spawned Skul10393 mana10922 and actual1459 costs, without changing imported effects, variance, actor pools or clocks. Initial native RED: four missing-drain failures, two exclusion guards pass. At merged26f5332a (implementation94fa4444), all six passed in18.34s/146.040s total with the old117 expectation; those observed results do not certify the missing CalcValue stages. Correction887760ff produces genuine6FAIL/2PASS73.91s: wrong10/117 drain, wrongfloor, wrongFrostbudget and accepted missing metadata. Implementation8fd5f604/refactorb8589ad6 passes9/9 in60.29s/206.409s total, including native signed-variance calculation endpoints290/384/491 and actual unmodified-RNG casts; [proof ledger](../../../data/class-completeness6-2026-10-12/power-drain-proof-ledger.json). The first stale-protocol compilation failure is not behavioral RED.

[Contract](../../specs/class-completeness.md). Sources: TC `Spell.h803`, `Unit.cpp6828/7011`, `SpellInfo.cpp754–759`, `SpellDefines.h181`; hashed receipt `data/class-completeness6-2026-10-12/power-drain-source.json`. Existing unimplemented damage modifiers and the distinct power-drain execute-log packet remain gaps; no full13/40, whole-ability, timing, legal-loadout or rendered acceptance claim.

## Level80 class completeness

[Victim melee critical chance](../../class-completeness6.md#victim-melee-critical-chance--batch30) applies native187 after caster/script crit values to melee auto-attacks and non-ranged attack-type spells. Native ranged-weapon metadata, not school alone, excludes actual ranged attacks; positive/negative/additive and natural-expiry controls use real native rows. Other incoming critical modifiers remain separate gaps.

[Contract](../../specs/class-completeness.md), [implementation/source/proof limits](../../class-completeness.md). The expanded native-row census covers all13 classes/all40 specs and7827 build-union memberships, including talent/hero/passive entries. Working means the controlled cast/passive dispatch boundary, not every legal loadout or full rotation; exact effect/aura, data, target and resource gaps remain in the durable handoff.

Shared native aura537 matches label plus aura owner;457 scopes charge haste by category and preserves running progress;416 applies masked spell/category cooldown haste with native millisecond truncation. Signed SpellPower costs now grant authored Runic power instead of being clamped away. School immunity/whole-spell misses are classified before Rune payment; composite schools require the combined immunity mask. Native level80 generator/spender/cap/rejection/immune and partial-Shadowfrost cases have targeted RED/GREEN proof. The baseline made no guessed coefficients, marker no-ops, DB writes, master merge or realm-port claim. [Continuation](../../class-completeness2.md) adds native cone24, raid corpse119, random destination72/86/91, party portals and identity conditions; class summon-template closure fixes an importer omission using owned data copies. The same continuation records ground/rectangle selectors, native ritual metadata, controlled Mage elemental lifecycle and per-GUID bag-item unlocks, including their source/threshold parity limits. Native Chrono Flames now repeats caster-to-recipient effective health history after the real Living Flame child impact, with authored spec percentages/window/caps and targeted controls; Retail temporal inclusion and full Chronowarden parity remain uncertified. Probe receipts remain bounded. The [representative live gate](../../class-completeness2.md#representative-both-skin-class-gate--2026-10-12) is now closed for all13 classes in both skins; full spell/talent acceptance remains open.

Native aura229 now reduces incoming area damage at its actual effect boundary, using school masks and multiplicative authored amounts. Feint native RED/GREEN and five mask/expiry/stacking/sibling/periodic controls are retained in the [continuation](../../class-completeness2.md#aoe-avoidance-shared-aura229); flag/leech and whole-Feint limits remain explicit. Native1784 has only443 bindings in the current CSV/DB, not aura16; do not relabel Leech as Stealth. Lycara378992 zero mastery amount remains an underivable points/percent gap.

The continuation also records [current-owner summon damage/raw mastery](../../class-completeness2.md#summon-owner-damage-and-raw-mastery--batch18), [unused-ready cooldown/charge credit](../../class-completeness2.md#control-of-the-dream--batch19), and [native Blooming Infusion counters/consumed missile modifiers](../../class-completeness2.md#blooming-infusion-and-consumed-missile-modifiers--batch20). Per-cast charged modifier values survive consumption without retaining their aura use; new charged buffs cannot alter already launched damage. [Bonus-stat negation](../../class-completeness2.md#bonus-stat-negation--batch21) adds native467's bonus-only percentage, preserving base stats and total-percent ordering across six spec categories. [Physical base-armor percentages](../../class-completeness2.md#physical-base-armor-percentages--batch22) now share live pet spell/melee mitigation and preserve player stat-armor ordering; ordinary NPCs remain excluded by the pinned source. [Minimum-speed floors](../../class-completeness2.md#minimum-speed-floors--batch23) apply native305 after normalization/slows without overriding roots/stuns; native expiry and strongest-floor controls prove player run/flight/swim rates, not whole dash or non-player template-relative parity. [Final armor factors](../../class-completeness2.md#final-armor-factors--batch24) apply native466 after base/stat armor and preserve fractions until final conversion; zero class fields remain zero. Native101's physical bit is the armor TOTAL_PCT factor beside466; unlike142 it has no player/pet guard, so ordinary and level-scaled creatures multiply their armor before the attacker-level multiplier ([spec](../../specs/class-completeness.md#armor-percentage)). [Zero-amount linked auras](../../class-completeness2.md#zero-amount-linked-auras--batch25) now apply/remove their native child by caster and synchronize stacks without recasting. Nonzero sign/magnitude conflicts remain named285 source gaps. [Masked moving casts/channels](../../class-completeness2.md#masked-moving-casts-and-channels--batch26) consume330 family masks at admission and refresh active permissions on aura changes, respecting native movement-exception vetoes. This is server cast/damage proof, not newly executed client/live movement proof. [Receiver mechanic durations](../../class-completeness2.md#receiver-mechanic-durations--batch27) apply the pinned232/234 stronger-negative rule and integer delta arithmetic to actually applied mechanics. Oppressing Roar's positive-only fields and out-of-mask selectors remain explicit source gaps. Symbiotic Relationship’s raw/effective heal basis remains an explicit source gap, not a chosen policy.

[Native form admission](../../class-completeness6.md#native-form-admission--batch28) adds64-bit required/excluded masks, authored form flags and recipient aura275 family-mask bypass. Ten native lifecycle/cast controls pass. Census now retains32 genuine form prerequisites rather than silently casting unshifted; thirteen275 memberships improve. [Lead fixture diagnoses](../../class-completeness6.md#lead-gate-fixture-diagnoses) distinguish dead cone targets, native Arcane level scaling and destination87 placement input from ground-selector regressions.

[Native casting restrictions](../../class-completeness6.md#native-casting-restrictions--batch29) add player-only263 checks at admission/completion, native flag exemptions and caster-class/parent-spell family intersections. Thirteen controls distinguish parent flags from effect masks and preserve264/E78 action gaps. [Cooking-focus integration](../../class-completeness6.md#cooking-focus-and-spellcaster-integration) retains both focus and spellcaster payloads with separate availability rules and consistent SELECT indices; twenty merged behavior controls pass.

## Twenty-third batch: authored combo builders and Rune Weapon parry

[Contract](../../specs/spell-dummy-scripts.md#twenty-third-batch-authored-combo-builders-and-rune-weapon-parry), [source/census/proof](../../spell-dummy23-coverage.md). `scripts/batch5.rs::shuriken_storm` applies active121471's native100-percent effect3 to197835's scripted per-enemy combo grant. Native Backstab energize already matches121471's modifier masks; the scripted Storm effect does not. This named correction does not double native builders twice or invent a Shadow-damage copy.

`summon.rs::spawn_summon` grants81256 only after authentic49028/27893 Rune Weapon creation. Native aura47 supplies30-percent owner parry; both durations are8sec. `despawn_summon` removes that owner's81256 on early actor removal. Missing real template spawns nothing/grants nothing; existing root admission remains unchanged. Mirrored attacks and49028 Dummy markers remain unbound.

All28 previously unreviewed roots now carry exact reasons:157/129/28→157/157/0, with all157 unresolved upper-bound roots retained. Zero underived means no unreviewed roots, not full gameplay closure. Thirty deferred roots re-read; known empower/radius/count data does not create missing movement/actor/history/selector algorithms. Targeted12/12 GREEN at449bb8e; no broad suite/live parity claim.

## Twenty-first batch: native combo builders

[Contract](../../specs/spell-dummy-scripts.md#twenty-first-batch-native-combo-builders), [source/proof/census](../../spell-dummy21-coverage.md). `effects.rs::energize` admits positive self-directed combo grants from depth0 native casts. `scripts/batch21.rs::berserk_combo_bonus` reads the active106951 index1 amount only for matching spell families; combined grant flows through ordinary pool clamping and overflow logging. Thrash's caster effect grants once, independent of its area target count. Native343216's1500ms periodic point remains independent and receives no extra builder point.

Completed-root Shred/Rake/Thrash, cap/overflow, expiry, actual periodic clock, unrelated family and rejected-friendly cast prove this numeric contract.405189 over-cap recovery and script-only grants remain uncertified. Seven other reviewed roots retain exact source-field or runtime-prerequisite deferrals, including known Spiritbloom30yd range and both known empower stage tables; these are not absent-data claims.

## Twentieth batch: authored totals and next-spell uses

[Contract](../../specs/spell-dummy-scripts.md#twentieth-batch-authored-totals-and-next-spell-uses), [source/proof/census](../../spell-dummy20-coverage.md). `scripts/batch20.rs` starts Barbed Shot's named246152 Focus aura once after completed root casting. `auras/refresh.rs` implements217200s2's authored five-amount damage total over the unchanged native twelve-second clock (seven initial/regular ticks). Equal distribution is inferred from the conflicting local total and clock, not independently observed Retail per-tick damage. Reapplication adds same-owner unpaid damage to that fresh total and divides across the preserved phase's future ticks, without repeating an extra-initial tick. Due ticks are paid before snapshot; foreign owners remain independent.

Stormkeeper's aura construction initializes charges from191634s5 before ordinary charge modifiers. Existing family-matched spell modifiers consume one use per completed cast; refresh rebuilds the authored count rather than resetting to the options row's single charge. No separate universal proc or overload algorithm is added. Unknown markers, soft-cap curves, massive-damage thresholds and unrelated guardian/rotation mechanics remain uncertified.

## Nineteenth batch: counts, healing totals and owned damage

[Contract](../../specs/spell-dummy-scripts.md#nineteenth-batch-counts-healing-totals-and-owned-damage), [source/census/all40](../../spell-dummy19-coverage.md). `scripts/batch19.rs` consumes Flurry's explicit count, Mutilate variable394 and Dispersion variable872. Dispersion preserves native extra-initial ticks; its description's total is distributed across native ticks with ordinary rounding, including Intangibility and existing Altered Form duration modifiers. This distribution is a local total/clock derivation, not independently observed Retail tick amounts.

Dispel passes the actual dispeller to the named34914→87204 response. Havoc's native single-target mark replaces only this owner's other mark. Direct single-target family-matched spell damage can echo its effective amount at s1 percent; the copied branch skips source modifiers/crit and recursion, but retains destination defenses. Rain of Fire's explicitly area-owned42223 child is excluded even though its per-enemy selector is6. Specialized child selectors/range/destination-mitigation live parity remain gaps.

Ardent Defender's native index1 field is reserved synchronously before deferred melee aura removal, preventing double rescue. Spell/direct/redirect paths share `apply_attack_damage`; normal melee uses the same reservation function inside its borrowed-health query. Environmental damage remains separate. Tests prove fatal-hit restoration/one-shot depletion and the deferred reservation boundary, not random melee outcomes or live timing. Aura316/118/137 behavior remains uncertified; this is only a Dummy-field closure.

## Eighteenth batch: native leech and reapplication

[Contract](../../specs/spell-dummy-scripts.md#eighteenth-batch-native-leech-and-reapplication-contracts), [proof/census/all40](../../spell-dummy18-coverage.md). Effect9 and aura53 share damage resolution with ordinary attacks; the resolver returns damage less overkill after immunity, absorbs, routing and scaling. Leech copies that amount through normal living-health clamping/logging, without another power, multiplier or crit roll. Periodic leech snapshots native SP/AP amounts, reads the original effect's amplitude and preserves existing tick scheduling.

`auras/refresh.rs` implements only named local rules:335467 redistributes unpaid same-owner damage over the refreshed phase's future ticks, paying due ticks before snapshot;190456 includes its explicit versatility capacity factor, spends s2 percent of each incoming hit and Protection73 accumulates remaining shields to s4 percent of maximum health. No generic Pandemic threshold or Karma redirect is invented. Conflagrate's existing grant now reads the native Charges31 modifier;267115 separately raises its117828 stack cap. Obliterate reuses the weapon-branch selector with local variable608's physical and frost child pairs.

Native leech paths cover34914/335467/234153/55078 and only205179's primary aura53. Singularity's area226, Ignore Pain142, Vampiric Touch markers/dispel retaliation, talent/guardian/Karma and Lifebloom near-expiry rules remain outside this implementation. Native14/14 at edfa536 includes completed roots and actual Drain Life channel/cancellation. Frozen census164/100/64→163/103/60 closes only Ignore Pain's two consumed Dummy fields; aura142 remains. E3=114/122, aura4=66/74, aura226=18/19. All40 prior script sets/counts reproduced; generic leech is not relabelled a named script. Generic registry admission is not full ability parity. Feature/master merge83c5fb3 includes account-achievement changes from632f3ae; single requested spell_cast404/404 and fmt/check pass, compiler warning-free. Later updates are docs/proof only.

## Seventeenth batch: named resources and restorative hooks

[Contract](../../specs/spell-dummy-scripts.md#seventeenth-batch-named-resources-and-restorative-hooks), [proof/all40/census](../../spell-dummy17-coverage.md). `scripts/batch17.rs` binds Shadow Bolt's spec-gated194192 gain, Conflagrate's245330 fragments/196406-gated117828 uses, Soul Fire's281490 gain and conditional base157736/445474 periodic mark, Blood Boil's55078 base disease and Marrowrend's native195181 stack grant. Shadow Bolt checks actual ActiveSpec266, not an invented passive. Blood Boil's absent212744 record is represented only by the native description's learned-spell condition, not a synthesized store row.

Lifebloom hooks bloom33778 on expiry/dispel and replace only other targets bearing the same owner's33763; replacement never blooms. The near-expiration refresh threshold remains underivable. Improved Regrowth231032 adds its native probability only when the target already has8936; any owner qualifies. Cenarion Ward102351's proc predicate requires actual damage before consuming its native charge; its hook casts102352 from the real aura owner onto the protected unit, not the event's attacker. Backdraft117828 joins53390's named stack-as-use handling, preserving30% per use instead of multiplying magnitude by stack count.

Guardian/Karma/Eye Beam and retained markers/talents remain excluded. Batch17 did not infer general leech from disease application; batch18 adds aura53 explicitly. Current proof and source limits live in the linked coverage ledger.

## Sixteenth batch: low-count rank and charge consumers

[Contract](../../specs/spell-dummy-scripts.md#sixteenth-batch-low-count-rank-and-charge-contracts), [capabilities/proof/all40](../../spell-dummy16-coverage.md). `scripts/batch16.rs` binds Holy Light's231642/88852 gain, Holy/Ret Judgment's315867/220637 gain, Crusader's Might196926's separate Holy Shock/Crusader Strike cooldown reductions and Riptide51564's named53390 grant. Rank fixtures pin KnownSpells separately from loaded child rows. Crusader's Might covers Holy Judgment275773 only; other Judgment variants and owned-mark/secondary/talent mechanics stay open.

Arcane Blast's successful-launch resource clone freezes36032s1 damage from pre-gain charges. Cast time and mana admission/payment separately read36032s4/s5. Arcane Power's native12042s2 is consumed once by the generic aura72 school/power-mask fold (TC a352b1fa SpellInfo.cpp:4185–4194), with no343208 gate. The obsolete named rank consumer duplicated that discount after arena preparation added generic school costs; the native cost control now requires100/70/70 for absent/unranked/ranked aura states. Tidal Waves stacks represent future uses, so53390 modifiers retain their per-use magnitude and completed matching casts consume one accumulated stack before removing the final native charge. No guardian module changes.

Healing Wave77472's native45/zero-chain-target heal now has a named single-primary plan, needed to test real Tidal use consumption through native cast admission. Generic45 stays unsupported; existing friendly self selection is unchanged. Frozen8ff636a→9721cec census stays163 roots/98 reviewed/65 underived: no existing census markers are silently removed.229 native rows match local CSVs; all prior40-spec ID sets match. Targeted12/12 warning-free; single post-origin/master spell_cast372/372 at e0fa018, fmt/check pass. Later master c644eaa adds three unrelated toy visual IDs; fresh156cd37 targeted12/12 and fmt/check pass without rerunning the broad gate. Corrected fixture/RED boundaries and source/proof scope are recorded in the linked ledger.

## Fifteenth batch: native health, weapon ranks and delayed hits

[Contract](../../specs/spell-dummy-scripts.md#fifteenth-batch-named-health-weapon-rank-and-delayed-hit-contracts), [capabilities/proof/all40](../../spell-dummy15-coverage.md). Eight local consumers: Vampiric Blood's30% max health, Frost Strike variable609's mutually exclusive weapon children, ranked Rising Sun Kick wounds, Karma's50%-max-health shield, Combustion/Pyromaniac's crit snapshot, High Tide's jump-loss removal, Beacon's owned effective direct group-heal copy and delayed Sigil hit/bleed. Existing High Tide charged modifiers already consume once per completed cast; triggered casts are not valid charge-spending fixtures. No generic Dummy/no-op or guessed actor/redirect action.

Beacon copies native53651s1/10% effective healing through53652 after the source heal resolves, excluding overheal, periodic healing and the copy itself. It replaces only its caster's old53563 mark; foreign marks survive. A fixed already-scaled heal reuses normal health clamping, threat and combat-log delivery without another spell-power/crit/modifier pass. Group membership and same-map ownership gate recipients; complete range/talent semantics remain uncertified.

Sigil uses the existing destination-owned ground pulse lifecycle with period0: wait root2sec, emit native204598 hit and targeted bleed once, then retire the field. It does not move with the caster or execute latent duplicate effect2; child markers/179 visuals/Fury timing remain open. Karma implements only authentic capacity/depletion/expiry, not its70% redirect.

Frozen615fb90 baseline164/93/71 becomes163/98/65 at1e719bf: only Vampiric Blood's last remaining Dummy root closes. Native238-row equivalence and exact previous40-spec ID sets checked. Targeted11/11 warning-free; single post-origin/master spell_cast gate360/360, fmt/check pass at6a56656. Detailed corrected RED and proof boundaries live in the coverage ledger. Guardian inheritance/melee and Eye Beam11-vs-tooltip10 discrepancy unchanged.

## Wildfire primary variance fixture

Current native source values replace stale golden constants, not runtime coefficients or variance tolerance. A legitimate upper primary draw reproduced the master red; bounded real casts now cover the full source interval while distinct AP/RAP preserve owned bleed and splash limits. [Source, exact failure and merged proof](../../class-completeness6.md#wildfire-primary-variance-master-red--batch61).

## Touch of Death primary damage — 2026-10-16

70391 root322109 fields s4/s3 supply100%/35% of the caster's live maximum health for weaker creatures/other admitted primary targets, matching pinned SimC76f8118 `monk/sc_monk.cpp:2698-2724`. Native metadata supplies Can'tCrit and ignore-caster-percent/ignore-victim-modifier attributes. TC a352b1fa `Unit.cpp:6922-6929,7033-7063,1209-1214` defines those exclusions; the shared supported school/caster/versatility and armor consumers now honor them. Unmodeled mechanic-specific damage-taken auras remain unproved, not silently certified by the attribute guard.

Actual ordinary322109 on an eligible999-health creature originally logs Damage0 and leaves999 health. After the health-based amount hook and native flag consumers, both999→0 and improved1400→1050 pass despite authentic NPC6 level10 armor, Enrage8599 effect0, Shield Wall871 effect0 and a deliberate100% caster crit input. No RNG override, template edits or guessed coefficient. Native admission/damage tests pass3/3 (20.52s). Disjoint ordinary Fire Blast/Vengeance/Shield Wall control plus base armor, creature-type, crit and defensive cases pass62/62 (52.65s): unflagged modifiers remain active. Source/proof receipts `data/class-completeness6-2026-10-12/touch-of-death-damage-{source,proof-ledger}.json`; [contract](../../specs/class-completeness.md#touch-of-death-admission). PvP integration, Stagger purification, mastery/follow-up talents, equality-source discrepancy and fresh UDP bot acceptance remain separate.

## Touch of Death target admission — 2026-10-16

70391 root322109's description requires a weaker creature, or passive322113 and a target below its authored15% threshold. Pinned SimC76f8118 `monk/sc_monk.cpp:2675-2688` compares target current health against the Monk's maximum and uses a strict percentage comparison for the improved rank. Its baseline uses `<=`, unlike the DB2 description's literal “less”; this implementation uses the description's strict baseline boundary, not a claimed native equality observation. Checked TC a352b1fa `spell_monk.cpp` contains no corresponding script.

The ordinary validation path now rejects ineligible targets before CastSuccess and cooldown publication. Two real HostClient tests cover seven cases: healthy stronger creature with/without rank;14% without/with the actually synchronized native passive;15% and16% with rank; and a creature below the Monk's maximum without rank. RED accepts healthy/untalented stronger creatures; GREEN2/2 at8cf90f31 (11.49s) preserves eligible casts and rejects invalid ones. No resource, crit, armor or damage override is introduced. The subsequent [primary-damage slice](#touch-of-death-primary-damage--2026-10-16) supplies the zero-base effect's missing amount; PvP integration, Stagger removal and follow-up talents remain separate. [Contract](../../specs/class-completeness.md#touch-of-death-admission); source/proof receipts `data/class-completeness6-2026-10-12/touch-of-death-admission-{source,proof-ledger}.json`.

## Beast Mastery Kill Command pet damage — 2026-10-16

70391 spell34026's description names83381; effect23978 is Dummy0, while83381 effect83776 is physical damage with native AP coefficient2.47199988. Pinned SimC76f8118 `sc_hunter.cpp:6587-6588,7549` executes the main pet's83381 action. Checked TC a352b1fa `spell_hunter.cpp` has no corresponding handler; no TC binding is implied. The root's hit hook now shares Survival's owned-persistent-pet bridge, selecting83381 instead of259277. The actual pet remains damage source, not the Hunter.

Ordinary HostClient34026 after native Call Pet883/creature299 reproduces owner CastSuccess with no pet hit, then passes with one positive83381 damage event attributed to that called pet and matching victim health loss. Triggered children do not require a separate CastSuccess event. Native BM plus unchanged Survival and both foreign-owner exclusions pass3/3. An intermediate Survival zero-damage failure remains recorded without a confirmed cause; its assertion now includes actual outcomes. No RNG/stat changes or retry-until-pass test loop was added. Animal Companion, pet charging, additional talent consumers and full admission remain outside this bounded bridge. [Contract](../../specs/spell-dummy-scripts.md#fourteenth-batch-local-hit-actor-and-periodic-consumers); source/proof receipts `data/class-completeness6-2026-10-12/bm-kill-command-{source,proof-ledger}.json`. Fresh UDP bot behavior remains unproved for this fix.

## Fourteenth batch: owned hits and native clocks

[Contract](../../specs/spell-dummy-scripts.md#fourteenth-batch-local-hit-actor-and-periodic-consumers), [capabilities/proof/all40](../../spell-dummy14-coverage.md). `scripts/batch14.rs` binds Frozen Orb/Blizzard's distinct named hit slows, Survival Kill Command's actual persistent-pet caster, Wildfire's named primary hit/bleed, Deathmark's80 total Energy on its existing bleed clock, Eye Beam's named primary/ranked crit, Metamorphosis's8yd destination hit and player daze, and Consume's named Fury. These are eight bounded entries, not full class/talent parity. Frost child-hit tests are not root-cast A proofs; cone179/129 actors remain separate gaps.

Eye Beam's native2000ms/200ms/EXTRA_INITIAL_PERIOD yields11 attacks. Local variable316's total expression references10: retain the discrepancy, do not replace the native clock with a guessed timer. Rank fixtures explicitly restrict KnownSpells to prevent automatic passive sync from making the no-rank case vacuous. Pet fixtures assert66 damage from the real22AP inherited state; no production scaling workaround. Landing player-CC fixtures require actual duel hostility, not an NPC faction substituted onto a player.

Frozen e88f0a6 census166 roots/90 reviewed/76 underived becomes164/93/71 at95604f2. Remaining Dummy roots259489 and360194 close; Metamorphosis consumes its root E3 while descendants retain gaps. Native118-row equivalence and fixed40-row table checked. Requested single spell_cast gate347/349 exposed two fixture stores missing the new authentic Wildfire children; ff410fe adds those rows and bounded failing-plus-new cases pass14/14. No second broad gate claim; proof/integration limits are recorded in the coverage ledger. Guardian inheritance/wolf melee remain unchanged.

## Thirteenth batch: authored hits, ranks and channel clocks

[Contract](../../specs/spell-dummy-scripts.md#thirteenth-batch-locally-authored-hit-rank-and-channel-contracts), [capabilities/proof/all40](../../spell-dummy13-coverage.md). `scripts/batch13.rs` binds named Mortal Strike healing reduction, Rend bleed, paid-Rage Execute refund, Shadow Word: Death execute/Deathspeaker variant, native Rapid Fire tick children, persistent-pet Bestial Wrath, Aspect's initial primary Cobra and Haunt's death-only reset. The launch boundary passes actual paid costs to Execute after its native zero-speed child resolves; no damage interpolation assumption. Rapid Fire reuses the native aura clock, including EXTRA_INITIAL_PERIOD, rather than adding a script timer. Death reset receives the owned aura instance's caster and removal reason.

Local guardian templates/bindings do not establish non-demon owner inheritance or complete wolf melee damage/AI. The1500ms template attack time alone is not a stat curve or movement contract. Those gaps remain unchanged. Batch13 targeted proof was12/12 at2476ec8; the later source audit removed an unbound pet buff, retaining only the named Bestial strike. Revised Bestial2/2 at33b965c plus unchanged ten prior fixtures are proved; details in the coverage ledger. Census168→166 roots,88/80→90 reviewed/76 underived: Execute and Shadow Word: Death consume their remaining numeric E3 contracts; damage interpolation/legacy variants are not certified. E3=117/125, aura4=68/76, aura226=18/19. Full40-spec table reproduces the exact prior baseline.151 native rows match local CSVs (numeric tolerance1e-10; strings/newlines exact). Post-origin/master at1e27532, single spell_cast gate325/337 exposed stale slot DB inputs (guardian_spells/tdb_playercreateinfo absent). Required canonical data link repaired without DB writes; all12 failures plus12 new cases pass in one bounded24/24 rerun. Format/check pass; no fresh337/337 claim. Exact scopes in the linked proof ledger.

## Twelfth batch: actor, resource and snapshot consumers

[Contract](../../specs/spell-dummy-scripts.md#twelfth-batch-actorresourcesnapshot-consumers), [capability matrix/all40 table](../../spell-dummy12-coverage.md). Native51533 explicitly names228562: its two29264 wolves reuse the generic summon lifecycle and authentic archive58875/58867 bindings. The parent casts the child once; shared EffectSummonType owns count2/property1161 (TC SpellEffects.cpp1934–1936). Repeating the child by its count duplicates this work after generic count support. Dummy remains counted for unimplemented immunity, buff/Maelstrom and elemental contracts; no guessed melee rotation. Native49206's pet-title3238 is a named temporary Gargoyle actor exception, not a blanket pet AI change. Its27829 archive binding51963 uses native cast timing and always critically strikes with1242147. Successful owner costs empower its211947 aura using49206s4 and PowerType DisplayModifier; failed/triggered/foreign casts do not. Non-demon stat inheritance and Army entry remain uncertified.

The launch-owned resource clone now also multiplies2098 Dispatch and196819 Eviscerate damage by actual combo expenditure. Aura construction snapshots Rake155722's60% stealth/rank bonus once; reapplication recomputes it, later rank/stealth changes do not alter an existing bleed. Owned233490 grants30108s3's10% spell/periodic vulnerability once, not per instance; foreign/expired marks are excluded. Melee/direct vulnerability, stacking/dispel/refund remain open.

Shield of Righteous53600 has a spell-owned native6yd/120-degree caster-facing cone and applies132403's live-primary-stat armor modifier once even with no enemies. Butchery212436 with459868 applies its named459870 bleed per successful enemy hit without changing existing294029 cooldown reduction. Child Dummy mirrors/optional/talent/soft-cap contracts remain counted. Census168 roots,88 reviewed/80 underived: Rake's E3 closes and Shield exposes latent132403 aura4; E3=119/127, aura4=68/76, aura226=18/19. Ten focused functions pass at60ce7ac; inherited Rapture fixture corrected/focused1/1. Single post-origin/master spell_cast gate325/325 ata995632, warning-free; fmt/check pass. Exact proof scopes in coverage/ledger. Sources:134 exact matching native/CSV rows and archive bindings; evidence game-engine/data/diagnostics/spell-dummy12-2026-10-10/. No DB mutation, realm run, C++ body or full parity claim.

## Guardian source follow-ups

[Contract](../../specs/spell-guardians.md), [source receipts/capabilities/all40 table](../../spell-guardians2-coverage.md). `guardian::initialize` explicitly dispatches native summon205180/entry103673 to Eye Beam205231, which the root description names as `$205231s1`. It does not insert a synthetic archive row or substitute Imp416. The existing actor loop supplies native2000ms casts, owner selection and20000ms lifetime; existing demon owner-SP calculation stays live. Three deterministic actor functions prove cast delay/damage, changed owner SP, retarget/no selection/expiry/no pet replacement and unbound-Wild-Imp exclusion. Only child variance is pinned0. Direct effect28 proof does not close root Dummy/Presence/per-DoT/extension contracts or certify a new inheritance coefficient.

`scripts/add_guardian_templates.py --db data/world.db.new` prepares authentic TDB103673/display116130 using the existing Retail template conversion. Actual prepared-copy fixtures load that overlay, not a renamed Imp. Native template/model preservation, idempotence, missing-model rollback and live world.db refusal pass3/3. Production content lacks the template; operator owns the prepared-copy import. No world.db writes occurred.

183 pinned12.1.0.69933 rows exactly match CSV. Archive creature spell/addon/smart-script/linked-aura/proc sources and CreatureDisplayInfo were rechecked. Xuen123904 explicitly names123999→123996, but its actor AP share and target policy remain unproved. Rune Weapon51906 and tamed-pet passive amounts are zero; pet_levelstats supplies base attributes, not inheritance percentages or weapon damage, with stale/placeholder level limits. Local gametables do not establish owner stamina share. Wolf1500ms and ghoul2000ms template times alone cannot derive melee damage. Non-demon inheritance, stamina scaling, guardian melee and other named rotations remain underivable; no guessed percentage/ID/flat swing added. Modern Army42650 Lesser Ghoul Orders cannot be inferred from legacy42651/24207.

Initial census164 roots94/70 from93/71. Final origin/master809b61e integration includes batch15/petbattle-effects3: baseline163 roots98/65, combined163 roots99/64; E3=114/122,aura4=66/74,aura226=18/19 unchanged by guardian follow-up. All40 batch15 baseline IDs/counts exactly reproduced; only Affliction core script presence3→4, root A unchanged. Single requested spell_cast gate352/352 at6ea07e4; later25bc1fc integration proves bounded guardian/batch15 tests22/22 plus fmt/check, not a repeated/fresh broad gate. Evidence: `game-engine/data/diagnostics/spell-guardians2-2026-10-10/`, including `integrated/` census/table receipts. No realm/full-rotation claim.

## Guardian actors and damage transfer

[Contract](../../specs/spell-guardians.md), [capability matrix/all40-spec table](../../spell-guardians-coverage.md). Effect28 creates its existing `SummonedCreature`; supported guardian/minion/runeblade titles (and named49206's temporary pet-title Gargoyle) gain `GuardianActor` when an authentic creature spell list or the explicit [Darkglare native-description binding](#guardian-source-follow-ups) exists. `guardian_spells` imports9586 TDB1210 creature_template_spell rows; startup opens it read-only and logs a missing import. Parent supplied the canonical9586-row table before batch12 (verified2026-10-10). Companion actors never acquire this combat AI. Actors read the owner's current `CombatTarget` and use normal creature casts, native power pools/costs, cast time and cooldowns. Lists are deduplicated in archive index order; untimed spells are used once, with no invented repeat clock. Existing demon-pet power coefficients are reused for demon templates; other guardian power scaling is not newly inferred. Native duration, owner death/removal or map departure removes actors.

Native111859 effect28 summons416 with properties3314; archive416 indices0/2 both name3110. Behavioral fixture invokes that effect directly: root111859's Dummy/cleanse remains unresolved. Native Firebolt1750ms and summon27000ms drive the timing proof. Wild Imp55659, Dreadstalker98035, Xuen63508, Rune Weapon27893, Darkglare103673 and Tyrant135002/250289 have no archive creature-spell/smart-script bindings in the inspected rows; [follow-up](#guardian-source-follow-ups) derives Darkglare's base attack from an explicit native-description reference. No Imp416 replacement, six-bolt budget, guessed child spell, inferred melee AI or whole-root closure.

Aura81 is a school-mask percentage split to its caster after protected-target mitigation/absorption. `effects::deal_damage`, real melee `try_swing` and direct damage share the same split records; melee subtracts shares before health application, not a post-hit heal/undo. A batch-local recipient health reservation prevents concurrent deferred swings from spending the same health-floor budget. Recipient damage consumes its absorbs and native health scaling without rerunning offensive modifiers/armor or recursively sharing. Living same-map, non-self and non-immune recipient checks apply. Sacrifice6940 reads native effect2's20% health floor; transfer caps at remaining health budget and consumes the protected ally's aura when the floor is reached. Integer health truncation remains in the existing damage pipeline. Rank/talent and complete proc parity are not certified.

Local sources/receipts: Mainline12.1.0.69933 effects/descriptions/SummonProperties; read-only world.db and TDB1210.26091 archive. `scripts/import-guardian-spells.py` and its atomic importer tests operate on a prepared DB copy/in-memory fixture; no world.db changes occurred. Durable evidence: `game-engine/data/diagnostics/spell-guardians-2026-10-10/`. Historical census168,87 reviewed/81 underived belongs to this baseline; [batch12](#twelfth-batch-actor-resource-and-snapshot-consumers) updates it. Atonement/Spirit Link and Karma's damage redirect remain underived; [batch15](#fifteenth-batch-native-health-weapon-ranks-and-delayed-hits) adds bounded Beacon copying and Karma shield capacity.

## Shared resource mechanics

Successful casts now clone a resource-dependent spell snapshot between cost validation and target planning, before payment. Flat native `SpellPower.OptionalCost` is additional spend capped by current power; only the required portion gates admission. `spell_cast/resources.rs` freezes combo expenditure into `SpellDuration.DurationPerResource`/`MaxDuration` and named Envenom/Black Powder damage coefficients. The owned spell clone follows the existing missile path; no mutable caster snapshot or post-impact pool read is introduced.

Arcane Barrage's explicitly authored36032s2 charge damage is frozen at launch. Its existing AfterCast reads the same charges before clearing them and, only with active321526, refunds native hundredths-percent of maximum mana. Native Rupture/Rip/Envenom durations and Black Powder area behavior have bounded tests. Surge only gains authentic optional mana payment: its tooltip supplies a maximum, not the interpolation curve, so an initial linear assumption was rejected and removed.

**Coverage:** [contract](../../specs/spell-dummy-scripts.md#shared-resource-mechanics), [all40-spec table](../../spell-mechanics-coverage.md). Eight focused native functions pass; the requested post-master `spell_cast` filter ran once and passed307/307 at823f5a4, with format/check exit0.168 Dummy roots remain,87 reviewed/81 un-derived. This is partial behavior, not whole-root closure. Generic optional spending intersects six roots/seven memberships, native resource duration three/four; these overlap. Guardian/imp rotations, damage copy, optional percentage costs, rank-specific resource caps and the Surge damage curve remain unresolved. Generic actors and aura81 redirects now have a separate [bounded implementation](#guardian-actors-and-damage-transfer). Native104317 names55659/3210, but runtime creature55659 and TDB smart-script spell bindings are absent; the empty TDB AIName/ScriptName cannot establish actor spell budget/cadence.

Sources: local Mainline12.1.0.69933 `SpellPower`, `SpellDuration`, `SpellEffect`, `Spell` descriptions; read-only native SQLite/TDB rows. Durable receipts/generator/ledger: `game-engine/data/diagnostics/spell-mechanics-2026-10-10/` (95 matching native rows, no semantic drift). No C++ body, inferred actor spell ID, database write, UDP5000, realm or deployment claim.

## Teleports and aura overrides

[Contract](../../specs/spell-teleports-overrides.md). Retail Blink1953 uses effect29 and target55's second radius (20yd); legacy teleport15 and current252 share dispatch. Forward movement samples GroundMap and stops 0.5yd before the first blocking baked LOS triangle using monotonic prefix queries. Missing ground/LOS never becomes an unchecked move. Same-map players use MovementControl's teleport epoch; cross-map moves reuse NewWorld/WorldPortAck.

Target9 uses HomeBind, persisted separately in redb character_home_binds and loaded at login. New characters persist their actual creation position in the character-creation transaction. Login only loads saved binds; existing characters without a row fail Hearthstone explicitly, never infer a bind from race or logout position. Innkeeper binding UI is not part of this change. Target17 reads authored `tdb_spell_target_position` from the established TDB1210.26091 archive through `import-azerothcore.sh` / `mysql_dump.py --prefix tdb_` and `tdb_schema.sql`. Import preserves the `(ID, EffectIndex, OrderIndex)` key and nullable Orientation; fixed destinations consume OrderIndex0. NULL orientation uses DifficultyID0 Retail SpellEffect.EffectPos_facing, converting degrees above 2π to radians, as TrinityCore a352b1fa SpellMgr::LoadSpellTargetPositions does. Rows with no matching Retail effect cannot supply a cast destination. Missing destinations still fail explicitly. Target106 ordered nearby selection is not implemented by this fixed-destination loader. Real TDB mage teleport3567 supplies the cross-map cast/ack fixture; no inferred city coordinates or alternate unprefixed table. Source: TrinityCore a352b1fa SpellMgr.cpp LoadSpellTargetPositions, Spell.cpp destination selection, SpellEffects.cpp EffectLeap/EffectTeleportUnits.

Harpoon190925's trigger186260 uses effect41; Feral Lunge196884 uses effect254/target53 then148. Both move the caster through the same endpoint/ground/LOS path, never the enemy. Effect254's target87 accepts SpellCastIntent.destination in Bevy coordinates, validates finite values and DB2 range, and retains it per cast/queue rather than in shared player state. Target grouping skips an earlier effect with unsupported targets: Infernal Strike189110's unhandled dummy87 cannot lead its supported jump254. This implements authoritative endpoints with movement teleports, not a parabolic motion presentation.

Aura332 resolves MiscValue-specific or SpellClassSet/mask substitutions using its amount as replacement ID. KnownSpells/action slots stay unchanged; removal/expiry restores originals. Queued requests retain the original ID and resolve the active override on release. UnitAuras now carries AuraOverride::ActionBar pairs and AuraOverride::Animation set IDs (aura312 MiscValue), including normal expiry/removal. Companion shared-protocol branch `spell-teleport-override` is required. Client must initialize new protocol fields, consume actionbar/animation overrides, and provide ground destinations; no client consumption or live rendering proof is claimed. TC a352b1fa Unit.cpp GetCastSpellInfo and SpellAuraEffects.cpp aura312 clientside mapping ground these behaviors.

## Temporary summons and spell area triggers

[Contract](../../specs/spell-summon-area-trigger.md). Effect 28 dispatches once per cast through `summon.rs`; `summon_properties` is loaded from world.db. `scripts/db2.py extract/import` adds local CASC SummonProperties (FDID 1345276, WDC5 layout A4CA5ECF). Missing metadata/template is logged, never synthesized. `SummonedCreature` records owner and SummonProperties; `UnitSummonedBy`, faction and `FollowOwner` reuse pet ownership/movement conventions without modifying PetStable. Positive durations expire on the aura clock; zero/negative durations are untimed. Owner removal/map departure also removes the summon.

Reference: TrinityCore a352b1fa `SpellEffects.cpp::EffectSummonType` / `SharedDefines.h::SummonTitle`. Matrix probe 49028 uses creature 27893, SummonProperties 3242 (ally/runeblade), duration 8000 ms. Other six summon roots' creatures are absent from current world data.

Effect 179 dispatches once per cast through `area_trigger.rs`. Radius is the first supplied SpellRadius (including the second radius slot); duration comes from the cast spell. An explicit EffectTriggerSpell executes through the normal triggered pipeline, on entry when EffectAuraPeriod is zero, periodically otherwise. Inside selection uses caster map, living units, radius and the trigger's explicit relation; depth and casting item level survive delayed area execution. Missing trigger/radius logs a lifecycle-only entity, not an invented action.

The nine matrix roots all lack trigger bindings, including Wildfire Bomb's child 265163. Purging Breath 1287715 → 1287711 supplies an authentic non-matrix damage binding for engine proof. Local AreaTriggerCreateProperties (FDID 1937227) contains none of these matrix create-property IDs: their server actions/scripts cannot be reconstructed from that client table. Selector87 destination propagation was added by master toy-effects4; area-trigger actions without bindings and moving-orb behavior remain unclaimed.

## Aura-owned native fields — 2026-10-17

[Contract](../../specs/follower-dungeons.md#azure-sourced-partial), [published Curve inputs](world-data.md#native-curve-type-inputs). Aura395 has a separate lifecycle from effect179: REAL application creates an actual `SpellAreaTrigger` plus an exact target/instance/effect binding; original caster and attached aura target remain distinct. Removal/death, missing ownership or map departure retires that field and undoes only its caster's action aura. Refresh does not duplicate or restart it; TC `SpellAuras.cpp::SetDuration/RefreshDuration` does not change the duration captured by the REAL-only create handler.

The catalog admits unscripted fixed spheres with one CAST/ENEMY action and supported native Type0/Flags0 scale curves. Other scripts/shapes/actions/modes and missing Curve IDs report explicit errors, never guessed radii/types. Native25356→29766 casts374523 once on entry; its1000ms periodic aura owns damage, not the field clock. Actual70448 Attributes5=524800/0x80200 includes native `EXTRA_INITIAL_PERIOD`0x200: one periodic tick on application, then1000ms ticks, not a fabricated extra entry hit. The first action fixture's zero-initial-tick assumption was wrong; production periodic handling remains unchanged. Sphere checks include source unit combat reach and strict3D distance. Effect179 retains its existing explicit-binding behavior. Unsupported NPC reach inputs remain explicit source gaps, not zero-sized units.

Published33927 points scale the source5yd sphere from1.25 to5 near6s. Actual374161 lasts300s, so the last live scale sample precedes expiry; the field must not be asserted alive at300s. Four new native fixtures at21f3045a reach shared missing-field REDs in87.99s after actual fourteen summons/self-auras; downstream action/scale/lifecycle assertions are not independent REDs. Initial5d4e3f6d execution passes five creation/lifecycle/scale/curve cases in121.30s; fixture-only ExtraInitialPeriod correction936dc142 passes the failed action case in37.90s. Those historical results are retained5 plus targeted1, not a fresh6/6 run. Subsequent70c01738 attachment/copy and self/ally/other-copy controls pass2; the real Garrick escape fails at4.838501yd inside radius4.997453+source reach1.5. Native-only per-unit envelope projection at931db8fe passes all six follower hazard cases24.75s, preserving legacy179 radii and explicit missing-reach warnings. The requested final Azure/aura packet freshly passes82/0failed/0ignored557.55s, including all enabled field controls; check/fmt pass with exact source/protocol/HEAD/clean/published-input guards. This does not establish the five missing follower-model reaches, native pixels or full Azure. Source: pinned TC a352b1fa `SpellAuraEffects.cpp::HandleCreateAreaTrigger`, `AreaTrigger.cpp::{CalcCurrentScale,SearchUnitInSphere,UndoActions}`, `Object.cpp::_IsWithinDist` and the actual70448 rows.

## Authoritative target LOS — 2026-10-16

[Contract](../../specs/line-of-sight.md#player-casts-authoritative-target-visibility). Stockade's moving Petty Criminal advanced about4yd during845ms between the native request sample and server receipt. The former client-endpoint check refused it despite a clear authoritative ray (`end_in_body=false`, segment clear, caster origin correct).

Player cast start now uses the same authoritative eye-to-eyes/chest/feet search as creatures. Client ray age cannot authorize or reject visibility; walls/live hulls still govern the search. Internal queued casts retain spell and target, not client geometry. Player cast completion keeps its existing start-only LOS policy. No tolerance increase or retry was introduced. Regression sends an ordinary loopback cast, moves its target4.25yd before delivery, and checks the resulting cast; doorway/wall controls distinguish stale evidence from genuine obstruction. Native release-server acceptance remains separate from this regression.

## Pipeline

1. **Receive** (`Update`): only an authenticated in-world connection's `PlayerLink` player casts. The spell resolves by ID or unique exact name. Explicit target = `target_entity`, else the server `CombatTarget`.
2. **Queue or validate:** after spell resolution and known-spell admission, hold one `QueuedSpellCast` (spell snapshot and explicit target) while cast/GCD remaining time is positive and at most 750 ms (user decision 2026-10-15; Retail/TrinityCore use 400 ms). New admitted requests replace it. No resource, target or cooldown validation at admission. Outside that window use immediate validation. `start_queued_casts` runs after `advance_casts` and due missile effects in the fixed chain, waits for both active cast and applicable GCD to end, takes the slot once and calls the normal start path. Full validation then runs in order:
   - player casters must know the spell (`class_progression::knows_spell` on [[class-progression]] `KnownSpells`; NPCs exempt; triggered spells skip it);
   - caster alive;
   - not stunned, silenced or pacified ([[auras]] crowd control);
   - no active `CastState`;
   - school not locked out;
   - GCD of the spell's `StartRecoveryCategory`;
   - spell and category cooldown;
   - charges;
   - moving (see below), when the cast bar or the channel would break on movement;
   - every handled effect's implicit targets supported;
   - explicit target;
   - costs.
3. **Explicit target** comes from the first handled effect that selects one:
   - enemy, ally or any relation;
   - alive;
   - `spell_range` min/max, hostile (`_0`) or friendly (`_1`); a max of 0 means 5 yd melee.
   - An ally spell with no valid ally target self-casts (retail auto self-cast).
   - Facing is not checked.
4. **Start:**
   - The GCD starts here: `StartRecoveryTime`, keyed by category, and sends `SpellCooldownUpdate{is_gcd}`.
   - Instant spells execute now.
   - Cast-time spells insert a replicated `CastState` and log `CastStart`.
   - Channeled spells (`SPELL_ATTR1` 0x4/0x40 with a positive duration) execute when their cast time, if any, is over (TrinityCore `Spell::handle_immediate`); `CastState::channel` then runs for the spell duration (Duration mods) and the channel's periodic effects tick as [[auras]].
5. **Execute** (instant, cast completion, before a channel), in order:
   - recheck caster, target and costs;
   - plan all targets;
   - spend costs;
   - start cooldown and charges;
   - log `CastSuccess`, send `SpellGo`, and put caster and targets in combat (`starts_combat`, [[combat]] Combat State);
   - resolve effects in index order, with each target's aura effects applied as one aura after the other effects. A spell with a missile `Speed` or `LaunchDelay` resolves on each unit target after its flight (`spell_cast/missiles.rs`, TrinityCore `Spell::HandleDelayed`): `LaunchDelay + max(max(distance, 5) / Speed, MinDuration)` (Spell.cpp:2486-2518), one missile for a destination spell (`CalculateDelayMomentForDst`, Spell.cpp:880-896). The delay is fixed at launch; a target that moved is still hit, one that died or left the world is not (Spell.cpp:2746-2748, 2807). Frostbolt at 20 yd lands 0.57 s after `SpellGo`; its damage, Chilled, threat, procs and combat log come then. With `SPELL_ATTR4_BOUNCY_CHAIN_MISSILES` (Avenger's Shield) each chained unit is hit the flight from the unit before it after that unit's hit, without another `LaunchDelay` (Spell.cpp:2489-2505). Spell reflection (`spell_cast/reflect.rs`, `WorldObject::SpellHitResult`, Object.cpp:1947-1978): a reflectable spell (`m_canReflect`: magic and not `IS_ABILITY`, or `ALLOW_SPELL_REFLECTION`; not `NO_REFLECTION`, `NO_IMMUNITIES` or passive) rolls each unit target's REFLECT_SPELLS(_SCHOOL) chance at launch; a reflect logs `Miss(Reflect)` with the launch, procs the reflect aura (`PROC_HIT_REFLECT`, which reflect auras alone proc off) at the target's delay and resolves the spell on its caster at 1.5 × that delay (Spell.cpp:2524-2545); `REFLECTION_ONLY_DEFENDS` deflects instead. A Frostbolt into Spell Reflection 23920 at 20 yd spends its charge at 0.57 s and hits the mage at 0.86 s. Speed-0 spells resolve in the launch tick;
   - push proc events (cast success, one spell hit per distinct target) and spend charged spell mods.
   - A failed completion sends `CastFailed`.
6. **Cancel:** stop requests, caster death and movement all remove `CastState`. An ended channel removes its caster's auras of that spell (`auras::channel_ended`) and the ritual objects it held (`meeting_stone::ritual::channel_ended`).

### Pending cast cancellation

Explicit `StopSpellCast` for an active cast removes its `QueuedSpellCast` before canceling the cast. Hostile interruption, movement and death use the existing interrupt/cancel paths and retain the pending slot: release revalidates movement, school lockout, death, targets and costs. A failed start sends owner `CastFailed` and consumes the slot. Policy, pinned sources and proof limits: [spell queue contract](../../specs/cast-presentation.md#spell-queue-window).

### Movement

- Casting still freezes translation (`networking_movement.rs`).
- `process_player_inputs` records `Moving(bool)`: whether the newest input has a direction.
- `advance_casts` runs after input and cancels a non-cast-while-moving cast when `Moving` is true.
- Cast-while-moving is TrinityCore `Spell::CheckMovement`: a cast bar breaks only with `SpellInterrupts.InterruptFlags` Movement (0x1); a channel breaks when `ChannelInterruptFlags_0` holds Moving or Turning (0x8 | 0x10, `SpellInfo::IsMoveAllowedChannel`). Summoning Stone Effect (59782) has InterruptFlags 0 and ChannelInterruptFlags 15406: its 5 s cast survives walking, its channel does not.
- `cast_granted` casts a spell a game object grants its user (meeting stones), without a spellbook check.
- `MovementSpeed` cannot serve as the signal: it holds run/walk speed even while idle.

### Spell modifiers

The caster's [[auras]] spell modifiers (107/108/218/219) adjust cast time (ChangeCastTime, then ÷ spell haste), channel duration, cooldown and category cooldown, GCD (StartCooldown), max range, costs (PowerCostN by `OrderIndex`), effect base points, coefficients, crit and damage/heal amounts.

### Costs

- Every `SpellPower` row with `RequiredAuraSpellID = 0` applies; a row with `RequiredAuraSpellID` applies only while the caster has that aura (priest spec passives gate most priest costs).
- Amount (raw units) = `ManaCost + PowerCostPct% × base`. For mana the base is `CharacterStats.base_mana` (gtBaseMP, [[character-stats]]); a caster without `CharacterStats` fails with `NotEnoughResource`. For other pools and health it is the max pool (TrinityCore `Spell::CalcPowerCost` uses the max for non-mana powers).
- Power type -2 costs health.
- A missing pool or insufficient power fails with `NotEnoughResource`.

### Cooldowns and charges

`SpellHistory` (server-only component) stores absolute ready times from `Time` elapsed seconds:

| What | Keyed by | Source |
|---|---|---|
| Spell cooldown | spell | `RecoveryTime` |
| Category cooldown | `Category` | `CategoryRecoveryTime` |
| GCD | `StartRecoveryCategory` | `StartRecoveryTime` |
| Charges | `ChargeCategory` | `spell_category` `MaxCharges` and `ChargeRecoveryTime` with charge auras 411/453/454, recharging one at a time |
| School lockouts | school bit | interrupts |

- `SpellCooldownUpdate` carries `max(recovery, category recovery)`.
- Haste scales a 750-1500 ms GCD only through MOD_GLOBAL_COOLDOWN_BY_HASTE_REGEN 417 whose class mask affects the spell: GCD ÷ (1 + haste), kept within 750-1500 ms (Spell.cpp:9220-9224; paladin passive 137026 for Divine Toll and Hammer of Light). The category-133 spell-haste rule for non-melee spells (Spell.cpp:9209-9218) is not modelled.

## Messages

| Message | Recipients | When |
|---|---|---|
| `CastFailed{reason, detail}` | caster owner | rejected request or failed completion |
| `SpellCooldownUpdate` | caster owner | GCD start; cooldown start |
| `SpellChargesUpdate` | caster owner | charge spent |
| `CombatLogEvent` | owners of source and target (once each) | CastStart, CastSuccess, Damage, Heal, Energize, Interrupt, Death, Miss(Immune), Summon; aura events in [[auras]] |

- Other clients in range do not receive combat log events yet.
- Melee auto-attack still broadcasts the old `CombatEvent` to every client. Retirement: remove it when the client renders `CombatLogEvent`.
- Spell effects emit only `CombatLogEvent`.
- Spell-created effect28 units and effect56 controlled pets emit `Summon` after creation: source=summoner, target=unit, spell_id=actual summoning spell (including triggered child spells). Existing-pet recall and login/script/journal restoration do not synthesize events. Temporary summon ancestry routes logs to the player's connection without altering meter/quest/PvP credit. See [guardian contract](../../specs/spell-guardians.md).
- `log_event` stamps every event with server wall-clock Unix milliseconds before recording damage or sending to owners, including queued melee events. Constructors leave the timestamp unset until that boundary.
- `Interrupt.extra_spell_id` identifies the interrupted cast; `Dispel.extra_spell_id` identifies the removed aura. Other kinds leave it `None`; `spell_id` remains the interrupting or dispelling spell.

Failures use the contract `CastFailReason` directly (`TooClose`, `SchoolLockedOut`, `SpellInProgress`, `NoChargesRemain`, ...). `NotSupported` carries the cause in `detail`: "unknown spell", "spell not known", "implicit target N not supported", "power type N". `NotInFront` is unused (no facing check).

## Implicit targets

Relations come from faction reactions ([[factions]]): enemy = `can_attack`, ally = `can_assist` (the caster is always its own ally). The hostile range applies when the caster's reaction to the target is Hostile, else the friendly range.

| IDs | Handling |
|---|---|
| 0 | none. If both slots are 0, the effect hits the explicit target, else the caster |
| 1 | caster |
| 6 / 21 / 25 | explicit enemy / ally / any |
| 18, 22 | area center = caster |
| 53 / 132 / 63 | area center = explicit enemy / ally / any |
| 15 / 30 | area around the source (default caster), enemy / ally |
| 16 / 31 | area around the destination (default explicit target, else caster), enemy / ally |
| 56 | caster-area party/raid members and their controlled units, same-copy/range filtered; ExcludeCaster before caps |
| 120 | caster |
| 57, 118 | explicit ally |

- For player casts, 56 checks `GroupRegistry` using player `ClientId` or a controlled unit's `UnitSummonedBy` owner. `SpellCastPlugin` initializes this existing group-state dependency. Ysera's Gift 145110's area filter reads live health, chooses one random target in the best injury/player tier, and retains a full-health target when no injured candidate exists (TC `SpellInfo.cpp:304`, `Spell.cpp:9516-9580`, `spell_druid.cpp:3100-3110`). Real native heal tests cover group membership, ownership, priority, range, departure and copy isolation; [contract](../../specs/class-completeness.md#raid-area-healing). Charmed controllers and TreatAsRaidUnit NPCs requiring unmodeled CreatureStaticFlags remain outside this proof; passive145108 now uses the [source-backed periodic hook](auras.md#script-hooks).
- Radius: slot A uses `EffectRadiusIndex_0`; slot B uses `_1`, falling back to `_0` (TrinityCore `CalcRadius`).
- Area results are nearest first, capped by `SpellTargetRestrictions.MaxTargets`.
- 87/104/134 now support selected destination, authored destination-directed enemy cone and line;47 selects the caster-front destination. Other unimplemented IDs still fail a handled cast.
- Effects with the same implicit targets (and, for areas, the same radii) share the first one's selection, as `Spell::SelectSpellTargets` (Spell.cpp:741-781): Avenger's Shield's interrupt and silence hit every unit its damage chains to.
- Chains (`targets::chain_targets`, `Spell::SearchChainTargets`, Spell.cpp:2216-2320): an explicit unit target with `EffectChainTargets` > 1 jumps to the living unit nearest the last one within the jump radius (7.5 yd ranged, 5 yd melee, 10 yd otherwise), searched within the radius × the jumps left; `SPELL_ATTR2_CHAIN_FROM_CASTER` searches from the caster over the spell range and `ChainFromInitialTarget` keeps the first target as the source. Each jump multiplies damage and healing by `EffectChainAmplitude` (`m_damageMultipliers`, Spell.cpp:8631-8636). Not modelled: line of sight, the melee front arc (`SPELL_ATTR5_MELEE_CHAIN_TARGETING`), chain heal (target 45 is unsupported) and the ChainTargets/ChainJumpDistance/ChainAmplitude spell mods.
- Triggered spells currently retain a relation bypass for explicit units; this is not full native flag parity. Explicit target aura eligibility is checked separately [above](#triggered-explicit-target-aura-eligibility--2026-10-16); full-mask flags do not include IGNORE_TARGET_CHECK.

## Effects

Registry: `effects::handler(effect_id)`. Unhandled effect IDs are skipped and logged once per (spell, effect).

| ID | Name | Handling |
|---|---|---|
| 2 | SCHOOL_DAMAGE | scaled amount (below), crit, shared death path |
| 6, 35, 65 | APPLY_AURA, APPLY_AREA_AURA_PARTY/RAID | grouped per target, `auras::apply_aura` |
| 10 | HEAL | scaled amount, crit, clamped to max, overheal logged |
| 30 | ENERGIZE | Points/PointsIndexN-modified base points in raw units of `EffectMiscValue_0`, clamped, excess logged |
| 31 | WEAPON_PERCENT_DAMAGE | `BasePoints`% of a weapon hit |
| 38 | DISPEL | `auras::dispel` of `EffectMiscValue_0`, up to `BasePoints` (min 1) auras |
| 58, 121 | WEAPON_DAMAGE, NORMALIZED_WEAPON_DMG | weapon hit + `BasePoints` |
| 64 | TRIGGER_SPELL | casts `EffectTriggerSpell` at each target, bounded (below) |
| 68 | INTERRUPT_CAST | cancels an interruptible `CastState` and locks the interrupted spell's school mask for this spell's duration (Kick 1766: 6 s) |
| 136 | HEAL_PCT | `BasePoints`% of target max health |

TRIGGER_SPELL:
- The triggered spell ignores cost, cooldown, GCD and cast time.
- Depth is capped at `MAX_TRIGGER_DEPTH` = 3 hops, the same as the coverage closure.

Spell scripts (`spell_cast/scripts.rs`, TrinityCore `SpellScript`): behaviour the DB2 effects do not carry, keyed by spell ID. `on_hit` runs once per distinct planned unit after the spell's effects and auras resolved there (`SpellHitFn`, `Spell::DoSpellHitOnUnit`), before the hit proc events.
- Frostbolt 116: its only effect is TRIGGER_MISSILE 32 to 228597 (SCHOOL_DAMAGE); neither carries a slow. The hook casts Chilled 205708 (APPLY_AURA MOD_DECREASE_SPEED 33, -50%, `SpellDuration` 31 = 8 s, `SPELL_ATTR0_AURA_IS_DEBUFF`) at the hit unit, triggered (TrinityCore `spell_mage_frostbolt::HandleChilled`, spell_mage.cpp:978-995, registered on 116 in sql/old/9.x/world/22012_2022_03_06/2022_02_10_02_world.sql:5).
  A Frostbolt-pulled Blackrock Spy chases at 3 yd/s instead of 6 (game-server `polymorph_tests.rs`).

New-class scripts use per-effect `on_effect_hit` hooks; a DUMMY effect with a hook participates in target validation/planning rather than being silently skipped. `area_target_filter` narrows selected area targets before hit resolution.
- Chaos Strike Initial 344862 effect 0 triggers 162794, whose effects trigger the existing damage spells 222031/199547 (TrinityCore `spell_dh_chaos_strike_initial`).
- Living Flame 361469 effect 0 triggers heal 361509 for an assistable target or damage 361500 otherwise (`spell_evo_living_flame`). Energizing Flame's mana refund remains unimplemented.
- Azure Strike 362969 effect 1 retains the explicit enemy plus effect 0's baseline count minus one other selected enemy (`spell_evo_azure_strike`). Other enemies are nearest-first, not TrinityCore's random selection; count-changing talent modifiers remain unimplemented.
- Tiger Palm 100780 already has SCHOOL_DAMAGE and ENERGIZE effects. ENERGIZE now applies existing spell-point modifiers: Windwalker passive 1258122 adds 2 Chi to effect 1's zero baseline. Blackout Kick uses existing damage and aura-gated cost handlers. Power pool sizing/spec/form selection remains separate powers-agent work.

Formulas:
- **Scaled amount:** `BasePoints + BonusCoefficient × SP + BonusCoefficientFromAP × AP`.
  - SP and AP are `CharacterStats.spell_power` / `attack_power` ([[character-stats]]).
  - Units without `CharacterStats` (NPCs) have SP = AP = 0.
- **Crit:** `CharacterStats.crit_pct` (5% base for players, 0 for NPCs; MOD_CRIT_PCT auras fold in through `StatModifiers`) + CritChance mods. A crit adds 100% of the amount, scaled by CritDamageAndHealing mods.
- **Damage and heal mods:** see [[auras]]: spell mods, done/taken multipliers, immunity and absorbs.
- **Weapon hit:** a roll in the main hand's `[min, max]` + `AP / 6 × speed`; 121 uses the weapon type's normalized speed. Main-hand damage comes from the equipped weapon ([[character-stats]] Gear), unarmed 1–2 at 2.0 s.

Damage and heal side effects:
- Damage sets `LastDamager` and puts source and target in combat with each other (`combat_refs::set_in_combat_with`, [[combat]] Combat State).
- A lethal hit on an NPC applies `DeathState` and removes `Replicate`.
- A lethal hit on a player writes `PlayerDeathTransition`.

## Powers

- **Pool list:** the class list comes from `chr_classes_x_power_types`, minus the alternate bars 10/23/24/25.
- **Order:** `chr_classes.DisplayPower` first, then ascending ID. Example: DK = Runic Power, Runes.
- **Initial values:** each non-mana pool starts at `DefaultPower`, with max `MaxBasePower`.
- **Max** (`Player::UpdateMaxPower`): `MaxBasePower` + MOD_INCREASE_ENERGY 35 + MOD_MAX_POWER 418 of that power ([[auras]]), rounded; a lower max clamps current. `power_type` holds the retail base, passives the rest: Holy Power 3 + Boundless Conviction 115675 (+2, auto-learned) = 5; Chi 4 + Windwalker Monk 1258122 (+1) = 5, + Ascension 115396 = 6; Runes 6 + Runic Longevity 194653.
- **Display power** (`Unit::CalculateDisplayPowerType`): Cat Form and Ghoul → Energy, Bear/Dire Bear → Rage, Travel/Ghost Wolf → Mana, otherwise `DisplayPower`. The display power leads `order` (the first `UnitPowers` entry); the rest stay ascending. See MOD_SHAPESHIFT in [[auras]].
- **Runes** (`spell_cast/runes.rs`, TrinityCore `Runes`, `SetRuneCooldown`, `Player::RegenerateAll`, `Spell::TakeRunePower`): per-rune cooldowns, not a regenerating pool. A spent rune (lowest ready index first) gets the rune cooldown and joins the back of the depletion order; only the first 3 (`MAX_RECHARGING_RUNES`) recharge, a finished rune leaves the order and its leftover time is dropped. The Runes pool holds the ready count, so costs check it like any pool; ENERGIZE of runes finishes the front of the order. Cooldown: 10 s (`RUNE_BASE_COOLDOWN`) ÷ (MOD_POWER_REGEN_PERCENT on runes × (1 + haste)); TrinityCore multiplies by (1 − pct) instead.
- **Mana** stays in the legacy `Mana` component, with its max from `stats.rs`. `PowerPools` only orders it. Retirement: move mana into `PowerPools` and delete `Mana` when the client reads `UnitPowers`.
- **Units:** pools hold the raw DB2 values, the same units as `SpellPower` costs and ENERGIZE.
  - Rage, Runic Power, Soul Shards, Lunar Power and Pain are tenths (`DisplayModifier` 10). Insanity is hundredths.
  - `UnitPowers` sends raw `current`/`max` floored to integers, `partial` fractional thousandths and signed `regen_per_sec`; `charged_points` contains the aura-derived 1-based combo-point indices. The client divides by the display modifier for `PowerType`.
  - Resend on whole value, max, rate, display order or charged-point changes, not partial alone. The sent fraction includes that tick's regeneration. The rate becomes zero on the tick a pool reaches its max or decay center.
  - `UnitRunes` sends `duration_ms` and rune-indexed `ready_in_ms`; queued runes include their wait, so values can exceed duration. Resend on depletion-order, duration or rune-count changes, not countdown ticks. A max-only rune passive must update the view even when every rune is ready.
- **Regen** (raw units per second, TrinityCore `Player::Regenerate`):
  - In combat: `RegenCombat`.
  - Out of combat: `RegenPeace`, starting only after `RegenInterruptTimeMS` since leaving combat.
  - A positive rate fills toward max. A negative rate moves toward `CenterPower`: Rage/RP decay to 0, and Soul Shards settle at 3.
  - Examples: Energy +10/s. Rage −12.5/s (−1.25 rage/s) out of combat only. Holy Power −0.1/s after 10 s.
  - Mana has rate 0 in the data. It regenerates 1% of base mana (gtBaseMP, `CharacterStats.base_mana`, not the pool max) per second, in and out of combat, times the aura regen multiplier (TrinityCore `Player::UpdatePowerRegen` `basemana / 100`; SimC `init_resources`).
  - Class/spec passives supply the multiplier via MOD_MANA_REGEN_PCT/MOD_POWER_REGEN_PERCENT: class −36% (Paladin, Priest, Shaman, Druid), healer spec +25%, hybrid spec +60%, Arcane +60%. Level 80 (base 50 000): Holy Paladin 400/s, Frost Mage 500/s, Arcane Mage 800/s.
  - Not implemented: flat MOD_POWER_REGEN 85 (per 5 s); haste scaling of Energy/Focus regen (`power_type` flag `RegenAffectedByHaste` 0x400).
  - No five-second rule: Mana lacks `UseRegenInterrupt` (0x2), so spending mana pauses nothing (TrinityCore `Player::GetPowerRegen`). Focus +5/s in and out of combat; Runic Power holds in combat and decays −12.5/s (−1.25/s) out of combat.
  - Dead players regenerate nothing (`Player::Update` calls `RegenerateAll` only while alive).
- **Player health regen** (`powers::regenerate_player_health`, TrinityCore `Player::RegenerateAll`/`RegenerateHealth`, Player.cpp:1594-1802): `PlayerHealthRegen` counts ms while alive. Every 2000 ms, out of combat, health gains 20% of max ÷ level below level 15, else 1.5% of max. The whole part is added and the fraction carried; reaching max sets max and clears the fraction. Out of combat that base is × MOD_HEALTH_REGEN_PERCENT 88, plus 0.4 × MOD_REGEN 84 (per 5 s to per 2 s); in combat only MOD_REGEN_DURING_COMBAT 116 regenerates, `Rate.Health` (1) × its percent. Either way ×1.5 while not standing ([[stand-state]]). Always added: MOD_HEALTH_REGEN_IN_COMBAT 161 and gear `ITEM_MOD_HEALTH_REGEN` ÷ 2.5 (`CharacterStats.health_regen`, `m_baseHealthRegen`). In combat it runs only with 116, 161 or gear regen; while polymorphed, max ÷ 3 even in combat. Level 10, max 1110: 22.2 per tick, 666 → 1110 in 20 ticks (40 s).
- **SpellScaling amounts** (`effects::calc_base_value`, TrinityCore `SpellEffectInfo::CalcBaseValue`): an effect with a scaling `Coefficient` is its scaling class column of gtSpellScaling (`spell_scaling_data.rs`, generated by `scripts/gen_spell_scaling_table.py` from local CASC `gametables/spellscaling.txt`) at the caster's level, clamped to its `spell_scaling` range, × the coefficient. Food 1127 at level 10: Consumable 1000 × 0.25 = 250 MOD_REGEN. Not modelled: `SPELL_ATTR11_SCALES_WITH_ITEM_LEVEL` (item-level RandPropPoints; such spells keep BasePoints), `SPELL_ATTR8` target level, the casting item's -6/-7 multipliers.
- **Rage from auto-attacks** (`combat.rs` `reward_swing_rage`, Retail values from SimC `sc_warrior.cpp`; AzerothCore's Wrath `RewardRage` damage/level conversion is not used). Only units whose primary power is Rage (AzerothCore `HasActivePowerType(POWER_RAGE)`):
  - Dealt: every landed swing gives swing time (2 s) × 1.75 rage × (1 + Σ MOD_RAGE_FROM_DAMAGE_DEALT 213 %), rounded to 0.1 rage (`melee_t::trigger_rage_gain`). Unspecced 3.5 rage; Arms Warrior 137049 (+250%) 12.3 rage; Protection 1258163 (−56%).
  - Taken: a damaging swing taken gives 3 rage, at most once per second (`warrior_t::assess_damage`).
- **NPCs** get `UnitLevel` (template `min_level`) and empty `UnitPowers`. Creature class/level power stats are not imported.
- **Players:** `UnitLevel` is owned by [[class-progression]]; the cast pipeline only reads levels.

## Sources

- [spell_cast/mod.rs](../../../crates/server/src/spell_cast/mod.rs) — pipeline, `SpellHistory`, messages
- [spell_cast/targets.rs](../../../crates/server/src/spell_cast/targets.rs) — implicit targets, range
- [spell_cast/effects.rs](../../../crates/server/src/spell_cast/effects.rs) — handlers, damage/heal paths
- [spell_cast/powers.rs](../../../crates/server/src/spell_cast/powers.rs) — `PowerTypeTable`, pools, regen, `UnitPowers`
- [spell_cast/runes.rs](../../../crates/server/src/spell_cast/runes.rs) — Death Knight rune cooldowns
- [spell_cast/class_resource_tests.rs](../../../crates/server/src/spell_cast/class_resource_tests.rs) — maxima, forms, runes, charged points
- [spell_cast/tests.rs](../../../crates/server/src/spell_cast/tests.rs) — real-DB App tests
- [spell_cast/test_harness.rs](../../../crates/server/src/spell_cast/test_harness.rs) — shared real-DB harness (also used by aura tests)
- [spell_cast/new_class_tests.rs](../../../crates/server/src/spell_cast/new_class_tests.rs) — six core spells with local DB2 and HostClient delivery
- [new-class spec](../../specs/new-classes.md) — bounded behavior and remaining integration work
- [spec](../../specs/cast-presentation.md)

## See Also

- [[spell-info]] — the data this resolves
- [[character-stats]] — spell power, attack power, crit, base mana
- [[auras]] — aura runtime, procs, spell and stat modifiers
- [[combat]] — melee, combat state, death
- [[networking]] — input ordering, replication dirty queue

## Eleventh batch: local Mainline low-count paths

Implementation in `spell_cast/scripts/batch11.rs` and the existing fixed-field helper; [contract](../../specs/spell-dummy-scripts.md#eleventh-batch-bounded-low-count-paths). Final native batch13/13 and inherited Colossus1/1 at`bce19e3`; format/check exit0, warning-free. Requested post-master `cargo test -p server --bin game-server spell_cast` ran ONCE at`d369cf6`:298/299, with the sole Colossus counterexample to the rejected universal271 rule. Named334168 correction restores that focused proof; no fresh299/299 broad claim. Reproducible census/receipts in `game-engine/data/diagnostics/spell-dummy11-2026-10-10/`.198 compared local raw/imported rows have zero semantic drift;17 CRLF/LF-only string differences recorded separately.

Local Mainline12.1.0.69933 sources:193315 description-variable338 defines `(clamp(level-10,0,10)*8+195)/275`;228260 names1242173, whose s1/s3 name ten/one1242189 bolts and s2 retains1000 raw Insanity. Native bolt travel/defense remains shared; no guessed inter-bolt cadence/replacement charges.5740 names42223m1*8: duration8000ms, effect2 period1000ms, effect0 radius8yd. Fixed fields reuse the independently clocked batch9 implementation; overlap is not replacement.

184575's native404358 trigger is conditional on403826; script admission no longer grants untalented splash. The native radius is8yd although the local tooltip says6yd; use the native row and retain this discrepancy.53595 names88263 for OTHER enemies, conditioned on caster position inside owned26573; field ownership/map/expiry/radius determine that condition. Only the named damage effect executes, not its unknown markers/talent child.

5221's343232/405834 stealth condition derives60% extra damage, double crit chance and343232s1 extra point; earlier bleeding-rank bonus composes.106830 names405233's3000ms bleed.60103/334046 name334168's100% Flame Shock vulnerability; the existing generic42 proc already applies it, so no redundant apply hook was added. Existing aura271 amplified unrelated spells under334168 despite its Flame Shock-only description; that named consumer now uses existing `class_mask_affects` and retains foreign-caster exclusion. A universal nonempty-mask rule failed the inherited Colossus Smash proof:208086's tooltip promises all owner damage despite a nonempty mask. The final correction is therefore named334168 only, preserving Colossus's existing base contract rather than inventing universal271 semantics. Optional Flame Shock spread, weapon-item Flametongue state, Shred405300/Thrash429523 and unknown Dummy markers remain open.

Census frozen before`3b25ebe`:168 Dummy-bearing core roots,83 reviewed/85 un-derived. After final Rust`bce19e3`: **168**,84 reviewed/84 un-derived. Shred5221 closes its entire E3 field; Hammer53595 exposes latent88263 E3 instead of hiding it. E3 remains120 IDs/128 memberships, a4 remains67/75; a226 falls19/20→18/19 through the Rain clock. Scope13 classes/40 specs,630 book IDs/1551 memberships and302 core IDs/355 memberships unchanged. This is not whole-ability parity.

Selection raises core script presence: Outlaw1→2, Retribution1→2, Protection2→3, Destruction2→3, Feral3→4. Shadow's already-bound Voidform improves without inflating its root count. Enhancement improves bounded Core A4→5 via the named271 consumer correction, not a fabricated per-spell binding. Transfer, imps/pet actors, mana scaling and unavailable source/geometry contracts remain open.

All40-spec before/after table preserves batch10's exact baseline method; book/core closure intersects explicit scripts/consumed fields. Named271 correction adds proof, not a script count.

| Spec | Book script roots before→after | Core script roots before→after | Core A before→after |
|---|---:|---:|---:|
| Mage Arcane (62) | 2→2 | 2→2 | 3→3 |
| Mage Fire (63) | 1→1 | 2→2 | 4→4 |
| Mage Frost (64) | 1→1 | 2→2 | 4→4 |
| Paladin Holy (65) | 1→1 | 2→2 | 4→4 |
| Paladin Protection (66) | 2→2 | 2→3 | 3→4 |
| Paladin Retribution (70) | 2→2 | 1→2 | 2→3 |
| Warrior Arms (71) | 1→1 | 1→1 | 5→5 |
| Warrior Fury (72) | 2→2 | 4→4 | 6→6 |
| Warrior Protection (73) | 2→2 | 3→3 | 4→4 |
| Druid Balance (102) | 5→5 | 5→5 | 5→5 |
| Druid Feral (103) | 4→4 | 3→4 | 3→4 |
| Druid Guardian (104) | 4→4 | 3→3 | 4→4 |
| Druid Restoration (105) | 4→4 | 3→3 | 3→3 |
| Death Knight Blood (250) | 2→2 | 2→2 | 3→3 |
| Death Knight Frost (251) | 2→2 | 2→2 | 4→4 |
| Death Knight Unholy (252) | 2→2 | 3→3 | 4→4 |
| Hunter Beast Mastery (253) | 2→2 | 2→2 | 3→3 |
| Hunter Marksmanship (254) | 2→2 | 3→3 | 3→3 |
| Hunter Survival (255) | 2→2 | 2→2 | 2→2 |
| Priest Discipline (256) | 2→2 | 3→3 | 6→6 |
| Priest Holy (257) | 1→1 | 4→4 | 6→6 |
| Priest Shadow (258) | 1→1 | 2→2 | 5→5 |
| Rogue Assassination (259) | 2→2 | 2→2 | 4→4 |
| Rogue Outlaw (260) | 1→2 | 1→2 | 3→4 |
| Rogue Subtlety (261) | 2→2 | 2→2 | 4→4 |
| Shaman Elemental (262) | 1→1 | 4→4 | 5→5 |
| Shaman Enhancement (263) | 2→2 | 3→3 | 4→5 |
| Shaman Restoration (264) | 1→1 | 2→2 | 3→3 |
| Warlock Affliction (265) | 0→0 | 3→3 | 5→5 |
| Warlock Demonology (266) | 0→0 | 2→2 | 4→4 |
| Warlock Destruction (267) | 1→1 | 2→3 | 4→5 |
| Monk Brewmaster (268) | 3→3 | 5→5 | 6→6 |
| Monk Windwalker (269) | 3→3 | 2→2 | 3→3 |
| Monk Mistweaver (270) | 4→4 | 3→3 | 3→3 |
| Demon Hunter Havoc (577) | 4→4 | 2→2 | 3→3 |
| Demon Hunter Vengeance (581) | 3→3 | 3→3 | 3→3 |
| Evoker Devastation (1467) | 4→4 | 4→4 | 5→5 |
| Evoker Preservation (1468) | 4→4 | 3→3 | 4→4 |
| Evoker Augmentation (1473) | 6→6 | 3→3 | 4→4 |
| Demon Hunter Devourer (1480) | 4→4 | 2→2 | 3→3 |

## Class Dummy scripts

### Tenth batch: low-count spec paths

[Contract](../../specs/spell-dummy-scripts.md#tenth-batch-low-count-spec-paths), [implementation](../../../crates/server/src/spell_cast/scripts/batch10.rs), [tests](../../../crates/server/src/spell_cast/dummy10_tests.rs), [updated census](../../spell-coverage.md). Verified2026-10-10. Sources: local Mainline12.1.0.69933 DB2 CSVs/imported read-only rows only;21 authentic IDs/251 compared rows,zero drift. No C++ implementations, invented IDs, world.db writes or realm process.

| Path | Observable native proof | Retained boundary |
|---|---|---|
|13877 Blade Flurry|331850 initial8yd strike caps five/eight with272026;381878 grants one combo point per actual initial hit. Far/allied exclusions and absent-rank/talent proofs.|Subsequent28% damage transfer, independent root markers/cap4.|
|26573 Consecration|Fixed-position81297 field:8yd,1000ms,12000ms; original caster position survives movement. Recast replaces only owner's old field; foreign field continues. Tooltip1.05 multiplier retained.|204054 slow/405289 Radiant variants, summoned actor parity.|
|196884 Feral Lunge|Named215802 bite hits only selected enemy beyond native minimum range; inherited generic254 endpoint movement remains covered by its existing terrain/LOS fixture.|Full traversal/visual parity; named-bite proof alone is not movement certification.|
|228260 Voidform|Named194249 damage effects apply after1500ms cast; actual Mind Blast+20%, native expiry and cancellation exclusion.|Initial Volley completed in batch11; replacement charges and root/child markers remain.|
|365350 Arcane Surge|Named365362 damage/regeneration effects apply after2500ms cast; actual Fireball+35%, expiry and cancellation exclusion.|Mana-spent damage, reduced-damage formula, Clearcasting.|
|275779 Protection Judgment|315867 gate launches220637 for one Holy Power on native damage hit; no-rank case generates none.|Other Judgment/talent variants.|
|105174 Hand of Gul'dan|Native30-raw cost (three shards), root s1=3 times86040 damage in native8yd target area; allies/far excluded.|Imp/Doom/variable-shard contracts; rootDummy0 intentionally remains counted because s1 also describes imp count.|
|188196 Lightning Bolt|343725s1=6 Maelstrom awarded once on actual hit; absent-rank proof.|Maelstrom Weapon consumption/other variants.|

Census frozen baseline`e64b2da` reproduces170 Dummy core roots. After Rust`83512eb`: **168** remain,83 reviewed/partial/deferred,85 un-derived (before80/90). Consecration's named periodic Dummy and Feral Lunge's named bite close two Dummy-bearing roots; no whole-ability parity claim. E3 admissible121/129→120/128; a4 66/74→67/75; a22619/20 unchanged. Voidform's named child exposes latent Dummy fields rather than concealing them. Scope13 classes/40 specs,630 book IDs/1551 memberships,302 core IDs/355 memberships remains unchanged.

Selection prioritizes core-zero Protection/Outlaw and core-one Arcane/Enhancement/Shadow/Demonology. Protection0→2, Outlaw0→1, Arcane1→2, Enhancement1→3, Shadow1→2, Demonology1→2; shared Lightning Bolt also improves Elemental3→4. Divine Storm's reduction formula and Glacial Advance's advancing geometry were not derivable from the permitted sources, so no guessed implementation. Prior Winter diagnostics overlooked slot1 radius14 (8yd); existing proof remains coincident-unit only, not a new range implementation.

Script-presence table preserves batch9's exact method/baseline: closure intersects explicit script bindings/consumed fields; named Meta/Fracture modifier probes remain core-only as in the original table. A counts bounded test-source evidence, never full ability parity.

| Spec | Book script roots before→after | Core script roots before→after | Core A before→after |
|---|---:|---:|---:|
| Mage Arcane (62) | 2→2 | 1→2 | 2→3 |
| Mage Fire (63) | 1→1 | 2→2 | 4→4 |
| Mage Frost (64) | 1→1 | 2→2 | 4→4 |
| Paladin Holy (65) | 0→1 | 2→2 | 4→4 |
| Paladin Protection (66) | 0→2 | 0→2 | 1→3 |
| Paladin Retribution (70) | 1→2 | 1→1 | 2→2 |
| Warrior Arms (71) | 1→1 | 1→1 | 5→5 |
| Warrior Fury (72) | 2→2 | 4→4 | 6→6 |
| Warrior Protection (73) | 2→2 | 3→3 | 4→4 |
| Druid Balance (102) | 5→5 | 5→5 | 5→5 |
| Druid Feral (103) | 4→4 | 3→3 | 3→3 |
| Druid Guardian (104) | 4→4 | 3→3 | 4→4 |
| Druid Restoration (105) | 4→4 | 3→3 | 3→3 |
| Death Knight Blood (250) | 2→2 | 2→2 | 3→3 |
| Death Knight Frost (251) | 2→2 | 2→2 | 4→4 |
| Death Knight Unholy (252) | 2→2 | 3→3 | 4→4 |
| Hunter Beast Mastery (253) | 2→2 | 2→2 | 3→3 |
| Hunter Marksmanship (254) | 2→2 | 3→3 | 3→3 |
| Hunter Survival (255) | 2→2 | 2→2 | 2→2 |
| Priest Discipline (256) | 2→2 | 3→3 | 6→6 |
| Priest Holy (257) | 1→1 | 4→4 | 6→6 |
| Priest Shadow (258) | 1→1 | 1→2 | 4→5 |
| Rogue Assassination (259) | 2→2 | 2→2 | 4→4 |
| Rogue Outlaw (260) | 0→1 | 0→1 | 2→3 |
| Rogue Subtlety (261) | 2→2 | 2→2 | 4→4 |
| Shaman Elemental (262) | 0→1 | 3→4 | 4→5 |
| Shaman Enhancement (263) | 0→2 | 1→3 | 2→4 |
| Shaman Restoration (264) | 0→1 | 2→2 | 3→3 |
| Warlock Affliction (265) | 0→0 | 3→3 | 5→5 |
| Warlock Demonology (266) | 0→0 | 1→2 | 3→4 |
| Warlock Destruction (267) | 1→1 | 2→2 | 4→4 |
| Monk Brewmaster (268) | 3→3 | 5→5 | 6→6 |
| Monk Windwalker (269) | 3→3 | 2→2 | 3→3 |
| Monk Mistweaver (270) | 4→4 | 3→3 | 3→3 |
| Demon Hunter Havoc (577) | 4→4 | 2→2 | 3→3 |
| Demon Hunter Vengeance (581) | 3→3 | 3→3 | 3→3 |
| Evoker Devastation (1467) | 4→4 | 4→4 | 5→5 |
| Evoker Preservation (1468) | 4→4 | 3→3 | 4→4 |
| Evoker Augmentation (1473) | 6→6 | 3→3 | 4→4 |
| Demon Hunter Devourer (1480) | 4→4 | 2→2 | 3→3 |

Native proof at`83512eb`: **11/11** batch10 functions, warning-free. Initial RED0/9 plus corrected Lunge range RED0/1; negative-rank fixtures remove KnownSpells passives and their aura (omitting apply() alone leaves auto-synchronized passives active). Damage buffs are proven through actual spell damage, not school-only AuraStatMods. Shared-protocol compilation drift resolved by merging origin/master before implementation. Requested post-master `cargo test -p server --bin game-server spell_cast` ran **once**, at`4b281cf`: **286/286**, including all11 new functions and inherited Feral Lunge movement. Format/check results are recorded in the handoff.

Durable receipts, frozen source tars, generator, exact40-spec inventories, census and proof ledger: `game-engine/data/diagnostics/spell-dummy10-2026-10-10/`; worker handoff `/home/osso/.worktrees/handoff-spell-dummy10.md`. Parent retains integration/live acceptance; no independent verifier/rotation claim.

## Ninth batch: local Mainline ground fields and owned marks

[Contract](../../specs/spell-dummy-scripts.md#ninth-batch-locally-derived-paths), [implementation](../../../crates/server/src/spell_cast/scripts/batch9.rs), [tests](../../../crates/server/src/spell_cast/dummy9_tests.rs). Sources are local Mainline12.1.0.69933 DB2 CSVs and read-only imported rows plus inherited TDB1210 routing, not TrinityCore C++ bodies.22 authentic IDs/185 rows match slot and canonical databases exactly. No downloads/imports/world writes or realm process.

| Path | Implemented / observable proof | Remaining boundary |
|---|---|---|
|73920 Healing Rain|Fixed requested11yd field;2000ms pulses through73921, six injured allies,18sec native lifetime. Recast replaces this caster's previous field; entering/leaving allies evaluated each pulse.|14/magic-reduction/aura4 markers and1298679 child. Tooltip fixed6*2 formula disagrees with18sec native duration; no total-tooltip parity claim.|
|61882 Earthquake|77478 base damage at requested8yd field, six1000ms pulses including expiry; two real casts retain independent clocks, ally/far exclusions.|Knockdown77/chance and extra Dummy metadata.|
|355913 Emerald Blossom|1500ms delayed355916 heal of three injured allies within10yd, anchored at original ally position; moving primary/healthy/far excluded.|Root zero Dummy1 and variants.|
|196770 Remorseless Winter|Actual196771 hit applies21179320% slow; expiry restores movement.|Coincident-unit native proof only. Batch10 source correction: child slot1 references radius14 (8yd); range is not newly tested here. Root25 marker/CAP remain.|
|55090 Scourge Strike|Missing owned191587 base DoT applies/ticks; existing own mark not refreshed as substitute for spreading.|35% plague eruption, spread and root3 marker.|
|263642 Fracture /187827 Meta|Named321068 gate on Meta's native15 Fury modifier; base25 vs transformed talented40, missing-rank/Meta boundaries.|Soul fragments/objects, Meta periodic marker; no complete Meta parity.|
|1234796 Void Step|Requested same-map teleport; missing/nonfinite/out-of-range refused before spending.|This is the sole whole Dummy-root closure; no unrelated mobility/terrain parity claim.|
|194509 Radiance|Selected allies receive194384 for60% normal duration (9000ms); independent foreign owners persist.|Atonement damage-transfer/raid-pet priority and other Dummy fields.|
|740 Tranquility|Named60% caster-max-health shield absorbs concrete incoming damage; channel cancellation removes remaining capacity. Depletion preserves channel healing rather than consuming the entire aura.|Ally Dummy marker, soft cap and HoT extension.|

Once-per-cast destination hooks run even in empty areas. Field pulses use normal defended-target, missile, combat and damage/heal paths, not a guessed custom damage formula. Root/child fields provide radii/caps/delay/cadence; no invented spell IDs.29 now admits the native ally dynamic-object selector, but admission is not new raid propagation. Rain/Quake/Void Step require finite, native-range requested points rather than silently moving an omitted destination to the caster.

Census baseline`2ab5e5e` reproduced171 Dummy-bearing core roots; after Rust`e78fad0`, **170** remain (Void Step closes). Reviewed/partial/deferred73→80, un-derived98→90; E3 admissible123/131→121/129, a4 64/72→66/74 (D1→0), a22619/20 unchanged. Radiance exposes child Dummy markers; Tranquility moves from D to admitted A without concealing unresolved fields. Scope13 classes/40 specs,630 book IDs/1551 memberships,302 core IDs/355 memberships unchanged.

Per-spec script counts derive census membership/closure plus explicit cast/aura/field consumers, not generic A damage tests. Before core counts: Protection Paladin/Outlaw0, Restoration Shaman/Frost DK/Vengeance1. Resolvable selections improve Restoration Shaman1→2, Frost DK1→2, Vengeance1→3, Elemental2→3, Unholy2→3, Preservation2→3, Devourer1→2, Restoration Druid2→3; Priest count unchanged while Radiance's existing path improves. Zero-script Protection/Outlaw death interception/random-roll mechanics require a broader independently sourced contract, not fabricated substitutions. Complete40-spec before/after table: `game-engine/data/diagnostics/spell-dummy9-2026-10-10/per-spec-scripts.md`.

Native proof: behavioral RED initially0/11,11/11 GREEN at`6b11521`; new overlap/empty-area/cancellation boundaries give14/14 at`7c471f0`. Separate requested-ground RED at`eb96054`; malformed Maelstrom fixture corrected and nine refusal cases pass at`314a8f1`. Requested post-master full `spell_cast` filter ran **once** at`314a8f1`: **274/275**, all14 batch9 functions pass. Sole failure was obsolete355913 generic no-action fixture; moved into the existing named-heal proof and remaining unbound matrix passes1/1 at`521455e`. Separate shield-depletion RED at`43701c9` caught removal of the healing aura; named lifetime fix`e78fad0` passes absorb5/5 and Tranquility2/2. Final format/check pass. No new275/275 full-filter claim; parent owns integration acceptance. No live/full-crate/rotation certification.

Durable frozen sources/census/source receipts/logs: `game-engine/data/diagnostics/spell-dummy9-2026-10-10/`; delivery/proof ledger: `/home/osso/.worktrees/handoff-spell-dummy9.md`. `SPELL_BATCH9_MODE=before python3 <evidence>/regenerate.py --write` reproduces baseline; default regenerates after.

### Eighth batch: projectile, taunt and low-coverage paths

[Contract](../../specs/spell-dummy-scripts.md#eighth-batch-locally-derived-paths), [implementation](../../../crates/server/src/spell_cast/scripts/batch8.rs), [cast tests](../../../crates/server/src/spell_cast/dummy8_tests.rs), [incoming-damage test](../../../crates/server/src/spell_cast/dummy8_defense_tests.rs). Mainline12.1.0.69933 local CSVs and read-only imported rows provide all IDs/amounts; TDB1210 routes named scripts.22 IDs/130 CSV rows match both the slot and canonical databases exactly. No new external fetch, data import or realm process.

| Root | Implemented and observed | Uncertified / missing |
|---|---|---|
|1226019 Reap|Named1225827 damage follows87/134 caster-to-destination line: catalog30yd radius/Width20, multiple enemies, destination direction rather than caster facing; behind/far/ally/other-map exclusions.|Soul collection, optional Fury, aura4/312; unit-center geometry lacks combat-reach edge expansion.|
|115546 Provoke|116189 actual114 attack-me equalizes highest threat and changes victim; active aura11 holds victim6000ms and multiplies this Monk's generated threat by9. Owned61146 statue selects118635's10yd enemy area; foreign statue/ordinary ally fail before costs.|Optional328670 speed; overlapping/reapplied taunt precedence; no player forced-target claim.|
|264178 Demonbolt|AfterCast280127 grants20 raw Soul Shards once, retaining damage.|Zero Dummy1/460553 marker; not relabelled no-ops.|
|187874 Crash Lightning|Catalog104 cone120 degrees/radius8;187878 enhancement lasts12sec.|Root Dummy1, proc318, cleave consumption/195592 split damage. Current Mainline tooltip lacks old TDB two-target gate.|
|115181 Breath of Fire|Catalog104 cone90 degrees/radius12; only owned Keg targets receive123725 DoT and deal5% less damage to that Monk, not foreign players; expiry restores normal damage.|Secondary falloff; no generic269 semantic change.123725s2 is consumed by a spell-specific incoming-damage script.|
|51723 Fan of Knives|One combo point for a successful cast's first damage hit; none for empty area, never per enemy.|Poisons, damage soft cap5.|
|34861 Sanctify|Five injured allies in requested10yd ground area; healthy/sixth/far units excluded.|Dummy3; Prayer recharge reduction retained.|
|29722 Incinerate|244670 supplies two base fragments once for root damage, not each splash unit.|Critical/talent additions and other markers.|
|30108 Unstable Affliction|233490 named base periodic aura applies/ticks/expires.|Five-instance accumulation, vulnerability, dispel/death rewards.|
|44425 Arcane Barrage|AfterCast consumes all Arcane Charges.|Charge-scaled damage, secondary40% hits, optional mana refund.|

Selectors104/134 use authored destination direction, radius, ConeDegrees/Width, map/life/faction selection. Missing required restriction metadata fails explicitly before costs; no guessed universal cone angle/width. Requested destination also now reaches ordinary31 ground-area selection, rather than accidentally centering Sanctify on its self-cast explicit target. Older batch6/7 plans pass `spell.cast_destination` to master's five-argument `effect_dest` API.

Targeted proof:13/13 at`70c4f20`, plus real incoming Breath damage/owner exclusion/expiry1/1 at`2d883e8`, after behavioral RED12/13 and a separate missing-reduction RED. After merging current origin/master`afecfa6`, the requested broad `cargo test --locked -p server --bin game-server spell_cast` ran once at`2fb37d8`: **261/261 pass**, including all14 batch8 functions and the incomplete-metadata104 refusal. Native format/check exit0, warning-free. No live/full-project/rotation certification.

Census compares frozen post-master-integration baseline`52d359f` with`2fb37d8`: **172→171** unresolved Dummy core roots (Provoke closes); reviewed/partial/deferred70→73, un-derived102→98. E3 admissible IDs/memberships124/132→123/131 (additional D1→0), a4 63/71→64/72 (D1 unchanged), a22619/20 unchanged (D0). Master toy-target support accounts for baseline admission differences versus batch7's historical117/125, not new dummy8 scripts. Crash exposes latent187878 Dummy markers. Scope stays13 classes/40 specs,630 book IDs/1551 memberships,302 core IDs/355 memberships. No skipped-child concealment.

Durable receipts, frozen code, census generator, ranked remaining roots and logs: `game-engine/data/diagnostics/spell-dummy8-2026-10-10/`. Delivery/proof ledger: `/home/osso/.worktrees/handoff-spell-dummy8.md`; `SPELL_BATCH8_MODE=before python3 <evidence>/regenerate.py --write` reproduces baseline; default mode regenerates after. Parent retains independent acceptance responsibility.

### Seventh batch: TDB-routed Retail derivations

Implementation [batch7.rs](../../../crates/server/src/spell_cast/scripts/batch7.rs), Survival lifecycle in [aura scripts](../../../crates/server/src/auras/scripts.rs); [bounded contract](../../specs/spell-dummy-scripts.md#seventh-batch-locally-derived-paths). TDB1210.26091 registrations route investigation, not behavior inference from names alone. Across the172 ranked roots,88 have exact script bindings, one has a proc override and four have linked rows. No spell_bonus_data table exists: local Mainline12.1.0.69933 SpellEffect SP/AP/scaling/resource coefficients provide numerical equivalents. Pinned TC a352b1fa hooks plus local descriptions/effect chains derive the selected paths;217 CSV rows across38 IDs match read-only SQLite with no semantic drift.

| Root | Runtime hook / source | Retained boundary |
|---|---|---|
|167105 Colossus Smash|After damage0, apply only208086 aura271; ordinary caster-owned damage multiplier and expiry execute it.|Child343/self marker, soft cap and optional talents.|
|61336 Survival Instincts|Aura apply/remove owns50322 effect0, excluding its unknown Dummy2.|Parent marker/child Dummy2; no full Druid-form parity.|
|206930 Heart Strike|Dummy0 on each native chain target,221536 gate,210738 energize.|Rank316575, extra mirrors and native chain front-arc limit.|
|194509 Radiance|Spell-owned native heal plan, s3 additional cap; stable owned-Atonement priority within injured allies.|Raid/pet priority; Atonement application/duration follows in [batch9](#ninth-batch-local-mainline-ground-fields-and-owned-marks).|
|85288 Raging Blow|Root s1 roll with383854,280270 application, restore one latest charge and notify owner.|Marker may be consumed by root proc; other reset talents.|
|190411 Whirlwind|Caster Dummy1 before weapon children; select count from Dummy0 native area, generate modified280715 and apply85739 maximum stacks.|Cleave copies/consumption and damage soft cap.|
|360995 Verdant Embrace|AfterCast Panacea387761 gates387763 self heal.|Flight, primary heal and root Dummy0.|
|1822 Rake|Initial damage and stun hooks require stealth plus231052 or405834, reading s4.|Child periodic snapshot remains open; s4 stays counted.|

Native proof:21 batch7 functions pass at Rust`81c2bf6`, including100 Survival variance samples, two real Raging casts preserving oldest recharge, both Rake rank IDs, Whirlwind four-stack catalog cap, ownership/rank/radius/expiry exclusions. Format/check exit0, warning-free. The requested full spell_cast filter ran **once**,244/245 at`df14af6`: Survival's fixture omitted Bear5487 effect12's existing3% Fire reduction, creating random overlapping-bound failures. Instrumented RED reproduced it; corrected expected97%/48.5% damage multipliers pass in the100-sample fixture and21/21 batch. Production scripts did not change after that broad run; the other244 passing functions remain valid. No repeated full filter or fabricated successful245/245 claim. Parent owns integration acceptance; no realm/full-project/rotation certification.

Census baseline172 reproduced; after172, reviewed/partial/deferred63→70, un-derived109→102. E3 stays117 IDs/125 memberships: Raging's root E3 closes while Survival exposes its child's unresolved Dummy2. Latent child markers increase a4 from57/65 to60/68 (Colossus, Raging, Whirlwind); a226 stays14/15. Gate memberships8/4/5 and scope13 classes/40 specs,630 book IDs/1551 memberships,302 core IDs/355 memberships unchanged. No no-op registrations or skipped-child concealment.

Reap1226019→1225827 and Provoke115546→116189 were derived but deferred in batch7; their projectile geometry and actual taunt/owned statue paths are now implemented in [batch8](#eighth-batch-projectile-taunt-and-low-coverage-paths). Durable source/archive extracts, schema-aware172-root mining, before/after census and proof ledger: `game-engine/data/diagnostics/spell-dummy7-2026-10-10/`; `/home/osso/.worktrees/handoff-spell-dummy7.md` records publication/release status.

### Sixth batch: local Retail rows

Implementation [batch6.rs](../../../crates/server/src/spell_cast/scripts/batch6.rs); [bounded contract](../../specs/spell-dummy-scripts.md#sixth-batch-locally-derived-paths). Eight ranked paths plus supplementary Ice Lance, emphasizing Balance/Resto, Ret, Brewmaster, Survival, Shadow and Frost. Local Mainline12.1.0.69933 DB2 descriptions/effects are checked against read-only SQLite; description newline normalization is not semantic drift. No external sources, invented IDs or world.db imports.

| Root | Authored action | Remaining boundary |
|---|---|---|
|93402 Sunfire|164815s1/o2/d base bridge;231050 explicitly spreads only the DoT through its native child area.|Root Dummy1 marker and optional child271/537 effects remain open.|
|85256 Templar's Verdict|224266s1 damage bridge; apply only child effect0.|Unknown parent/child Dummy markers remain open.|
|30455 Ice Lance (supplement)|228598s1 damage bridge.|Freezing/shatter unimplemented; not a Dummy census closure.|
|194153 Starfire|s3=70% secondary damage; native area/destination/resource retained.|429523 variant, soft-cap8 and effect3/77 semantics remain open.|
|121253 Keg Smash|s8=50% primary bonus; native secondary damage/slow retained.|Root markers, Shuffle/Brew reduction and damage soft cap remain open.|
|212436 Butchery|294029 grants s2=3000ms per successful damage-effect target (including full absorption), at most s3=5; existing259495 recharge only.|Damage soft-cap formula/talent bleed not certified; recharge proof is not Wildfire Bomb damage proof.|
|48438 Wild Growth|s2=5 injured allies inside native target-centered area; existing periodic aura execution.|Declining tick curve/Tree of Life target additions unimplemented.|
|32379 Shadow Word: Death|s6=5% caster-max-health backlash after target survives, ordinary damage/absorb/death/log path.|Other damage-effect/execute variants not certified.|
|191034 Starfall|327541s1=3sec owned Moonfire/Sunfire extension, s2=28sec remaining cap; do not reset ticks.|Base overlap/tick-child contracts remain open.|

Source receipts, RED/GREEN and final proof ledger: `game-engine/data/diagnostics/spell-dummy6-2026-10-10/`; delivery state: `/home/osso/.worktrees/handoff-spell-dummy6.md`. Initial14 cast functions reproduced11 missing-path failures and3 negative gates. At Rust revision`7cff842`, requested full `cargo test --locked -p server --bin game-server spell_cast` ran once: **224/224 pass**, including17 batch6 functions. Format/check exit0, warning-free. Fully absorbed Butchery regression uses authentic17 (200SP): corrected RED proves missing recharge; damage-effect hit ledger now records zero-loss landed hits without creating damage procs. Wild Growth's initial GREEN tick assertion was corrected to wait through catalog cast time plus first1000ms period. Parent owns independent integration proof; no live realm/full-project/rotation-parity claim.

Census frozen baseline **173** reproduced exactly; after **172**, only Wild Growth's Dummy target-cap field closes a full type gap (not its declining-heal semantics). E3 admissible IDs/memberships118/126→117/125; a4=57/65, a226=14/15 unchanged, additional gated memberships8/4/5 unchanged. Remaining172:63 reviewed/partial/deferred,109 un-derived; scope13 classes/40 specs,630 book IDs/1551 memberships,302 core IDs/355 memberships unchanged. Butchery's s3 remains unresolved for its damage-soft-cap action despite cooldown-cap consumption; no false whole-root closure. Sunfire/Verdict child markers remain latent even when base scripts exclude them.

Batch6 left selector104 unsupported. [Batch8](#eighth-batch-projectile-taunt-and-low-coverage-paths) implements authored caster-to-destination cones; missing ConeDegrees/Width metadata still fails explicitly, preserving the old incomplete-metadata104 fixture. Augmentation candidates need healer-role/selection, stat-distribution or empower/flight contracts beyond these bounded derivations.

### Fifth batch: local Retail rows

Implementation: `spell_cast/scripts/batch5.rs`; [contract](../../specs/spell-dummy-scripts.md#fifth-batch-locally-derived-paths). Sources exclusively local Mainline12.1.0.69933 DB2, cross-checked against read-only SQLite. No fabricated IDs or external source fetches. Eight ranked paths plus supplementary Lava Burst favor DH, Evoker, Warlock, Rogue and DK over previously frequent Mage/Monk additions; Shaman receives the Lava Burst path.

| Root | Authored action / native proof | Retained limits |
|---|---|---|
|162243 Demon's Bite|Applied258876's s3/s4 grant5–10 additional Fury; two casts prove rank/base25.|Root Dummy0 unidentified; full talent damage parity not certified.|
|162794 Chaos Strike|Tooltip197125h=20% rolls once on root hit, refunds193840s1=20 Fury. Chance100/0 catalog clones prove two weapon hits and one/no refund.|Root30 value is not separately consumed; ordinary probability frequency and talent variants not certified.|
|356995 Disintegrate|Own370898s1/d applies50% slow at channel application, not each tick; one cast proves damage ticks and3sec expiry.|Caster-side zero-valued aura4 remains unidentified.|
|116858 Chaos Bolt|Own description guarantees crit and crit-percentage scaling; zero/30% stat casts prove858/1116 damage within catalog variance.|Root Dummy1 Base20 remains unresolved.|
|324536 Malefic Rapture|Own324540s1 reference; count owned periodic damage/leech/percent effects, not heals/foreign auras. One cast proves one/two effects, ally/foreign/heal exclusions.|Base root E3 action closed, not full talent variants.|
|197835 Shuriken Storm|Own s2=one combo point per target hit. One cast proves two enemies→two points and ally/far exclusion.|Dummy2 stealth50 and Dummy3 soft-cap8 remain unimplemented.|
|195292 Death's Caress|Own s3=two Bone Shield195181 stacks. Two casts prove initial2 and9→10 cap while retaining damage/Blood Plague.|Child Bone Shield marker/consumption/haste/armor and Blood Plague leech not certified.|
|207317 Epidemic|Own212739s1/s2 gives infected-primary/other-nearby96/39 damage (AP100), only owned Virulent Plague. Own77575s3 extends owned Dread/Virulent by1000ms.|One cast proves ownership/radius/ally and Virulent expiry; separate Dread tick-clock assertion absent. Latent plague variants unimplemented.|
|51505 Lava Burst (supplement)|Own Flame Shock condition guarantees child285452 crit and own description scales with crit stats. Two casts prove condition/no condition and preserved8/0 Maelstrom.|343725 already adds8 to parent effect1 through generic PointsIndex1; no extra Energize script. No full talent/overload parity claim.|

At Rust revision`f6b842a`, the requested full `cargo test --locked -p server --bin game-server spell_cast` ran once:207/207 pass, including14 new functions and the updated deterministic gross-cost Chaos Strike fixture. A test-only coefficient-bound correction at`e89d353` avoids rare rounded-boundary flakes;14/14 batch5 functions re-proved there. Other193 filtered functions and production cargo-check proof remain valid; no redundant full-filter rerun. Format renewed exit0. These are native process/App proofs, not live realm or independent integration proof; parent owns that gate.

Census baseline reproduces176 exactly; after173. Script-owned E3 actions close162794/324536/207317, not every numeric field or optional talent. E3 admissible IDs/memberships121/129→118/126; a4=57/65, a226=14/15 unchanged. Remaining173:56 reviewed/partial/deferred,117 un-derived; class/spec scope unchanged13/40,630 book IDs/1551 memberships,302 core IDs/355 memberships. Source parser now recognizes qualified `batchN::NAME` registrations; the unchanged baseline still reproduces176. No no-op registrations.

Crash Lightning187874 was attempted then deferred in batch5; [batch8](#eighth-batch-projectile-taunt-and-low-coverage-paths) now supplies native104 cone and initial enhancement, with weapon-cleave/proc damage still uncertified. Durable before/after snapshots, ranked reasons, raw34-ID receipts, census generator and proof logs: `game-engine/data/diagnostics/spell-dummy5-2026-10-10/`. Delivery/remote/release receipts: `/home/osso/.worktrees/handoff-spell-dummy5.md`.

### Third batch: local Retail rows

Implementation: `spell_cast/scripts/batch3.rs`; [contract](../../specs/spell-dummy-scripts.md#third-batch-locally-derived-paths). Sources are read-only local Retail12.1.0.69933 `spell`, `spell_effect`, duration/radius/category/aura-options/PPM rows, not older expansion assumptions. Durable receipts/proofs: `game-engine/data/diagnostics/spell-dummy3-2026-10-10/`; pending proof status is tracked in `/home/osso/.worktrees/handoff-spell-dummy3.md`.

- 596 tooltip "your target and ${$s3-1} injured allies" supplies five total targets; effect1's radius23 supplies40yd. Spell-owned plan retains98367 marker trigger, caps additional injured allies, uses existing nearest-first area order. No claim that this order is a Mainline injury-priority formula.
- 703 conditional `a196861` names1330;196861 names256148. Regular Stealth1784 aura at OnHit gates these five-second children before normal proc interrupts run.256148 applies only after the caster's1330 actually lands:196861 says "Enemies silenced by Garrote"; a real Unending Resolve104773 effect3 silence-immunity cast fixture reproduces and prevents the incorrect debuff. Zero-valued703 aura4 stays unresolved; Improved Garrote window not inferred.
- 2050 `$s2`=6 seconds reduces its running category1673 recharge after2061 completes; existing timer mutation sends charge updates. Optional Prayer1246517 is not implemented.
- 2948 `$s2`=30 is a strict health threshold. Crit hook now receives the target before damage, keeping normal crit/mod calculation above the threshold and existing Fire Blast guarantee.
- 5221 `a231063`/`$231063s2` supplies25% bonus against bleeding targets. Applied aura spell/effect mechanic15 identifies a bleed, not merely any DoT. Other talent/stealth variants remain unsupported.
- 6343 names435203s1/d; damage-effect hit casts the20% ten-second snare per enemy.20243's chance is sourced from231834s1=30%, not its unused mirrored Dummy. Full timer reset sends the ordinary cooldown update; absent rank leaves it untouched.
- 8921 names164812s1/o2/d; base bridge launches only child effects0/1 (SP0.212 direct, SP0.184 per2000ms). It does not accidentally apply optional effect7's20% snare. Other Moonfire talent/resource fields remain unsupported.

Batch3 census (verified2026-10-10, code `c7a69c7`): Dummy-bearing core roots **180→177**; complete root-type closures2050/2948/8921. E3 admissible IDs/memberships **125/133→122/130**; a4 **57/65 unchanged**; a226 **14/15 unchanged**. D memberships remain8/4/5. Scope remains13 classes/40 specs,630 book IDs/1551 memberships,302 core probes/355 memberships. Five newly scripted roots retain unresolved fields; retained177 list has45 reviewed/partial/deferred and132 un-derived entries. Moonfire's child closure also exposes latent aura269; this is not hidden by the base-only effect mask.

Native proof:16 new cast tests. `dummy-integration.log` at `d80ba12` passes51; only its three Garrote tests were invalidated by the later silence-immunity fix, renewed and expanded to four passing Garrote tests at `c7a69c7`. Thus52 unique Dummy tests have surviving proof. Separate29 generic cast regressions survive the Garrote-only change; final format/check exit0, warning-free. Readability manually inspected; independent artifact verifier failed OAuth refresh, so no independent or PR-ready claim. No live realm/client proof. SQL opens mode=ro;21 actual IDs' descriptions and effects match raw local DB2 CSVs (`raw-db2-receipts.json`). No missing imports required for these bounded paths.


### Fourth-batch local Retail rows

`spell_cast/scripts/batch4.rs` adds ten bounded paths: eight ranked Dummy-root candidates and supplementary Vivify/Eruption behavior. Sources exclusively local Retail12.1.0.69933 descriptions/effects; no older-expansion scripts imported. [Contract and exclusions](../../specs/spell-dummy-scripts.md#fourth-batch-locally-derived-paths).

- 85222s2 caps healing at five injured allies after its normal radius selection. Current Mainline description says "radius around you", not an older cone; nearest-first ordering is retained, not invented injury ranking. Additional Dummy2 remains unresolved.
- 56641 references77443s1=20 Focus.53351s2 supplies the strict20% admission threshold before cost/cooldown. Steady cooldown/level normalization and Kill Shot talent overrides remain unimplemented.
- 33917/231064 gates its20% bonus on an applied bleed mechanic15.115151 names119611o1/d; only child effect0 is applied. Overheal travel and231606 tick procs remain unimplemented.
- 116670/274586 names425804, whose own132/31 area selection is narrowed to caster-owned119611 marks. Triggering one unrestricted child per mark incorrectly healed unmarked allies; one narrowed child plan fixes that boundary. Foreign/far/unmarked allies receive no splash.
- 188443 with343725 reads passive effect4=two Maelstrom per damage-hit target; rank absent produces none.49184 applies55095's base periodic effect to hit enemies only; CAP/secondary damage and Frost Fever resource procs remain separate gaps.
- 34861s3 supplies six seconds removed after596 completes; only running timers change.395160 gates Ebon Might extension on395153s1=1000ms, once after cast; existing finite owned395152/395296 expiries extend without creating buffs or changing tick clocks.

Batch4 census (verified2026-10-10, code`83891f4`): Dummy-bearing core roots **177→176**. Only Kill Shot53351 closes its entire Dummy-type gap; the other seven ranked paths retain fields. E3 admissible IDs/memberships **122/130→121/129**; a4 **57/65** and a226 **14/15** unchanged, with D memberships8/4/5 unchanged. Remaining176 roots:50 reviewed/partial/deferred,126 un-derived. Vivify/Eruption are supplementary non-Dummy-root paths, not census closures. New Howling Blast child closure exposes latent aura380/381 requirements; base mask does not conceal these.

Native proof:17 new functions.69 Dummy cast tests passed at`b5ca03e`; later ally-selection extraction invalidated only four Dawn/Vivify tests, renewed2+2 at`83891f4`. Final fmt/check exit0, warning-free.24 real IDs' descriptions/effects match local raw client DB2 CSVs. Native cast proofs, source snapshots, regeneration and census receipts: `game-engine/data/diagnostics/spell-dummy4-2026-10-10/`. Readability manually inspected; no independent/full-suite/live-realm proof claimed. Parent owns integration gate.

See [contract](../../specs/spell-dummy-scripts.md). Effect-hit scripts bridge Shield Block2565 to132404, Bloodthirst23881 to117313 and a chance-based184362, and Rampage184367 to184362. Damage/resource effects continue through normal dispatch. Bloodthirst reads modified effect2 base points, not ProcChance101 or PPM; local DB2 has no PPM entry for these casts.

Sources: TrinityCore `a352b1fa` `src/server/scripts/Spells/spell_warrior.cpp` (`spell_warr_shield_block`, `spell_warr_bloodthirst`, `spell_warr_bloodthirst_enrage`); Retail12.1.0.69933 `spell`/`spell_effect` rows2565,132404,23881,117313,184367,184362 in read-only `data/world.db`. Rampage's tooltip states "Enrages you"; its existing effect triggers name the attack spells. Fresh Meat and other optional talent variants remain unproved.

Holy Shock20473 branches on ally/enemy to25914/25912; parent ENERGIZE retains1 Holy Power. Sources: TC `a352b1fa` `spell_paladin.cpp:1261-1311`, `spell_pal_holy_shock`; Retail tooltip `$25912s1`/`$25914s1`, local SP coefficients2.16/11.17, no PPM row. Conditional talent272906 crit bonus is not covered.

Barkskin22812 PERIODIC_DUMMY effect2 casts213709 immediately (Attributes5 EXTRA_INITIAL_PERIOD), then on each1000ms tick while Brambles203953 is present; `auras/scripts.rs` registers this callback. TC `a352b1fa` `spell_druid.cpp:297-316` names the condition and child; Retail203953 tooltip supplies `$213709s1`/`$22812t3`, child AP coefficient0.075. No PPM roll: this is a timed conditional pulse. Brambles absorb/reflection semantics are outside this script.

Penance47540 DUMMY0 starts47757 healing or47758 damage channel with the shared normal channel-state constructor, then resolves the child without a second cost/GCD. The channel target is present before periodic aura application; target77 bolts select it. Source: TC `a352b1fa` `spell_priest.cpp:3070-3125,5546`; local child aura23 triggers47666/47750 at1000ms with extra-initial-period and2000ms duration (three bolts), SP0.932/4.011. No PPM/chance roll. Target marker aura4, Atonement and Power of the Dark Side talent semantics remain unproved.

Cobra Shot193455 reads effect2 (`$s3` =1 second) and reduces Kill Command34026 recharge after its damage hit. Retail12.1.0.69933 tooltip: "Reduces the cooldown of Kill Command by $s3 sec." Kill Command has ChargeCategory2067, so reducing only RecoveryTime500ms is incorrect: the script updates the running charge deadline and sends SpellChargesUpdate. No aura-options/PPM row. Tests use a real Kill Command cast before Cobra Shot. Dummy0 remains unresolved metadata; this does not certify all fields.

Blackout Kick100784 rank-two261917 reads effect2 (1000ms) after damage and reduces both Rising Sun Kick107428 charge recharge (category1518) and Fists of Fury113656 spell cooldown. Source: local Retail261917 description `$100784m3/1000`; parent100784 conditional `s261917`. No PPM row; ProcChance101 is metadata, not a roll. Other Dummy fields/Teachings variants remain unresolved.

Backstab53 damage-amount hook consumes Dummy3 ($s4=20%) only when the caster is in the target rear half-plane. The normal damage/crit/armor/log path stays shared. Source: pinned TC a352b1fa spell_rogue.cpp:225-251 spell_rog_backstab; local Retail53 AP1.0731 and effect3 BasePoints20. No aura-options/PPM. Dummy0 and optional Find Weakness remain unresolved.

Pyre357211 missile impact393568 triggers357212 at the hit position (TC a352b1fa spell_evoker.cpp:601-618 spell_evo_pyre). The spell script selects enemies using child357212 radius and cap and resolves through the existing hit/missile/effect pipeline. Script-owned destination87 is not added to generic target dispatch; no unit-target reinterpretation or caster-centered explosion. Retail tooltip `$357212s1`/`$357212A1`, SP4.4; no PPM. Optional Volatility369089 re-explosion remains unsupported.

Drain Soul198590 dynamically multiplies each damage tick by effect1 (100%) only below effect2 (20%) health; TC a352b1fa spell_warlock.cpp:823-850, local tooltip `$s2`/`$s3` plus `$o1`/`$d`/`$t1`, SP1.11375/1000ms/5000ms. Aura removal hooks now receive RemoveReason (including replacement), so only Death triggers205292/10 raw Soul Shards. Channel expiry/StopSpellCast produce no shard. No aura-options/PPM entries.

Fire Blast108853 crit override (TC a352b1fa spell_mage.cpp:804-815 spell_mage_fire_blast) returns100% independently of caster crit stats. Retail tooltip `${$s1*2}`/SP0.98625; no proc or PPM row. This is a core rotational behavior absent from the Dummy-only census, not a closed Dummy gap. Cast-while-casting and Hot Streak remain uncertified.

Death Coil47541 branches47632 damage/47633 healing (Retail tooltip `$47632s1`/`$47633s1`, AP2.11812/4.6375). TC a352b1fa spell_dk.cpp:589-609 only supplies the damage bridge; Retail tooltip defines the additional Undead healing and `$77575s3/1000` plague extension. Lichborne49039 says "become Undead"; friendly humanoids are rejected before costs, while NPCs use authoritative template type6. Existing caster-owned Dread1240996/Virulent191587 finite expirations extend1000ms without a periodic-timer reset; other casters’ auras are not extended. ProcChance101/PPM0 are not a random bridge. Optional Unholy Vigor not implemented.

Chain Heal1064 has a spell-specific target plan, not generic target45 admission. The explicit ally receives the first heal; up to three subsequent targets use greatest absolute missing HP within20yd of the current jump source, then30% falloff per jump. Sources: Retail1064 `$s2`=30/$s3=20, effect0 ChainTargets4/SP2.31/ChainAmplitude0.7; TC a352b1fa Spell.cpp:2216-2322 SearchChainTargets establishes missing-HP priority and advancing source. Old TC default12.5yd is superseded by the local tooltip20yd. Additional-jump LOS and optional talent236502 range increase remain uncertified. No proc/PPM roll.

Demon Spikes203720 applies203819 (TC a352b1fa spell_dh.cpp:1035-1055); effect0 is excluded without Deflecting Spikes321028 (same file:891-910). Retail tooltip `$203819s2*$AGI/100` =75% Agility armor, `$203819d`=12000ms, optional8-point parry modifier. Its child needed real aura268 support, not an inert aura marker: SpellAuraEffects.cpp:3951-3963 and StatSystem.cpp:255-276 use current primary stats. StatModifiers now carries per-stat armor percentages and MiscValue-2 primary-stat percentage; CharacterStats recomputes their armor contribution on stat changes. Calcified Spikes/Retaliation remain unsupported. No PPM roll.

Cooldown reductions send the original catalog/modded full cooldown duration alongside the reduced remaining time; otherwise the native client resets its sweep to100%. Fists of Fury cast -> completed channel -> Blackout Kick rank two test asserts both fields (source: native player_spells.rs CooldownTimer contract). Pyre plan selection and timer mutation are named helpers; behavior unchanged.

Cobra Shot193455 unconditional tooltip `$<mult>` resolves through SpellXDescriptionVariables to variable331: `(min(player_level-1,19)*12+112)/340`. The damage hook applies this real early-level curve (unity from20 onward), in addition to the cooldown script. Level10/19/20/80 casts exercise it; raw AP coefficients alone overstated low-level damage. No guessed curve or server DB edits.

Backstab53 unconditional `$<mult>` joins variable339: `(max(0,min(player_level-10,10))*11+260)/370`. Apply this level curve before the position bonus; level10/19/20/80 front/rear casts prove it. Variable615 confirms Chain Heal jumps =ChainTargets-1; variables385/84 contain optional or legacy Monk/DK branches and do not replace current explicit tooltip fields. Exact joins are retained in description-variables.json in batch2 evidence.

## Druid automatic forms

`druid_transform.rs` binds exactly five native BeforeCast scripts: Prowl/Tiger Dash→Cat768; Incapacitating Roar/Berserk/Stampeding Roar→Bear5487. It uses real triggered child effects, only when not already in that form, before completion/costs and after initial admission. Berserk alone runs its native AfterCast Mangle/Thrash/Growl spell/category reset, not charge restoration or GCD reset. Category-only cooldowns keep the native spell entry until category recovery (`SpellHistory.cpp:505`). Census emits observed `form_after` separately from form-prerequisite/warning receipts. [Source and behavior proof](../../class-completeness6.md#native-druid-beforecast-transformers--batch52), [contract](../../specs/class-completeness.md). Other form/aura/script gaps remain explicit.

## Feral Spirit owner contribution — 2026-10-15

[Contract](../../specs/spell-guardians.md#feral-spirit-owner-inheritance). Actual51533→228562 creates two29264 wolves. Pinned TC a352b1fa TemporarySummon.h:119 classifies only29264 as Spirit Wolf; StatSystem.cpp:1353/1423 supplies owner melee AP×0.31 (integer AP and bonus SP) and AP/3.5×attack seconds. Pet.cpp:1026 supplies weapon3–5×summon level; local template supplies1500ms. Both spell power and melee swings read live owner power. Other guardian formulas remain unchanged.

Named gap `FERAL_SPIRIT_INTRINSIC_AP_SOURCE`: archive pet_levelstats has2715 rows and zero29264. TC substitutes explicitly fake Strength22; this implementation does not. Intrinsic AP, armor/stamina shares, healing/AI/chase and full legal-loadout parity remain unsupported. Native centered training-dummy RED at89645435 reaches ownerAP100→10000 with damage10→10 and power(0,0); Merged afc0e5e5/protocol62865c23 controls24/24 pass (55.15s tests), check102.286s/fmt/readability clean. Live power asserts31→3100 SP/AP; ordinary dummy stays0/0. Independent/full-stat/legal-loadout acceptance remains open. Sources and proof ledger: data/class-completeness6-2026-10-12/guardian-{owner-power-source,owner-power-proof-ledger,wolf-pet-levelstats-source}.json.
