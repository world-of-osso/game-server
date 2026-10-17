# PvP progression

Retail Mainline behavior applies to both client art skins. Extend existing PvP/rated records and currency wallet; no duplicate economy.

- Use locally sourced Honor1792/Conquest1602. Preserve durable balances/weekly-earned counters when converting obsolete internal1/2 IDs; do not expose those IDs as alternate currencies.
- Pairing queue entries must not create a match outcome, rating change, honor or conquest. A real lifecycle/result and an authored numeric policy are required.
- Lifetime honor XP/levels/rewards are separate from spendable Honor. Persist progression and selected talents/War Mode state in existing PvP storage.
- War Mode changes require authoritative rest-area, combat and instance checks; its source-backed buff and talent activation must follow actual world state.
- PvP talent selection requires source specialization, slot-mask/unlock, level and PlayerCondition eligibility. Unknown conditions cannot be guessed true.

## Source catalogs and import

`scripts/reference/pvp-12.1.0.69933/` contains locally extracted CASC records. CurrencyTypes contains only1792/1602; PvpTalent407 rows, category3, unlock4, PrestigeLevelInfo24. Values come from Retail build keydcfc90fffd79ba00406ae46f5f657592, exported by game-engine `scripts/export_db2_csv.py` at exact layoutsFA3226AB/BE69BB5A/ECB2C5C2/9685CBCC/6582638F. The local TrinityCore `DB2Structure.h` preserved in game-engine `data/diagnostics/scrap-2026-10-10/` defines field semantics. Numeric source receipts remain in `data/diagnostics/pvp-progression-2026-10-11/db2/`.

Lead/operator import (agents do not write canonical world.db):

```sh
python3 scripts/import-pvp-progression.py --db /path/to/explicit/world.db
```

Imports only `pvp_currency`, `pvp_talent`, `pvp_talent_category`, `pvp_talent_slot_unlock`, `pvp_honor_reward` in one transaction, preserving unrelated tables. Requires an existing DB; validates every source first, rejects duplicate IDs and incompatible destination columns. Re-running replaces those owned catalogs without duplicates. Runtime integration is separately required; import alone is not progression completion.

## Lifetime honor XP — 70391 continuation

Concrete pinned-TC honorable world-player kills accumulate `lifetime_honor_xp` in existing durable PvP progress and replicate it separately from wallet1792. Same-team/self/gray victims, inactive recipients, no-PvP-credit victims and battleground/arena kill paths do not generate world-kill XP. Nearby living group recipients divide base credit before maximum-positive honor aura modifiers. No currency conversion is inferred from `RewardHonor`, which calls `AddHonorXP`, not `AddCurrency`.

`HONOR_LEVEL_XP_CURVE` is unresolved: keep `honor_level=None`; earned XP must not advance levels or grant milestone rewards. Current-install1391671 remains the old Prestige0..32 table. Honor rested-pool accrual also depends on that unknown curve and is not implemented by borrowing ordinary rested XP. Completion rewards require independently authored DB2/TDB rows; absent payouts remain gaps.

Pinned raw sources (prefix `https://raw.githubusercontent.com/TrinityCore/TrinityCore/a352b1fa/`):
- `src/server/game/Entities/Player/Player.cpp`, SHA256 `78e2294e84ff0a9b778690a42a9fe77c2a02885b9425d8039974bf881ac5b1e4`:6695–6913. Its universal8800 assignment is explicitly guessed and excluded.
- `src/server/game/Miscellaneous/Formulas.h`, SHA256 `ce1ea35a826bcff3410a7da5882e93c30195d167c11ef79264b0fe67c889e434`:55–83.
- `src/server/game/Entities/Player/KillRewarder.cpp`, SHA256 `a0252c17dbce436709362f9fd9e10552bad90c45239572168bddaaa3aaf9eb76`:89–118,133–137,190–198,220–250.
- `src/server/game/Entities/Player/Player.h`, SHA256 `9908cb848beaa931b878dcc2d47a016e3fff0288ddb686874163b603d5b08199`:1207–1208.
- `src/server/game/Spells/Auras/SpellAuraDefines.h`, SHA256 `1784b495ea9704bbe12223c2ad42d9d5fea7d320f0dc6c0055cd090cb7e5498b`:159/212/491 identities; active70391 SpellName identifies43681 as Inactive.

## Private Currency presentation fixture

`cargo run -p server --example pvp_currency_zero_fixture -- <private-redb> <character-id>` is an approved diagnostic only. It accepts only the owned2026-10-16/private5941 database while stopped, requires an existing saved wallet, refuses earned/unrelated balances and adds only a discovered1792 entry with quantity/week-earned0. It does not award Honor, alter lifetime XP or mutate canonical realm data. The native client must separately prove a real empty wallet hides Currency before using this fixture.

## World of Osso policy: HONOR_WALLET_MIRRORS_HONOR_XP

Explicit Lead authorization binds an implemented TC-sourced Honor award of N to two separate credits: the unchanged lifetime honor-XP path receives N, and the existing wallet receives N of Honor1792 subject to its authored15000 holding cap. The wallet cap must not reduce XP. Persist the wallet and PvP projection, publish the authoritative CurrencyStateUpdate and normal gain feedback, and keep static weekly0 bookkeeping unchanged. This is our event binding, not a claim that TC itself credits1792 in RewardHonor. The mirror is pinned by actual lethal-damage and persistence tests; native acceptance requires a real world-kill wallet increase in both skins.

The rule applies at the shared Honor award boundary so per-map BG Honor awards use the same mirror when their callbacks are implemented. Current world-kill eligibility/calculation stays unchanged. BG callbacks not yet wired remain an implementation gap, not a missing wallet-binding authorization. BG admission must retain the exact [PVPDifficulty source range](../../../shared-protocol/docs/specs/battleground-brackets.md): event bonus amounts use the instance bracket maximum, not a participant's current level or the broad template admission maximum. Arena/Conquest1602 quantities remain ARENA_COMPLETION_WALLET_BINDING; no mapping is authorized. Korrak1166 stays out of scope. HONOR_LEVEL_XP_CURVE remains a gap; no level advancement is added.

## World of Osso policy: BG_END_BONUS_ONCE_PER_MATCH

Explicit Lead authorization normalizes each per-map completion hook to one invocation per completed match. Pinned TC `Battleground.cpp`675/781 places OnEnd inside the online-player loop; team-wide script awards would otherwise multiply by roster size. We do not reproduce that placement. This is a named project normalization, not a Retail claim. Each eligible present participant receives the authored per-map win/end bonuses once, and HONOR_WALLET_MIRRORS_HONOR_XP mirrors the authorized XP1:1 subject to the holding cap.

Current callbacks: Arathi2107 OnEnd574–585 and Eye566 OnEnd437–448 each grant a winning participant one bonus-kill equivalent, then grant every eligible participant one equivalent. Each call rounds separately using `ceil(min(instance PVPDifficulty maximum,80) ×1.55)`:30–39 winners receive61+61=122; other present participants receive61. Inactive/minimum-XP-level checks remain; dead accepted participants receive the team award, departed/unaccepted players do not. Null-victim team awards do not apply kill-only group/aura/rest modifiers and do not emit type35 honorable-kill criteria. Terminal resource-tick replay must not duplicate rewards; both wallet and separate lifetime XP persist.

This checkpoint implements Arathi and Eye completion only. Resource-threshold/objective awards, other per-map callbacks, holiday/random completions and Arena/Conquest remain open; no generic completion amount is substituted.

## Isle commander Honor

Pinned TC Isle `HonorRewards`193–197 and `OnUnitKilled`383–394 grant a literal500 Honor to the opposite team when commander34924 or34922 dies, before ending the match. This is not a bonus-kill multiplier, a generic completion award, or the500 Conquest spell definition. Only an actual source battle-accepted commander death triggers it; the battle's terminal transition rejects dead-target replay. Credit only the winning accepted roster still present in that map copy, with the same positive Honor minimum-level/Inactive checks. Dead eligible recipients remain allowed. Preserve the existing1:1 XP/wallet mirror, holding cap, zero-weekly rule and durable updates.

Resource-node12-per45s awards remain separate unfinished work; winning by reinforcement depletion must not be relabeled as a commander kill. This event does not resolve Arena/Conquest or the lifetime-level curve.

## Sourced weekly counters — 70391

Honor1792 and Conquest1602 both author MaxEarnablePerWeek0 and do not set ComputedWeeklyMaximum0x4. Following TC Player::GetCurrencyWeeklyCap/ModifyCurrency, zero means no static weekly ceiling and new gains must not increment week_earned. Positive weekly caps still clamp accepted gains and count only the actual accepted amount. Existing historical counters are preserved; no unsourced weekly reset clock or Conquest season limit is introduced. The shared wallet implements this rule; persistence must retain zero counters for new uncapped PvP entries.

Authority: TC a352b1fa Player.cpp7131–7166/7357–7360, DBCEnums.h currency flags, current authenticated70391 CurrencyTypes. Exact source hashes/commands and the expected300-versus0 RED are in game-engine `data/diagnostics/pvp-progression-2026-10-16/earned-awards/`.

## Earned-wallet award binding audit — 70391

The prior Honor wallet-binding gaps are resolved by the explicit World of Osso mirror policy above, not by discovering a TC wallet call. `ARENA_COMPLETION_WALLET_BINDING` remains open for Arena/Conquest; source findings below remain factual provenance. RewardHonor/RewardHonorToTeam/EffectGiveHonor credit XP, not1792. Forty-two pinned BG/arena scripts and default BattlegroundScript::OnEnd do not provide a wallet producer. Loaded random-BG Conquest config values alone do not identify an award call. Current local PvpReward1307154 is absent; fresh RewardPackXCurrencyType1307153 decoding yields6 rows/0 unreadable/no1792 or1602. Sixteen current GIVE_CURRENCY spell definitions exist, but no kill/completion caller has been established. Spell definitions/quest reward quantities must not be assigned to arbitrary events.

Conquest sets the100-scaler flag; raw spell/config values cannot be treated as whole display units. WorldState31270/AwardCondition146857 remain separate active-season/eligibility authority gaps. `HONOR_LEVEL_XP_CURVE` remains unchanged. These source limitations must not be misrepresented as existing TC wallet behavior. The authorized Honor mirror can now implement earned-wallet awards; Arena/Conquest and active-season authority stay unresolved. The independently sourced weekly-counter correction alone does not create a payout.

## Gaps

Honor cap15000 is authored. Conquest uses dynamic WorldState31270/AwardCondition146857, not guessed1800/weekly550. Honor/conquest per result and rating policy are not locally derived. PrestigeLevelInfo maps e.g. honor-level5 to achievement12893; no certified Mainline lifetime-XP curve was recovered. The available HonorLevel game table has old Prestige0..32 columns and is not assumed equivalent. Lifetime leveling/reward grants remain blocked by that uncertified curve. Enlisted XP/quest-XP consumers and individual selected spell effects beyond temporary learning remain unproved; aura application and native both-skin selection have bounded proof below.

## Runtime checkpoints

War Mode follows local TC a352b1fa Player.cpp30965–31054: changes require rest/no combat, enabling requires a friendly zone with AreaFlags2::AllowWarModeToggle0x1000 in the area/ancestor chain. Both desired and active states are explicit; desired is durable in existing PvP storage. The actual aura path applies Enlisted282559 inside enable areas and269083 outside, and removes the opposite/disabled aura. TDB world states17042/17043 author baseline10%, not a faction bonus guessed from UI text. Native private UDP5790 proves actual Enlisted282559 application/removal in Stormwind rest area5148, in both skins. Bonus-consumer proof remains open; aura presence alone does not certify XP arithmetic.

PvP selection uses source spec, class-specific SlotUnlock and category masks. Mainline indices1–3 map to masks2/4/8 (20/30/40); legacy mask1/level10 is not a fourth native talent button. Selection refuses wrong spec/level/category, duplicate choices, death/combat, replacement outside rest and replacement while the old spell is cooling down. Source PlayerCondition IDs that cannot yet be evaluated stay explicitly unavailable, never guessed true. Per-spec selection persists in existing PvP records. Existing KnownSpells separates baseline ownership from transient PvP grants so deactivation cannot erase a class-owned spell; existing action-bar override handling is reused. Selection permissions and ongoing activation are distinct: entering combat must not forget a selected active talent. Activation consumes actual aura type119, not a cached Enlisted spell identity; accepted battleground map/instance membership also enables talents. Fresh authenticated player spawn explicitly initializes CombatStatus(false), so a noncombat login cannot silently miss all PvP context. Instance identity is evaluated before terrain lookup: entering map489/instance7 without terrain clears existing Enlisted while retaining the durable preference.

Operator prerequisite: `pvp_*` catalogs are required at normal world startup. Apply the explicit importer before starting a realm with this revision; test/runtime agents do not mutate canonical world.db. Catalog import is not a live progression claim.

## Lead import checklist

Before restarting the landed realm, run the importer from the merged server checkout against the explicit canonical DB path. No import or restart was performed on UDP5000 by this task. Confirm `SELECT count(*)` for `pvp_currency`/`pvp_talent`/`pvp_talent_category`/`pvp_talent_slot_unlock`/`pvp_honor_reward` yields2/407/3/4/24. The importer must finish transactionally before startup; do not seed numeric reward/rating policies. Honor1792/Conquest1602 balances are preserved by the existing one-way ID migration.

Private native proof uses diagnostics-owned world.db and fresh redb, fb_pvp2/Pvpproof on UDP5790. Both skins select Holy talents86/640 through actual pointer controls and display wallet0; cross-process relog retains War Mode and talent86. Screenshots and exact proof boundaries are owned by the [client contract](../../../game-engine/docs/specs/pvp-progression.md). No lifetime XP, match payout or rating progression claim follows from this proof.
