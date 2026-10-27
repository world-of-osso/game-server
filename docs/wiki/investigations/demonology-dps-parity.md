# Demonology DPS parity

Lantha Shadeborne (follower230861, specialization266) qualification against public SimC76f8118. Reference reproduction and source census are established; native parity is **not certified**.

## Reference reproduction — 2026-10-23

Exact1000-iteration commands reproduced196789.57490042652 DPS for300s single-target and420736.290114185 DPS for the300s two-add/30s-wave fixture. Both use level90 MID2 Demonology Diabolist BiS, seed70448, target_level93, fixed_time1, vary_combat_length0, optimal_raid0, item_db_source=local. Three-target command intentionally omits Patchwerk: the preset clears explicit raid events.

Acceptance bands: ST177110.61741038388–216468.5323904692;3T378662.6611027665–462809.9191256035 DPS. Compare native31146 encounters with identical equipment and effective talent ranks; dungeons retain their native kit.

Reference `stats_pets` is a map, not a `pets` list. Recursively include each stat's `children` and divide raw damage sums by the player's DPS sample count999, not a conditional guardian's sample count. This reconciles196789.57490042632 DPS without parent compound double-counting:

| Source | DPS | Share |
|---|---:|---:|
| Owner |59016.525249368|29.98966%|
| Felguard |7940.939825147|4.03524%|
| Temporary summons |129832.10982591129|65.97509%|

Largest channels include Wild Imp10431822.67446%, Tyrant12640939.07443%, Demonbolt2641785.48728%, isolated Implosion13060775.47622%, Imp Lord12771164.45704%. These are full-reference shares, not oracle probe shares.

## Native data boundary

Private SQLite backup of canonical engine `data/server-world/world.db` has SHA256 `51455391e7d4962589cc66d239bf77b2dbf3f4372ec8daee80c092f1e7989d9b`. It lacks published `content_creature_template` rows for Wild Imp55659 and Dreadstalker98035; corresponding TDB templates exist. Queried `guardian_spells` contains no rows for55659/17252/98035/135002/250289/258584. Missing published templates/bindings are state/input gaps, not evidence of a coefficient bug.

Native guardian policy data binds Wild Imp55659→104318, base Tyrant135002→270481 and Antoran Tyrant250289→1264093 by actual summon-root/creature pair. Public SimC `sc_warlock_pets.cpp1510–1548` selects250289 via native265187effect3. Its two-second hasted repeat GCD is explicit SimC policy; native1264093 supplies damage and geometry. Both Tyrant variants retain the base pet's owner-SP1.0 (`pets.cpp19`); Wild Imp retains1.2 (`pets.cpp705`). Native1264093 requires caster aura1251000. The Antoran spawn receives that native weapon mode instead of bypassing aura admission. Imp Lord receives native Service216187 as pinned `pets.cpp56–57/1714–1719` requires. Unknown guardians still use their actual imported creature spell lists.

The damage-channel census records63 spell IDs/101 native DB2 effect rows. Including75 active selected talent entries expands it to131 IDs/227 effects, with aura options, PPM rows, powers, cooldowns, class masks and labels. Raw TDB1210 `spell_script_names` matches686→`spell_warl_shadow_bolt`,264178→`spell_warl_demonbolt` and three utility spells710/108416/111400 across that expanded census. No matching signed `spell_proc` or PPM rows were observed. TC a352b1fa `spell_warlock.cpp691–710` implements Demonbolt's after-cast energize; absent TC scripts do not authorize invented summon timers or multipliers.

## Literal fixture controller

Spec266 registers its follower-only adapter in the shared `SpecDriver` registry and executes the profile's exported action rows through shared `literal_rotation::Driver`, `Prepared`, `ConfiguredPet` and equipment facts. The adapter supplies native identities, actual owned Imp/Tyrant counts, native target context and source action readiness; it does not create another interpreter. Shared controller capture precedes native warmup; Demonic Core reaction settings are configured before Demonology's pet setup and precast. The default Felguard and harmful precast use normal native casts before pull. Native30146 costs one shard, while declared starting shards are zero; setup waits for ordinary funding without granting or resetting resources. Ordinary dungeon followers retain the native kit. Potion inventory and external Power Infusion remain explicit gaps.

The master210df12e3 census was12839.69/196789.57 ST DPS (6.524%) and16544.99/420736.29 three-target DPS (3.932%). Missing summoned damage dominates this baseline. Its aggregate report does not expose per-action crit rates. A private snapshot must include current published55659/98035 templates; the older2026-10-23 snapshot lacks them even though current canonical data includes both. Refreshing private input state is not a spell-coefficient fix.

Native193331 initially failed with `target43 requires collision data`. Origin-centered parity fixtures now select explicit open LOS and a flat floor at0, without dungeon walls or baked-eye rows. Production maps still require their native bake. The actual native Dreadstalker now spawns and expires; its demise regression reached missing Core (0 instead of1), which the previous collision failure hid.

Pinned `pets.cpp1233–1257` rolls native267102 effect1 on each Dreadstalker demise when Demoniac is known. This uses ordinary native264173 stack application, not the Imp's accumulated fade distribution. The policy runs at actual summon removal. The train's real-expiry regression passes with one Core after the Dreadstalker is removed.

## Follower minion target permission

Lantha is an NPC with `UNIT_FLAG_PLAYER_CONTROLLED`. Native attack admission uses `faction::player_affected`, but summon initialization checked only for an owner `Player` component. That mismatch leaves a follower's actual minion on the NPC-versus-neutral attack path. The minion flag producer now uses the same owner predicate as attack admission, retaining native minion ownership and ordinary NPC restrictions. Train qualification proves the neutral-target permission regression and both Tyrant damage/SP tests fail before this fix and pass afterward. The full literal test now reaches unsupported `cooldown.summon_demonic_tyrant.remains_expected`, so no full-duration DPS result is available yet.

## Native cooldown estimates and Wild Imp exhaustion

The shared cooldown query implements pinned `cooldown.cpp603–615` expected-remains estimation from the native timer's actual start, current duration and remaining time. Timer starts survive reductions and rate changes; charge cycles retain their first start rather than resetting it on later uses. Reads do not advance timers.

Pinned `pets.cpp701/875–882` disables regeneration for actual Wild Imps. The normal player-controlled power path otherwise grants native Energy regeneration, which can prevent exhaustion and the associated Core grant. Actual104317/55659 guardian initialization now records the source's no-regeneration policy; both runtime regeneration and APL potential-regeneration facts use it. Native pool size and Fel Firebolt costs are unchanged. Train qualification is pending.

## Literal minion attack commands

The first full300s literal measurements still had zero Felguard and guardian output. Lantha's actual `CombatTarget` remained absent throughout combat. The direct Tyrant fixture had manually set that target, so its damage proof did not cover the controller boundary. The failed qualification still wrote diagnostic17098.57 ST/19645.99 three-target DPS; these are not certified parity results.

During combat, the Demonology adapter now retains the supplied primary enemy on the actual actor and orders its existing configured pet through normal `order_companion_pet`, matching the established BM adapter path. Guardians read that owner target normally. It does not inject owner melee, change native spell casting targets, issue precombat attacks or turn a `target_if` recipient into the default primary. The literal regression now requires real output from both a guardian and the configured Felguard. It passes, and full300s native qualification now records40564.06 ST/82175.60 three-target DPS (20.61%/19.53% of reference). Parity remains far outside the target band.

## Guardian secondary stats

Actual native Fel Firebolt used2000ms despite the owner's19.45591% haste; the source expectation is1674ms. Executing `GuardianActor` units now resolve their actual `SummonedCreature` owner for spell haste, with one timing reader shared by cast/GCD/channel/duration calculations. Spell-specific modifiers remain on the guardian. Native pool/stat identities are not copied or changed. The actual Fel Firebolt cast-duration regression passes.

Actual Burning Cleave separately observed zero critical chance against the owner's35.065216%. The landed `combat::read_pet_owner_critical_chance` helper now also resolves executing guardians through the same owner reader used for haste. Both native swings and spell critical-chance calculations consume that one helper. Existing Hunter qualification and spell-specific modifiers are retained. The native guardian critical-chance and WW white-crit regressions pass. The separate Hunter100%-owner control is a known pre-existing fixture failure: ordinary stat folding restores the actual owner's crit before its assertions.

## Implosion admission and execution

The real six-Imp regression rejected ordinary196277 with `CasterAuraState`: native metadata requires296553, but the Imp lifecycle never supplied it. Real Imp spawn/removal now maintains that native availability aura. The sourced execution selects from eligible owned demons at native effect0's limit, samples participating Imps without replacement, and launches lower-Energy/older Imps first. Native196278 damage scales its selected primary by remaining Energy; secondaries retain full native damage. Actual Imp retirement follows its impact and uses the independent native Demoniac chance, not the natural-fade accumulator.

To Hell and Back reads native1281511's group/count values and casts actual279910, binding143622 to the same native Fel Firebolt and no-regeneration behavior as55659. Only the actual replacement actors receive1250772/1281512. The private `world.implosion.db` is a backup of the existing `world.current.db` with exact143622 template/model publication via the existing importer; canonical data and in-flight train inputs stay unchanged. GREEN qualification is pending.

## Owner passive SpellMods

Native and pinned reference Fel Firebolt both use SP coefficient0.192178. The current native Imp instead misses selected owner passive modifiers: its first real bolt leaves80 Energy rather than84, and its observed critical chance lacks native Imp-erator's15 percentage points. These are native RED failures, not coefficient differences.

Pinned `player.cpp15268–15324` delegates passive spell/power/effect reads to the pet's owner; `sc_warlock_init.cpp120–122` registers class, selected-talent and current-set passives. The shared modifier reader now combines the caster's existing modifiers with self-owned, uncharged passive owner auras that actually match native family/label masks. Native334591 supplies the Energy discount,416230 the critical bonus and1296573 the set damage amount. Active/charged owner buffs are not copied or consumed. GREEN qualification is pending, including BM/Hunter pet controls.

## Dominion and guardian execution

The source-policy table also binds native Dreadbite271971, Greater Felbolt1277116 and the four Dominion actions. Dreadstalker AP inheritance1.06769 and Imp Lord SP2.75 are validated in pinned `pets.cpp`; the unvalidated Dreadstalker SP1.3 and Imp Lord AP1.375 are not imported. A fresh Imp Lord starts with200 Energy and waits for125 before casting. Native SpellPower retains the40-Energy cost; native regeneration remains in control. A funded ordinary-summon probe found the generic `update_max_powers` fold replaced the initialized200 with100 and clamped the current pool, so the125 gate could never open despite ready admission. That fold now reads the same sourced guardian base as initialization. GREEN qualification is pending.

`stacking_summons` records the missing Tyrant/HoG/Ruination counter relationships. Native1276166 owns the cumulative limit and duration. Native1276318 owns Dominion III's shard refund. The four summon choices follow the pinned uniform roll and unique-demon redirects. Soul Barrage's missing impact relationship uses native1292391 and the parent effect's target count and range. No spell coefficients or canonical data are changed.

Private measurement data also needs exact native templates for the four Dominion roots1282501/1282502/1276283/1276182. The existing importer can publish them on a private copy with repeated `--spell`; canonical data currently lacks these four rows. The current-input native-kit ST control remained12839.69 DPS even after the older Imp/Dreadstalker rows were refreshed, so that refresh alone is not a DPS fix.

## Explicit summon-root publication

`add_guardian_templates.py --spell104317 --spell193331 --spell193332` (each option and number separated by a space) selects native difficulty0 Effect28/56 creature IDs, deduplicates them and copies exact TDB templates/models through the existing retail-content importer. These roots resolve to55659 and98035. Explicit roots replace the default Darkglare/Wild Imp selection; they do not add fabricated spell lists, alter world spawns or broaden the default Wild Imp publication. Missing summon metadata fails before publication; reruns preserve the same model rows.

## Evidence

Artifacts under engine `data/diagnostics/demo-parity-2026-10-23/`: `266.reference.{single,three}.{json,txt,proof.json}`, `reference-channels.json`, `source-census.json`, private `world.db` and `world.sha256`. Prior source references are `dps-parity-2026-10-21/266.reference.single.*` and `266.reference-fixed.three.*`, not `all-spec-expanded-references/266.*`.

## Sources

- [Oracle contract](../reference/simc-spell-oracle.md) — match actors/formula IDs and compare first differing factor.
- [Parity contract](../../specs/dps-parity.md) — matched inputs and native-boundary requirements.
- [Guardian dispatch](../../../crates/server-combat/src/spell_cast/guardian.rs) — actual spell bindings and native admission.
- [Wild Imp source importer](../../../scripts/add_guardian_templates.py) — publishes qualified55659 only, not unqualified98035.

## See Also

- [[dungeon-simulator]] — fixture measurement.
- [[simc-apl]] — literal APL execution and native facts.
