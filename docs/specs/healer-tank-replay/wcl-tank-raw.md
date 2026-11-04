# Raw WCL tank replay

## Contract

`scripts/healer_wcl_intake.py --mode tank` emits `mode: tank_raw` for the configured first actor. The default and explicit `--mode healer` preserve the existing byte-level output contract. Tank gear and talents come from the five recorded CombatantInfo builds. No MID2 profile changes are permitted.

Raw receipts retain time, school, defense type, source actor and instance, ability, periodic flag, logged outcome, unmitigated amount, recorded loss/absorb and time-varying max-health observation. Positive hits without raw amounts, friendly redirects, and hits where raw differs from loss plus absorption plus mitigation are excluded with counted raw/observed-pressure receipts. Unknown raw amounts must not be represented as a known excluded-raw percentage. Self damage is not replayed as hostile pressure; native self damage remains native.

The native replay preserves logged miss/dodge/parry/immune and block/critical classifications. It does not infer unavailable raw rolls for avoided events or roll avoidance again. Native armor, damage-taken modifiers, block amount (including native critical-block chance), Stagger, shields, transfers and hostile-hit procs resolve through the existing melee/spell receipt paths. Raw crit amounts already include the logged critical result; they must not be doubled again. Raw pressure is normalized by observed victim max HP to native victim max HP, then divided by native source-to-HP scaling before resolution. Recorded critical results remain available to procs.

Each enemy profile interval must have a matching `source_keys` entry `[sourceActor, sourceInstance]`. Raw hits must map to an active native enemy with that key: a branded enemy must not stand in for all attackers. Intervals inferred from first/last damage should be rounded outward to the native 50 ms frame. They do not prove exact spawn/despawn times. Native dummy levels remain scenario inputs, not observed WCL NPC levels.

Non-self logged effective healing onto the tank is replayed in chronological order with incoming hits, with damage first on tied milliseconds. Healing is normalized to native max HP, without another healing bonus or critical roll. Native overheal and heal absorption still apply. `pet_owners` in config explicitly maps WCL summons to party-owner slots; unknown owners fail, not silently map to the healer. Non-tank APLs are held during tank replay, so they do not add independent native healing. Opening auras preserve the existing duration/source limitations; external shields or later external defensive buffs are not inferred from healing amounts.

## Exhausted opening Light's Preservation

For static potion shield 1235568 only, a first in-pull buff removal coincident with its final absorbed receipt and a matching partially damaging hit proves exhaustion. The input may restore the sum of that owner's shield-consumption receipts before removal as `absorb_balance`, normalized by the opening observed max HP and labelled `INFERRED_WCL_exhausted_by_partial_hit`. A removal with no matching partial hit does not qualify. No initial amount is inferred for decaying shields. The native receipt restores the already-modified point balance without applying outgoing shield bonuses again. Observed consumption is not scheduled as expiration: native hits spend that pool.

For Warrior `CZ6jq9Q8DJHapMBc` f24 p1, 1235568 absorbs 88,947 and 43,707 at pull 1.163/1.164 s; the final hit also loses 43,312 HP. The restored 132,654 opening points are inferred remaining state, not a potion coefficient or quality. The pre-pull 30-second Buffs/Casts query and the full-key potion Casts query contain no application/use event, and CombatantInfo does not identify a quality. The DB2 spell is shared by several item effects; no production potion defect or automatic potion choice follows from this reconstruction.

## Evidence and limits

The report retains native heal/absorb spell receipts, per-hit health loss, tank cast timing and five-second mitigation frame counts for Shield Block (73) or Demon Spikes (581). Self-heal receipts include frame-end maximum health for normalization. This is a 50 ms simulation, not sub-frame event timing. WCL maximum health observations and opening aura remaining duration are not exact state snapshots. Excluded damage, unresolved external defenses and source scaling must accompany comparisons. Observed cast timing is not a new cooldown trigger policy.

## Verification

- Python tests compare default/explicit healer CLI output to a golden byte fixture generated before tank mode, and exercise raw inclusion, logged avoidance, excluded raw and non-self healing.
- Native raw-hit fixture exercises actual Stagger deposition; the healer fixture proves post-mitigation pressure still bypasses Stagger.
- Logged external-heal fixture proves effective amount normalization and refusal to resurrect a dead tank.
- Per-pull diagnostic reports are evidence, not general tank parity claims. Policy changes require ready-but-not-cast separation from at least 50 casts by several players; medians at cast are not triggers.
