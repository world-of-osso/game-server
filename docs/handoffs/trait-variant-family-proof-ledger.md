# Trait-variant family proof ledger

## Active goal

Implement lead-approved `TRAIT_VARIANT_FAMILY_FROM_SIMC` in one follower action resolver. Read the actual active native `TraitLoadout`, not kit baseline membership. Resolve the selected action before eligibility, radius, cost and casting. Preserve ordinary form/aura overrides, native legality and fresh-pack safety. Fire owner13122 owns Hot Streak/Pyroclasm/other Fire consumers; no second resolver. Exclude other action families and new rotations.

## Named policy and sources

Pinned SimC: `76f8118cf3f7f41dad20708e8328e1f4ab2deebc`.

|Family|Selected trait → action|Native node/entry|Source|
|---|---|---|---|
|Lunar Inspiration|155580 →155625, replacing kit8921|82108/103170|`engine/class_modules/sc_druid.cpp:4457–4460` selects155625 only with Lunar Inspiration;9366–9372 separately constructs Convoke children. Native155625 applies its own periodic aura; base8921 script emits164812. The selected variant must maintain155625, not test164812.|
|Flamestrike choice1|2120 →2120, replacing kit1254851|109409/135619|`engine/class_modules/sc_mage.cpp:3891` chooses selected2120 first.|
|Flamestrike choice2|1254851 →1254851|109409/135602|Same constructor uses selected1254851 otherwise.|

Native TraitDefinition140375 (2120),140358 (1254851),108175 (Lunar Inspiration155580) have `OverridesSpellID=0`. Therefore these pairs are explicit named project policy, not a claim native DB2 authored override metadata. Full exported loadouts in the two test fixtures retain native parent/budget validation. The Fire control switches only the alternative entry of native choice node109409 through the ordinary evaluator.

## Proof ledger

- RED tests authored: actual selected Feral Moonfire cast/DoT/damage/maintenance; actual Fire2120 cast/damage/exclusion of sibling; native alternate1254851 choice control.
- RED runtime: first admission controller3252915 stopped before Cargo when the lead expanded producer/classifier scope; no test result produced. Requeue exact-source RED after the next coherent source commit. No production resolver implementation before observed behavioral RED.
- Existing runtime evidence:32b2a678f Fire cast unselected1254851 eight times; selected2120 is absent. Feral selects155580 but hardcoded8921 does not cast in Cat.
- Current-kit73/103 reruns: lead's eg958796e0e7c9/a10a19b8 artifact gives2/2 executionPASS against464fbb46 snapshot; DPS54569.60/49373.95 and18/20positiveadds each. Gate479PASS/1unrelatedpet-achievementFAIL/34ignored, not all-green. No duplicate baseline build; current metrics remain separate from resolver RED/GREEN.

## Next approved scope

After resolver: `WARRIOR_DEMOLISH_SIMC_CHANNEL`, SimC `sc_warrior.cpp:3852–3888`, native436358 channel scheduling440884/440886/440888 and ordinary child damage; add the missing kit action. TC a352b1fa has no indexed named Demolish script. Separate RED-first implementation, not part of the resolver change.
