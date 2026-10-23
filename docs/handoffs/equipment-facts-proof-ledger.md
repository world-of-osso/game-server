# Shared native equipment APL facts

## Contract and sources

Generic across native Retail specs: original equipped IDs/names, trinket is.<name>, has_use_buff, has_buff.{strength,mastery,versatility,haste,crit}, cooldown.{duration,remains,ready}, has_cooldown, proc.any_dps.duration and set_bonus.<tier>_<n>pc. Unknown operands/names error. Preserve ItemUse/Prepared and trinket1→trinket2→main-hand preference. No item-name mechanics map or guessed timers/coefficients. Stat-allocation redirects never replace effect/membership identity.

Native ItemEffect/bonus rows and SpellStore supply metadata. TC a352b1fa SpellHistory.cpp1119–1144 uses spell recovery/category only when BOTH item recovery fields are negative; zero/mixed signs are authoritative. ON_EQUIP ProcCategoryRecovery is distinct. Reachable aura durations are native, never invented when absent. Active-stack/completed-node traversal distinguishes cycles from shared-child DAGs; DUMMY3 TriggerSpell is script input, not a generic cast edge (native AraKara452146→443541 backreference).

`SIMC_TIER_ALIAS_ITEMSET_BINDING`:151 distinct aliases/ItemSet pairs from all1183 pinned76f8118 `item_set_bonus.inc` rows, source SHA3ed02060fb5ddcda37eac3472cc24a9bb1432bc97866792dbfd1b052aaae10e2. Native set eligibility is reused from [item-set lifecycle](native-item-set-proof-ledger.md), not duplicated or inferred from item names. Generic item-name normalization follows pinned util.cpp3234; names load once with the existing native item catalog.

## Proof ledger

- At80c23d207, initial Equipment3genuineRED plus native set2RED39.20s; metadata remained unchanged then. `equipment-set-red-v2-full.log`.
- At a8bd77d3b, Equipment5genuineRED (including exact Ele names/native tier operands), while native set/equipment/commerce7PASS53.88s. `item-set-green-equipment-red-v2-full.log`.
- After lp1033/master6b2f73a27 merge,61473b22e native set/equipment/commerce7PASS + Equipment7genuineRED74.81s,9m33compile, zero Rust warnings. Added missing-child rejection and shared-child DAG witnesses, native field operands still unsupported. `equipment-red-v3-full.log`/`.proof.json` record the exact historical source-mechanics DB, not a current DPS measurement.
- Native source receipt `equipment-native-source-v1.json` includes original ItemEffects/spells/durations for Badge61034, Puzzle193701, Mirror207581, Bloodthirsty139329, AraKara219314, Guillotine270173 and Thunderfury19019. Tests do not fabricate spell/item IDs, timings or numeric combat grants. Malformed-graph tests alter only test-owned graph edges.
- Generic implementation follows these REDs; targeted GREEN pending. No proc-execution or DPS claim.

## Explicit gaps

Metadata reachability is not proc execution. Funhouse1213432 is native DUMMY3/Trigger0; Stormbound1295275 has PeriodicTrigger23/Trigger0. Their script-created stat child is not encoded in those native edges; no child ID or stat grant is guessed. Ret/Ele item child mechanics remain unqualified. Native on-use execution's older item override path is a separate boundary from these metadata facts.

Parent owns final integration. Any DPS measurement must use a new SQLite backup of CURRENT canonical world.db with source SHA, not these historical reproducibility inputs.
