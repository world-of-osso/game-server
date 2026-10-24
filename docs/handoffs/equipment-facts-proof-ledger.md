# Shared native equipment APL facts

## Contract and sources

Generic across native Retail specs: original equipped IDs/names, trinket is.<name>, has_use_buff, has_buff.{strength,agility,intellect,mastery,versatility,haste,crit}, cooldown.{duration,remains,ready}, has_cooldown, proc.any_dps.duration and set_bonus.<tier>_<n>pc. Unknown operands/names error. Preserve ItemUse/Prepared and trinket1→trinket2→main-hand preference. No item-name mechanics map or guessed timers/coefficients. Stat-allocation redirects never replace effect/membership identity.

Native ItemEffect/bonus rows and SpellStore supply metadata. TC a352b1fa SpellHistory.cpp1119–1144 uses spell recovery/category only when BOTH item recovery fields are negative; zero/mixed signs are authoritative. ON_EQUIP ProcCategoryRecovery is distinct. Reachable aura durations are native, never invented when absent. Active-stack/completed-node traversal distinguishes cycles from shared-child DAGs; DUMMY3 TriggerSpell is script input, not a generic cast edge (native AraKara452146→443541 backreference).

`SIMC_TIER_ALIAS_ITEMSET_BINDING`:151 distinct aliases/ItemSet pairs from all1183 pinned76f8118 `item_set_bonus.inc` rows, source SHA3ed02060fb5ddcda37eac3472cc24a9bb1432bc97866792dbfd1b052aaae10e2. Native set eligibility is reused from [item-set lifecycle](native-item-set-proof-ledger.md), not duplicated or inferred from item names. Generic item-name normalization follows pinned util.cpp3234; names load once with the existing native item catalog.

## Proof ledger

- At80c23d207, initial Equipment3genuineRED plus native set2RED39.20s; metadata remained unchanged then. `equipment-set-red-v2-full.log`.
- At a8bd77d3b, Equipment5genuineRED (including exact Ele names/native tier operands), while native set/equipment/commerce7PASS53.88s. `item-set-green-equipment-red-v2-full.log`.
- After lp1033/master6b2f73a27 merge,61473b22e native set/equipment/commerce7PASS + Equipment7genuineRED74.81s,9m33compile, zero Rust warnings. Added missing-child rejection and shared-child DAG witnesses, native field operands still unsupported. `equipment-red-v3-full.log`/`.proof.json` record the exact historical source-mechanics DB, not a current DPS measurement.
- Native source receipt `equipment-native-source-v1.json` includes original ItemEffects/spells/durations for Badge61034, Puzzle193701, Mirror207581, Bloodthirsty139329, AraKara219314, Guillotine270173 and Thunderfury19019. Tests do not fabricate spell/item IDs, timings or numeric combat grants. Malformed-graph tests alter only test-owned graph edges.
- Generic implementation33fa641d1 follows these REDs; nine focused controls now GREEN after the native primary-name and SpellMisc duration fixes. No proc-execution or DPS claim. Native ItemEffect zero/mixed-sign source controls additionally cover45/553/15477/15541/31108/170101/4894; zero overrides retain0 despite native spell recovery500/1000ms, while native4894 takes180s instead of its spell's120s. These assert source calculation, not those items' cast execution.

- Primary-stat control atde7b6e406 failed genuinely: Living Flame45148's native64712 intellect aura returned `[0, unknown agility, unknown intellect]` instead of`[0,0,1]`. Combined native batch runtime56.50s; `reconcile-primary-judgment-red-v3-full.log`. Native indexes1/3 now retain agility/intellect names and stat facts accept them; strength61034/84966 and agility61033/84969 controls remain in the unchanged test. Unknown-stat errors retained. All nine Equipment controls PASS atdd0ed8fe5, warning-free compile6m35s/runtime156.72s combined with other slices; `reconciled-native-launch-red-v2.{proof.json,full.log}`. Same production implementation at02090739e consumes real WW269 gear/traits successfully: existing combo/utility consumer1PASS47.55s, `ww-consumer-green-v1.*`; inputs copied into a new diagnostics directory, frozen inputs untouched.

## Explicit gaps

Metadata reachability is not proc execution. Funhouse1213432 is native DUMMY3/Trigger0; Stormbound1295275 has PeriodicTrigger23/Trigger0. Their script-created stat child is not encoded in those native edges; no child ID or stat grant is guessed. Ret/Ele item child mechanics remain unqualified. Native on-use execution's older item override path is a separate boundary from these metadata facts.

Parent owns final integration. Any DPS measurement must use a new SQLite backup of CURRENT canonical world.db with source SHA, not these historical reproducibility inputs.
