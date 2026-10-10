# Pet battle effect-property coverage

Verified 2026-10-10; authenticated local-CASC build12.1.0.69933; Rust revision `896af2fbecc446b007506daa731f8bd207148c67`.

## Coverage by rows — do not conflate these metrics

| Metric | Before | After |
|---|---:|---:|
| Handler IDs / referenced IDs |12/104|40/104|
| Rows referencing handler IDs |2008/2829 (70.98%)|2605/2829 (92.08%)|
| Rows in recursively admitted ability graphs |930/2829 (32.87%)|1568/2829 (55.43%)|
| Admitted ability records (including empty/passive records) |657/1404|913/1404|

**The >=90% executable-row goal is NOT met.** Handler references exceed90%, but1037 rows referencing implemented IDs still belong to rejected graphs, plus224 rows use64 unsupported IDs. New admission adds638 rows. Static admission is not behavioral proof of every row or current Retail parity. The largest unsupported ID has13 rows; remaining gaps inside common IDs are not rare.

## Original12 Partial handlers — exact changes and remaining gaps

| ID | Previously missing | Added this cycle | Still not established/implemented |
|---:|---|---|---|
|24|OverrideIndex; multi-pet targets; hit/round state bookkeeping; weather family modifiers|Real slot routing for404/419, last-hit/round states, weather family modifiers, shared blocking|103 nonzero OverrideIndex rows; unrepresented team routing; exact Retail rounding/crit coupling|
|26|Special/max-health states; object/proc graphs; zero-accuracy next-swap trigger|Percent max-health cap via state99, source/recipient healing context, Bend Time team trigger|Per-level maxHealthBonus2 requires level metadata; most object/clone/proc graphs; unexplained Accuracy0 rows|
|50|CasterState/TargetState; shared object/team semantics; refresh state|Required-state guards and exact Decoy refresh/team lifecycle|Other object auras such as Cyclone190; reactive proc ordering|
|49|Unused Param1 values; Chance0 scripted removal; callback scope|Unused Param1 accepted; unconditional0 inferred from one-shot Bend Time; source-owned removal|Unsupported callback graphs and unknown target routing still rejected|
|23|Healing recipient versus original source power/dealt modifier|Separate source/recipient for pad healing and correct swapped recipient cap|Ambiguous backline/GM routing; live Retail rounding proof|
|222|Weather family modifiers and shared blocking/caps|Inherited shared damage pipeline with explicit variance|Referenced graph gaps; current export has only zero OverrideIndex for222|
|31|Direct Critter CC immunity and damage/death condition states|Direct CC rejects Critter; hit/round states maintained; ordinary deaths set Is_Dead|Special clone/type/stat semantics and missing metadata remain gated|
|54|Some stack/proc/object graphs and chain semantics|Shared stack/refresh infrastructure retained; last-hit/failure state fixed|Confusing Sting632/Tentacle Stab1962 reactive graphs; nontrivial chain/proc ordering|
|22|Nonzero Points rejected even when tooltip-only|All dummy Points fields accepted without HP side effects|Healthy577's state2 is separate unsupported per-level stat behavior, not a dummy heal|
|52|Weather control immunity|Arcane Winds prevents control; existing Critter/Resilient and action-aware timing retained|Exact Retail CC/expiry ordering remains unobserved|
|53|Swapped recipient max HP and weather healing modifiers|Percent heal uses recipient cap and source/recipient modifiers|Ambiguous GM team targets; no universal runtime parity certification|
|63|Source power lost on swaps; Photosynthesis wrongly treated as a pad|Preserve source power; Wish/Tranquility/Mists follow active; Photosynthesis stays wearer-bound|All4 rows admitted; runtime timing/rounding still needs independent Retail evidence|

"Dispatch" below means the admitted parameter/state/target subset is implemented. It does not mean all rows or all referenced abilities are supported. Isolated dummy/trigger tests identify their narrower proof boundary explicitly.

## Full104-property matrix (descending row count)

| Property | Rows | Admitted rows | Implementation / boundary | Real-ability fixed-RNG proof |
|---:|---:|---:|---|---|
|24|945|508|Partial: zero OverrideIndex; slot routing404/419; other team routing rejected|Bite110; Sunlight404; Tidal Wave419|
|26|244|155|Dispatch; special-stat/proc graphs and unexplained Accuracy0 gated|Burrow159; Metabolic Boost194; Bend Time1905|
|50|194|98|Dispatch: state gates, max stacks/refresh; non-Decoy object graphs rejected|Curse218; Moth Balls507/effect1165|
|49|138|55|Dispatch: chance/default0, unused Param1; callback graphs gated|Burrow159; Failsafe723; Bend Time1904|
|23|115|85|Dispatch: source power/dealt modifier, recipient cap/taken modifier|Healing Wave123; Tranquility254/255|
|222|90|78|Dispatch: explicit variance, weather family modifiers, central Magic cap|Burn113; Burrow159|
|31|86|69|Dispatch: absolute state, Critter direct-CC immunity; unknown state semantics gated|Deep Breath169; GM Stun292|
|54|77|54|Dispatch: chain guard, duration/max stacks; reactive proc graphs gated|Poison Fang152; Aquatic passive|
|158|69|57|Dispatch: stop remaining effects after failed chain|Death Coil121 miss/hit|
|22|61|40|Dispatch: all Points fields are tooltip metadata, no HP operation|Healthy577 (isolated dummy); Curse218|
|116|57|46|Dispatch: priority evaluated before order; equal priorities use speed/RNG|Quick Attack563; Belly Slide529|
|52|44|36|Dispatch: control immunity including Arcane Winds; action-aware expiry|Time Stop600; Grasp249|
|28|29|23|Dispatch: cancellable/self aura; shared Decoy charges; other objects gated|Decoy334/333 swap/refresh|
|85|29|13|Partial: reset auras with named state; missing state161 remains rejected|Tidal Wave419 clears objects|
|79|28|19|Dispatch: additive bounded states; priority cleanup bound to original actor|Quick Attack563; forced swap780|
|177|28|12|Dispatch: target-state equality stops later branch, not prior damage|Frog Kiss233/Resilient149|
|135|27|3|Partial: grounded self-kills282/663/2209 only; other routing rejected|Explode282 skips Failsafe|
|32|25|13|Dispatch: percentage of actual last hit, no second power scaling|Death Coil121 restores42 of84|
|80|24|11|Partial: global weather, exact delta reversal, Elemental exemptions; flat-source scaling gated|Darkness256; Moonfire595; Sunlight404; Scorched Earth172|
|178|24|15|Dispatch: target-state/chain immunity guard and immune reporting|Bash348 immune/skip|
|29|23|10|Dispatch: additional damage only with required caster/target state|Takedown221 normal/stunned|
|62|23|8|Dispatch: target max-HP percentage without power/variance; typed damage/cap|Trample377|
|384|23|23|Dispatch: faster/first-strike conditional attack|Tail Sweep122 faster|
|141|22|21|Dispatch: split pool across living enemy pets, conditional weather bonus|Arcane Storm589 three/two targets|
|27|21|20|Dispatch: capped ramp; hits156/930 versus uses for others|Frog Kiss233; Vicious Fang156 miss|
|136|19|16|Dispatch: personal aura cleanse, not global weather/team objects|Sear Magic763|
|145|18|12|Dispatch: highest-health live bench swap; displaced action cancelled|Sweep457|
|373|17|8|Dispatch: zero-based ability slot assignment; surrounding boss states gated|Necrotic Reconstruction2262 (isolated trigger)|
|139|15|12|Dispatch: lowest-health live bench swap; displaced action cancelled|Death Grip780|
|157|14|4|Dispatch: caster-state equality gates remaining branch|Belly Slide529 Blizzard state58|
|111|13|2|Partial: self resurrection; ambiguous team resurrection gated|Failsafe723 restores2000, clears dead|
|179|13|0|No; explicit rejection|—|
|229|13|0|No; explicit rejection|—|
|246|12|6|Partial: zero modification clears one-based slots; nonzero modification undetermined|Bend Time1905/1904 next incoming pet|
|68|11|3|Partial: source-HP percentage, grounded self-sacrifice/enemy routing; other graphs gated|Explode282|
|104|11|9|Dispatch: required-state heal, explicit variance|Devour538 only after death|
|53|10|7|Dispatch: recipient maximum-HP percentage, no second power scaling|Wish273/274; Recovery726|
|159|10|0|No; explicit rejection|—|
|0|9|0|No; explicit rejection|—|
|107|9|0|No; explicit rejection|—|
|131|9|0|No; explicit rejection|—|
|156|9|0|No; explicit rejection|—|
|363|9|0|No; explicit rejection|—|
|150|8|0|No; explicit rejection|—|
|76|7|0|No; explicit rejection|—|
|96|7|0|No; explicit rejection|—|
|100|6|0|No; explicit rejection|—|
|168|6|0|No; explicit rejection|—|
|170|6|6|Dispatch: weather-gated additional damage|Solar Beam753 Sunlight state61|
|226|6|0|No; explicit rejection|—|
|25|5|0|No; explicit rejection|—|
|121|5|0|No; explicit rejection|—|
|122|5|0|No; explicit rejection|—|
|44|4|0|No; explicit rejection|—|
|59|4|0|No; explicit rejection|—|
|63|4|4|Dispatch: team pad versus wearer; retain source power through swaps|Wish273; Tranquility254; Photosynthesis268|
|112|4|0|No; explicit rejection|—|
|169|4|2|Partial: negative-state damage; front-pair weather routing; reactive events gated|Scorched Earth171/172 Elemental exemption|
|172|4|0|No; explicit rejection|—|
|372|4|0|No; explicit rejection|—|
|385|4|4|Dispatch: slower/last-strike conditional attack|Tail Sweep122 slower|
|66|3|0|No; explicit rejection|—|
|128|3|0|No; explicit rejection|—|
|129|3|0|No; explicit rejection|—|
|230|3|0|No; explicit rejection|—|
|248|3|0|No; explicit rejection|—|
|370|3|0|No; explicit rejection|—|
|77|2|0|No; explicit rejection|—|
|117|2|0|No; explicit rejection|—|
|123|2|0|No; explicit rejection|—|
|124|2|0|No; explicit rejection|—|
|138|2|0|No; explicit rejection|—|
|149|2|0|No; explicit rejection|—|
|194|2|0|No; explicit rejection|—|
|197|2|0|No; explicit rejection|—|
|223|2|0|No; explicit rejection|—|
|249|2|0|No; explicit rejection|—|
|254|2|0|No; explicit rejection|—|
|255|2|0|No; explicit rejection|—|
|256|2|0|No; explicit rejection|—|
|360|2|0|No; explicit rejection|—|
|45|1|0|No; explicit rejection|—|
|61|1|0|No; explicit rejection|—|
|65|1|0|No; explicit rejection|—|
|67|1|0|No; explicit rejection|—|
|72|1|0|No; explicit rejection|—|
|74|1|0|No; explicit rejection|—|
|75|1|0|No; explicit rejection|—|
|78|1|0|No; explicit rejection|—|
|86|1|0|No; explicit rejection|—|
|97|1|0|No; explicit rejection|—|
|137|1|0|No; explicit rejection|—|
|140|1|0|No; explicit rejection|—|
|144|1|0|No; explicit rejection|—|
|165|1|0|No; explicit rejection|—|
|171|1|1|Dispatch: required-state additional heal|Photosynthesis267/268 doubles sunny tick|
|201|1|0|No; explicit rejection|—|
|204|1|0|No; explicit rejection|—|
|358|1|0|No; explicit rejection|—|
|359|1|0|No; explicit rejection|—|
|362|1|0|No; explicit rejection|—|
|374|1|0|No; explicit rejection|—|
|376|1|0|No; explicit rejection|—|
|380|1|0|No; explicit rejection|—|

## Undetermined / deliberately rejected, not fabricated

- **Target/override representation:** property24 OverrideIndex has103 nonzero rows, but labels/descriptions do not establish variance/critical/hit reuse. Broad team/bench descriptions without a implemented route are rejected; Sunlight404 and Tidal Wave419 have explicit three-slot routes. Heal Back Line70/GM Heal295/GM Revive714 and unknown135 kill routing are rejected rather than selecting the caster repeatedly.
- **State/special stat semantics:** state161 referenced by85 is absent from BattlePetState; no invented category. State2 maxHealthBonus is per-level, but Pet has no level field. Clone/type-override/boss state graphs remain gated. Sunny Day403 has state99=50 and65=25: max cap15000 and heal225 from Healing Wave30*6*1.25 are tested; no healing effect is invented on weather application. Current-HP transition/rounding has no captured Retail proof.
- **Reactive and conditional aura semantics:** proc events1..4/9/12 remain rejected because current data does not determine precise callback ordering/recursion, hit ownership and aura expiry interaction.179 (13 rows) needs retaliation ownership;229 (13 rows) needs conditional type-state polarity/refresh semantics. These are the two largest unsupported IDs. Complex chain values in632/1962 and special immunity/reverse/passive options in135 remain unproven.
- **Weather/terrain and cooldown boundaries:** Darkness, Moonlight, Sunlight, Arcane Winds and Scorched Earth have real-ID CPU cases. Flat-modifier weather (Sandstorm454) is rejected until the tooltip AttackBonus power owner is established; no attacker/source substitution. Cleansing Rain duration changes, Lightning proc damage and non-Decoy terrain/object effects are not certified.246 modification1 in Forbidden Hourglass1932 is not inferable from its terse description, so only0 clearing executes.
- **Numerical/runtime boundary:** inherited weak multiplier.67, family passive/crit constants and rounding are not newly invented, but exact current Retail game-table/observed timing remains absent. No live Retail comparison or independent verifier PASS; Claude OAuth expired before verification ran.64 unsupported IDs are listed in the matrix; not every omitted rare handler is intrinsically unknowable.

## Data citations / derivation

Source of new semantics is read-only `data/world.db` imported from authenticated local CASC. Join `battle_pet_ability_effect.BattlePetAbilityTurnID` -> `battle_pet_ability_turn.BattlePetAbilityID` -> `battle_pet_ability.ID`, and `BattlePetEffectPropertiesID` -> `battle_pet_effect_properties.ID`; resolve state IDs through `battle_pet_state` / `battle_pet_ability_state`.

- Death Coil121 effects288/289/2137 (24/158/32):14 points,50% last-hit heal ->84 damage/42 heal at power100. Param labels identify Points/Accuracy/ChainFailure; actual hit accounting is not another power-scaled heal.
- Frog Kiss233 / Arcane Blast421 labels PointsIncreasePerUse/PointsMax; Vicious Fang156 / Huge Fang930 descriptions explicitly say "each time it hits", unlike ramp-on-use descriptions. Tests distinguish a miss.
- Darkness257 states66=-50,41=-10,82=1,56=1; Moonlight596 states66=25,87=10,89=6,55=1; Sunny Day403 states65=25,99=50,61=1. Elemental241 description supplies negative-weather immunity. Arcane Storm589 labels BonusState54/BonusPoints4; its description says split among enemies. Pool24*6/3=48 or(24+4)*6/2=84.
- Decoy333 states69=1,74=2,75=333; Decoy334 description says block two attacks. Tranquility254 and Renewing Mists511 explicitly persist through pet swaps; Photosynthesis268 does not describe a team pad, and its conditional heal267 tests Sunlight61.
- Explode282 explicitly prevents Failsafe; its68 Points40 is source-health percent and135 kills the caster. GM Kill All Enemy967 contradicts that same default target, so its graph is rejected. Tidal Wave419 explicitly damages enemy pets and destroys both teams' objects. Failsafe723111 Percentage20 restores2000/10000, independent of power.

All hand-derived expected numbers are comments beside the actual assertions in `crates/server/src/pet_battle/property_tests.rs`; pre-existing cases remain in `tests.rs`. No battle data staged or committed.

No local TrinityCore pet-battle effect interpreter was found in known reference locations. Diagnostic `PetBattleAbilityEffect.cpp`/`PetBattle.cpp` files are **LegionCore9222216**, not TrinityCore or current Retail authority; their explicit unknown/no-op removal and kill handlers were not adopted. Retained TrinityCorea352b1fa stat provenance and local Retail SharedPetBattleTemplates formula provenance live in the [engine wiki](wiki/systems/pet-battle-engine.md#numeric-provenance), not a claim that TrinityCore implements these new properties.

## Proof / reproduction

From the leased slot, no dependency override/CSV environment needed:

```text
/home/osso/Projects/world-of-osso/game-engine/scripts/agent/agent-run pbfx cargo test --locked -p server pet_battle -- --nocapture
```

At Rust896af2f:55 engine tests +7 wild/startup tests pass (62 total), no failures/ignored cases within the filter; unrelated1735+18+4+3 tests filtered, not run. `cargo fmt -p server -- --check` and `cargo check --locked -p server` exit0, no warnings. No full suite rerun, client/live capture, merge or protected-service work.

Durable proof root: `/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/pbfx-20261010/`. `batch1-red/green/green2`, `batch2-red/green`, `batch3-red/green`, `batch4-red/green`, `bend-time-red`, `admission-refresh-red`, `priority-owner-red`, `priority-class-red`, `final-targeted`, `fmt-check`, `cargo-check` logs retain actual intermediate failures and final outputs. Batch2's initial Takedown expected value omitted the Humanoid-vs-Beast weak multiplier; the corrected test uses a neutral Humanoid defender. Batch3's initial Belly Slide expectation incorrectly assumed Order was the first event despite its begin-round AuraApplied; it now checks the actual Order event. These are fixture corrections, not extra passing proof.

Machine census: `coverage.json`; authentic joins/labels/state IDs: `effect-evidence.json`, `property-labels.json`, `state-evidence.json`. The final requested command ran once after the last Rust change; documentation-only changes do not invalidate it.
