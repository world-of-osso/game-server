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

Post-master8e7c7a685 library-boundary proof: [combined44-case qualification](ret-paladin-parity-proof-ledger.md#combined-integration-qualification), including all nine Equipment cases and the unchanged WW consumer.

## Native instance ilvl and proc.default_value — 2026-10-24

Replacement set/Warrior packetf36aa1a72 is separate. Native fact RED at278c7feec: three genuine assertion failures12.61s after7m12s compile, no warnings. Exact Dev1467/Fire63/Assassination259 profiles load actual identities/modifiers; ilvl is unknown instead of289/334/344, proc default unknown instead of qualified0, and unqualified Puzzle193701 lacks named UNSOURCED error. All cases load before assertions. Frozen exact inputs/hashes and full commands/logs: engine `data/diagnostics/native-spell-diff-2026-10-23/equipment-facts-f36aa1a72/`.

Pinned76f8118 `unique_gear.cpp3970–3989` reads per-slot item_level; use the existing native item_instance_stats producer for passive and on-use slots alike. Assassin270175 has no explicit override: original219 + native bonus13662/13848 resolves344 (reference gear reports344). No stat redirect or fabricated item level.

`unique_gear.cpp3798–3820` delegates to matching buff.default_value, aggregated max-with-zero at3776–3787; `buff.cpp471–473` returns that field, not stat_buff.stats[].amount. `buff.hpp327` initializes unset default to negative DBL_MIN; `stat_buff_t` constructor3397–3480 and add/set_stat helpers do not set it. Exact native drivers in these profiles are sourced against `unique_gear_midnight.cpp`: Gaze1256896(2020), Vaelgor1259293/1260459(2747,6511), Seastone1295058(3533), Vial1293316/1295179(3567,6543), Heart1297760/1297761(3748), Font1297911/1297908(3806,6545). Their custom/ordinary stat buffs leave default unset, or disabled/damage drivers expose no matching stat buff. Vial's create_all_stat_buffs helper (unique_gear_helper.hpp670–707) changes stat amounts only. Those exact native roots bind aggregate0; other roots remain explicitly UNSOURCED rather than guessed zero. This source policy is not a new native aura/value grant or proc qualification.

Atb0ce2e2b5 all12 native Equipment controls PASS36.90s after59.88s compilation: new profile-level/default and named-unknown boundary3 plus retained9. Two-package `cargo check --locked -p server -p server-simulation --tests -j2` PASS2.76s, no warnings; fmt/diff check PASS. Binary SHA`27b99439ee265ec1a6e40cfecbf8d7d6be4f456885cfc5b09407d585bb859c01`; native world SHA`cd7230ac1d62bf7d7ea8fef5fbb82faa9f36b61ec78108e591f1204c614a8ed6`. `source-binary.proof.json` retains native ItemEffect roots, source-file hashes and proof boundary; `red/green/check.proof.json` record exact commands/revisions. Qualified source fact semantics do not imply any adapter routes every operand or that DPS parity is established.

## Explicit gaps

Metadata reachability is not proc execution. Funhouse1213432 is native DUMMY3/Trigger0; Stormbound1295275 has PeriodicTrigger23/Trigger0. Their script-created stat child is not encoded in those native edges; no child ID or stat grant is guessed. Ret/Ele item child mechanics remain unqualified. Native on-use execution's older item override path is a separate boundary from these metadata facts.

Parent owns final integration. Any DPS measurement must use a new SQLite backup of CURRENT canonical world.db with source SHA, not these historical reproducibility inputs.
