# Boss scripts

## Encounter coverage continuation — 2026-10-12

- [x] Census every hosted DungeonEncounter difficulty record and Journal-only encounter, preserving unknown/ambiguous identities and separating structural admissions from behavioral proof: [coverage](../encounter-coverage.md).
- [x] Order work using locally authenticated Midnight Season2 relations, then Normal leveling LFG ContentTuning.MinLevelSquish, then remaining maps by descending expansion. This does not activate a Mythic+ season.
- [ ] Complete Murder Row/Kystia3101 from pinned TC a352b1fa: primary Chaos Barrage1s/3500ms, Mirror Images14s/30500ms and Heroic+ Fel Nova12s/15s are ported; selector→Blink→Fel Nova spell hooks are ported too. Initial JustAppeared/FelCrazed four-stack Felshield is ported. Nibbles constructor unkillable flag is ported through spell/periodic, melee and direct damage, following TC Unit.cpp936–945 (non-self lethal damage leaves1HP). Nibbles primary/friendly phase entry is ported: frame priority2, authored Fel Spray8s/51s, Spittle4500ms/14500ms,1s health check against1230289 effect0, friendly faction/display and Light Infusion's local2s cast/15s channel. Successful channel completion dispatches the source callbacks for Illicit3s→3/2/1 shield renewal/faction16/display126199, but full-cycle acceptance is blocked: local Light Infusion15s completes while Destabilized1265412's20s self-stun persists. Pinned TC474238 flags0x40020 contain only IGNORE_CAST_IN_PROGRESS/DONT_REPORT, not IGNORE_CASTER_AURAS or FULL_MASK; both are excluded from TC IS_TRIGGERED_MASK. It correctly receives CrowdControlled(Stunned). No approved correction exists; neither timer/flag is changed. Named failing acceptance gate is retained explicitly. Ignore-cast-in-progress preserves all other cast checks. Explicit script root/visual-kit behavior, Fel Spray area-trigger damage, doors/boundary and loot remain open. Mirror Images' source periodic callback and random destination offset are ported; wild spell summons preserve authored Normal difficulty/SmartAI and boss ownership. Felstorm aura removal retires the creature after its source1ms delay without death credit. Felstorm damage and other mirror difficulties remain uncertified. Cast onset and completion must respect local DB2 cast times (Chaos Barrage3000ms, Fel Nova3500ms); source flags ignore only cast-in-progress, not school lockouts or cast time.
- [x] Prepare mirror255050 template/model and its actual Normal difficulty1 row from local TDB + DB2 summon1264098; `scripts/encounter_summons.py` changes independent copies only, preserves population and unrelated templates. Other difficulty rows remain absent source gaps. Kystia behavioral fixtures prepare this source data inside isolated temporary snapshots; canonical publication is not a prerequisite for their gate.
- [x] Prepare Kystia primary-actor binding SQL from pinned TC instance/header + local DB2, retaining unrelated bindings and never granting Nibbles kill credit: `scripts/encounter_cpp_bindings.py`. Lead applies SQL to a reviewed copy; private application is not publication.
- [x] Prepare Golden Serpent2139 primary135322 (map1762/bit0, Journal2165/actor4904/display84202) from pinned TC instance/header/script plus local DB2/TDB. `encounter_cpp_bindings.py --encounter golden-serpent` emits lead-copy SQL only; Animated Gold135406 never grants boss credit. `encounter_summons.py --encounter golden-serpent` admits Animated Gold's actual base difficulty0/template models on independent copies without new placements/credit or manufactured difficulty rows. Animated Gold's sourced lethal-loss floor includes self damage and commits before death/loot; original incoming loss drives its one-time Molten Gold cast, PC immunity/uninteractibility and removal cleanup. Appearance is passive. Its first lethal transition clears active scripted point movement and stops the replicated gait without changing walking mode; follow/absorption remains unported. Spit Gold effect0/periodic-damage removal summons Animated Gold at the target's position only for expiry/death, not dispel/cancel/evade. `encounter_model_geometry.py --db <independent-copy> --source-dump <local-TDB1210.sql>` prepares the authentic template-selected Serpent/Gold model-info rows and source invisible display11686, retaining BoundingRadius/CombatReach/other-gender/build without activating movement. Current Retail LittleWigs replaces legacy cast registration and uses native timeline inputs; obsolete `OnEngage` clocks are not alert authority. Native timeline-backed alerts remain open. Native molten area visuals remain open; no friendly-intro bypass or whole-boss certification.
- [x] Gate Molten Gold aura395 using the authored static sphere12928 attached to its target, Player-only ordinary null-target cast265914 on enter and removal on exit/area retirement (valid Player selection first, otherwise caster). Include source reach/object scale, vertical bounds, map-copy isolation and real periodic damage; native area visuals remain open.
- [x] Gate Golden's **TC script schedule (partial; Retail 12.1 timeline inputs unavailable)**: Spit9s/11s (random threat-list player within300yd), Gust12.5s/21.5s self, Tail15.5s/17s victim, Lucre40s/41.5s self. Use ordinary source cast flags/local DB2 cast durations; due events wait while casting. Keep Golden Retail boss-mod alerts disabled, including stale catalog entries. `NotStarted` stays friendly/uninteractible until the authored landing path ends; no synthetic landing unlock. Source `InitializeAI` relocates already-started actors; evade retires summons/population actor and uses actual30s respawn/FAIL without credit. Area-trigger effects and native acceptance stay open. TC combat timing is explicitly approved independently from unavailable Retail alert inputs; not12.1 timing certification.
- [x] Gate Golden's `NotStarted` intro: gravity disabled, friendly35/uninteractible, emerge271598 cast, then source `MovePath` 13532200 → 13532201 → 13532202 from TDB `waypoint_path` (Land move type, template run speed, `DoInitialize` 1000ms wait before each path, exact/smooth Catmull-Rom spline for the flying path, TC update order). Submerge end removes 271598; landing end restores gravity, FACTION_MONSTER_2 (16) and selectability at the source respawn point. Fly anim tier/hover visuals have no replicated state and remain open.
- [x] Gate Golden's appearance on the instance script: manual spawn group1256 is withheld at copy fill and spawned into a copy when its last living TempleEvent (TDB `creature.StringId`) creature dies, unless Golden is already Done or present. Group1255's source trigger is the Serpentine Seal288466 gossip: 3s later it summons Shadow of Zul with script StringId `SerpentBossEvent`, whose event spawns 1255 at +10s.
- [x] Gate the Serpentine Seal → Shadow of Zul chain: 1255 is withheld at copy fill; using the seal once (then refused while in use) summons Zul after 3s; Zul's `JustAppeared` sees the script StringId and runs +1s Talk0/269905/270692, +10s 1255 with 269623 per spawn, +15s Shadowform removal, +16s invisible display and 1s unsummon; the 17th TempleEvent death then spawns the Serpent. `JustAppeared` runs at the creature's next update, before sight/aggro. Pool GO state/collision is covered below; native visuals remain open.
- [x] Gate Shadow-Borne temple trash (134157/134174/134158): 269619 visual on appearance and the source engage schedules at their authored first/repeat delays. Minion of Zul is covered separately below.
- [x] Gate Zul's +1s pool289347 activation: retain spawn GO_STATE_READY1, set authoritative ACTIVE0 and disable collision only in Zul's copy. Native state-to-animation mapping remains open.
- [x] Gate Minion of Zul133943: TempleEvent-only summon visual, Bound by Shadow269935 on appearance, Fixate269936 on engage; one area enemy within300yd, preferred over higher threat while listed, cleared on removal. Different casters' channelled auras coexist. Absorb depletion/dispel kill the bearer; cancellation does not. Native visuals remain unproved.
- [x] Gate Gust Slash AT17933: TDB cylinder/orbit geometry, clockwise radius-5 orbit with radius blend, 30s life, one source 269932 cast per entering living player (resolved through the player's hostile selection), no repeat while inside.
- [ ] Gate Animated Gold's real Lucre SpellHit→3s aura check→authentic same-copy walk/follow→arrival/triggered Luster/one announcement→4s unsummon, with real Kings' Rest navigation and no unbaked-map direct shortcut. Preserve source3D reach/target displacement; roots and movement-interrupting casts must pause/resume the baked route; near-point collision/floor and full motion-priority equivalence remain partial until behavioral proof.
- [ ] Certify every boss through engage/casts/phase/death/credit/loot/lockout, and both native skins per completed instance. No complete instance claimed by this continuation.
 (server)

Dungeons phase 3: creature scripts selected by `creature_template.ScriptName`, ported from TrinityCore master a352b1fa (`CreatureAI`/`ScriptedAI`/`BossAI`, `InstanceScript`, `CreatureTextMgr`) and the Stockade scripts (src/server/scripts/EasternKingdoms/TheStockade/). Code: `crates/server/src/scripted_ai/`. Protocol: shared-protocol `protocol/encounter_messages.rs`, `ChatType::Monster*`. Client: game-engine `docs/specs/boss-encounters.md`. Data: `content_creature_text` (TDB1210 creature_text, `scripts/import-creature-text.sh`) with `content_broadcast_text`.

## Ragefire Chasm source hooks

- [x] `encounter_serverside_spells.py` prepares Adarogg's actual local TDB `serverside_spell`/`serverside_spell_effect` rows119297/119298, preserving all source columns and unrelated copy data; canonical/symlink/hardlink inputs and incomplete source records are refused. No DB2 substitutes or derived effect IDs.
- [x] The spell store loads those tables like TC `LoadSpellInfoServerside` (DifficultyID0): every modelled source field becomes SpellInfo; an ID already in DB2 is refused, never overridden. Copies without the tables have no server-side spells.
- [x] `encounter_summons.py --encounter adarogg` admits Demonic Leap61409 (actual base row/models plus authored Normal difficulty1) only after the server-side import establishes 119297's summon identity.

Leveling tier begins at locally sourced MinLevelSquish7 (Ragefire Chasm, then Deadmines). Source: TC a352b1fa `RagefireChasm`.

- [x] Gordoth61528 relocates already-started actors to its source landing point, casts Ground Rupture119999 at7.3s (native1s cast) and Seismic Slam120024 at13.3s with only source flag0x2. It enrages50420 once when projected health falls below31%, resetting through source30s evade/respawn.
- [x] Flag0x2 per-cast clones skip SpellHistory::IsReady and HandleCooldowns (including charge writes), retaining GCD, cast-time, caster-aura and power checks. Native category1152 history is not restarted by the flagged cast. No FULL_MASK shortcut.
- [x] Gordoth landing SetHomePosition/SetFacingTo persists an explicit home separately from original spawn data; return honors landing position/facing and respawn restores source spawn-home state.
- [x] Gordoth DoAction1 follows source MoveJump(speed50,min-height4), effect-motion landing/facing, vial DESTROYED2 state/collision and seven researchers' gore/Bloody Suicide→51744 self-deaths without boss credit. [Jump contract](scripted-jump-movement.md).
- [x] Ordinary-player AT7899 entry uses its authored DB2 rotated box/TDB script binding, same-copy Gordoth lookup and source instance-lifetime OnlyOnce state. Failed lookup does not consume the trigger; subsequent entry may retry. Initial placement inside is presence, not walked-in entry.
- [ ] Gordoth repeat12.1s/36.3s callbacks are coded but moving-target proof, GM-mode handling, cage task/animkit/native pose and map389 navigation remain open. Pinned UpdateAI never pumps its scheduled +30ms cage task; no guessed scheduler update.
- [x] Koranthal61412 appearance casts119928, engage removes it/talks, Twisted119300 first6s/repeat7.3s, Storm119971 first20.5s/repeat47.2s and reschedules Twisted to15.7s. Casting blocks event dispatch as source.
- [x] Shadow Storm119973 effect0 dummy casts its actual effect value119965 triggered; destination-only missile effect32 forwards its destination to119964 and deals real damage.
- [x] Koranthal death marks DONE/talks; evade retires without kill credit and respawns after source30s, recasting appearance Vortex.
- [x] Prepare Koranthal primary-binding SQL with `encounter_cpp_bindings.py --encounter koranthal`: pinned source hashes and local Journal695/actor1321/display42101; idempotent output preserves unrelated bindings. No canonical data mutation.
- [x] Adarogg61408 engage schedules Inferno Charge at10s and Fire Breath119420 at20s, both repeating15–20s. Inferno targets a random non-tank threat-list player (`NonTankTargetSelector`), summons server-side119297 Demonic Leap there and channels119405 onto it through its NEARBY_ENTRY condition. Death despawns summons; evade uses `_DespawnAtEvade` and leaves the wild summon to its5s expiry.
- [x] TC `EffectCharge` returns without a unit target, so the Leap's dest-only charge119298 (target46) never casts119299; Inferno Charge deals no damage in pinned source.
- [x] AT7904 (`at_adarogg_intro`) shares AT7899's OnlyOnce DB2-box detector. DoAction makes the nearest StringId houndmasters_1/_2 within70yd yell groups0/1, then Adjules Chow Time120113 at +2s/+1s/+1.2s with the two authored MovePoints and MovePath6140800. Path ends schedule the next path (6148/6996ms) and Eat Trogg120064 at1s, repeating2–3s, twice. Engaging cancels the intro scheduler.
- [ ] Ground MovePath6140800–6140802 needs map389 navigation, so the walked intro cycle is unproved; path-end behaviour is proved by dispatching the source WaypointPathEnded at the authored last node. Unit-target `EFFECT_CHARGE` (Chow Time's charge) is unmodelled; the following MovePoint replaces it in source. Dead houndmasters are not found (DeathState also marks respawn-pending actors).
- [x] Unbound copies: canonical binds only Koranthal on map389, so Gordoth has no boss state there and initializes at its spawn instead of panicking.
- [x] `encounter_cpp_bindings.py --encounter adarogg|gordoth` prepares 61408→1443 bit0 (Journal694/actor1343/display38759) and 61528→1446 bit3 (697/1345/42100) from pinned instance/header hashes.
- [x] Slagmaw61463 engage schedules Lava Spit119434 at1s on a random threat-list unit, repeating1s for five casts; the sixth event casts Magnaw Submerge120384, a random server-side Magnaw Teleport other than the last (initially West: North119424/East119425/South119426/West119428) 3s later, removes Submerge1s after that and resumes Lava Spit1s later. One event per UpdateAI; casting defers events. Teleports use their TDB spell_target_position; a creature target relocates on its own map (`NearTeleportTo`).
- [x] Slagmaw's2.5s boundary check evades (`_DespawnAtEvade`) when `GetVictim()->GetDistance(me)` exceeds50yd: exact distance minus Slagmaw's display42247 CombatReach24 and the player's1.5. Without prepared model geometry the check logs an error and does not evade.
- [x] `encounter_serverside_spells.py` also prepares the four Magnaw teleports; `encounter_model_geometry.py` also prepares display42247.
- [ ] Slagmaw binding: journal actor1344 display38391 differs from TDB model42247, so no binding SQL is prepared. Vehicle2206 accessory61800 (Magnaw's Head) and `ENCOUNTER_FRAME_ENGAGE` are unmodelled. The East teleport lands ~48yd from spawn and stays engaged: dungeon maps skip the flat40yd spawn leash ([contract](creature-aggro.md)); open-world behavior is unchanged.
- [ ] Canonical publication of the Adarogg/Gordoth bindings and server-side/summon/model rows, loot/lockout/native acceptance remain open. Fixtures bind the source actor/bit in an independent map resource; they do not mutate canonical data.

## Deadmines source audit

- [x] TC a352b1fa `Deadmines/deadmines.h` and `instance_deadmines.cpp` identify Glubtok47162, Helix47296, Foe Reaper43778, Ripsnarl47626, Cookie47739 and Vanessa49541. Census retains all11 Normal/Heroic encounter records; Lumbering Oaf47297 is a journal companion, not Helix's primary. These identities do not authorize runtime kill credit.
- [ ] Pinned Deadmines directory has only header/instance files, no boss AI files. Local primary templates have empty AIName/ScriptName and no source0 SmartScripts; authentic combat schedules are unavailable. Do not synthesize timers or import Classic mechanics as Retail boss behavior.
- [ ] TC ordinals0–5 differ from local difficulty-specific DB2 bits (e.g. Glubtok2976/bit8 Normal,2981/bit9 Heroic). Current `raid_encounter_creature` key `(map_id,creature_entry)` stores one bit; no ambiguous multi-difficulty credit binding is published.
- [ ] Factory13965 opens after Glubtok, Mast16400 after Helix, Heavy17153 outside Helix combat and Foundry16399 after Foe Reaper per source. Source-authored door behavior is identified, not implemented or proved. Missing primaries47296/47739/49541 have no TDB placements; no invented spawn coordinates.
- [ ] No Deadmines full-combat, loot/lockout or native acceptance claim. [Census](../encounter-coverage.md).

## Wailing Caverns source slices

- [x] `encounter_cpp_bindings.py --encounter anacondra|cobrahn|kresh|pythas|skum|serpentis|verdan|mutanus` prepares all eight exact local Normal identities (encounters585–592/bits0–7), checking pinned header/instance hashes and source AI identity. Source8 primaries match journal/model rows; no guessed actors or placements. Copy-only SQL preserves unrelated bindings and is idempotent; no canonical mutation. Cobrahn3669→586/bit1 is Journal475/actor1046/display4213.
- [x] Cobrahn Lightning Bolt20805 existing SmartAI row first0ms/repeat6–8s produces real Nature damage after its3s cast plus missile flight (speed24yd/s, minimum5yd unit distance). Fixture pins only the successful branch of its80% chance, retaining the selected source repeat, spell and target. Generic bound BossAi engage/death emits successful encounter586, never585.
- [x] Wailing action34 SetBossState mode1 resolves source ordinal0–7 through the pinned encounter identity to its runtime slot. States0–3 use existing instance lifecycle; missing bindings cannot grant credit. Isolated real Serpentis row data3/DONE3/mode1 changes encounter590 at runtime slot1, not raw slot3 or another copy; generic BossAi death cannot mask this action-level proof.
- [ ] Mode0 SetData, other maps and SPECIAL state4 remain unconverted and diagnosed, never reinterpreted as SetBossState. Anacondra's actual row5 mode0 must not complete Serpentis. Cobrahn source/compatibility ordinal0 differs from DB2 lockout bit1; the earlier generic BossAi lifecycle alone is not source-action proof. Aggro text, other chance outcomes, loot/lockout/native and full-instance acceptance remain open.
- [x] Accepted DONE transitions for all four druid encounter identities dispatch source ACTION_ALL_DONE0 to Disciple3678 in that copy; its partial `npc_disciple_of_naralex` port talks group0. Real source NPC/chat fixture proves three insufficient, fourth notifying, copy isolation and no repeated-DONE duplicate.
- [ ] Four actual boss fights are not this condition proof: Anacondra's sourced mode0 death row remains unconverted and spell2 is absent from local DB2/TDB serverside spells (no substitution). Disciple gossip/escort/awakening/combat, Mutanus spawning, loot/lockout/native gates remain open.
- [ ] Disciple activation data: Mark of the Wild5232 is absent from both local DB2 and TDB serverside tables. Template3678 npcflag0 has no spawn override; source spawn251412 is phase169. TDB menu202 retains its start option, but normalized menu202 options are empty and template gossip_menu_id is0. No fabricated gossip flag/replacement buff; notification fixture uses existing server phase policy, not native phase-visibility proof. No correction of authored SmartScript parameters or replacement with Classic schedules.

## Ruby Life Pools source hooks

Pinned TC `ruby_life_pools.cpp` has four spell scripts and no boss AIs. Local Melidrussa/Kokia/Kyrakka/Erkhart templates have empty AIName/ScriptName; no boss schedules are inferred.

- [x] Ice Shield384933 effect0 periodic dummy refreshes existing372749 duration without reapplying/refilling the shield; missing shield stays missing. Respect local DB2's immediate initial tick, then1s period.
- [x] Excavate372793 effect0 periodic dummy casts373497 triggered from the original caster on the bearer; stops after removal. Respect local DB2's immediate initial tick, then1s period.
- [x] Executed371652 effect0 apply sets source flag fields (FAKE_DEAD flags3, FEIGN_DEATH flags2, PREVENT_EMOTES_FROM_CHAT_TEXT flags1), preserving other bits without a real death. Other effects alone do not run this hook. Source has no cleanup hook; flags persist after aura removal. Native rendering remains unproved.
- [x] Storm Infusion395029 effect1 TARGET_DEST_DEST offsets destination by local(9,0,4). Actual area-trigger creation is tested at two facings; area actions/native visuals remain unproved (local effect has no radius/trigger-spell binding).
- [ ] Source spell-hook gates do not certify SAI path prerequisites, boss combat/credit/loot/lockout or native visuals.

## What it must do

### Framework
- [x] `EventMap`: events run in due order (ties in schedule order); `Repeat` reschedules the last event from now; `Reset` clears it (`EventMap.cpp`).
- [x] A creature with a ported ScriptName gets its AI when it enters the world and only in a map whose `instance_template.script` is its instance script (`GetInstanceAI`); the AI's `InitializeAI` runs then and on respawn (default calls `Reset`; source overrides may relocate instead).
- [x] Hooks: `JustEngagedWith` when the creature first gains threat, `DamageTaken` (health before the hit and the health loss) on melee and spell damage, `JustDied`, `EnterEvadeMode` → `Reset`, `JustSummoned`/`SummonedCreatureDespawn`, `MovementInform(POINT)` at the end of `MovePoint`, `UpdateAI` every tick while alive. A hook raised while the creature's own AI runs is queued behind it.
- [x] `BossAI::UpdateAI`: events advance only while the boss has a victim and execute only while it is not casting. `_JustEngagedWith` sets the boss `IN_PROGRESS`, `_JustDied` `DONE`, `_Reset` `NOT_STARTED` (unless `DONE`), despawning summons.
- [x] `InstanceScript` per dungeon copy: boss states `NOT_STARTED`/`IN_PROGRESS`/`FAIL`/`DONE`, `DONE` never goes back; lives until the copy unloads. `IN_PROGRESS` sends `EncounterStart { encounter_id }`, `DONE` `EncounterEnd { success: true }`, a reset/FAIL from `IN_PROGRESS` `EncounterEnd { success: false }` (Retail SMSG_ENCOUNTER_END; TrinityCore sends nothing) — only to the copy's players.
- [x] Engaging a boss sends `EncounterEngageUnit` (boss frame, priority 1); its reset or death `EncounterDisengageUnit` (Retail; the TrinityCore Stockade scripts do not call `SendEncounterUnit`).
- [x] `Talk(group)`: a not-yet-said text of the group by probability; BroadcastText text (female text for female/genderless speakers when present); Type 12/14/16/41 → `MonsterSay`/`MonsterYell`/`MonsterEmote`/`RaidBossEmote` carrying the speaker; TextRange 3 reaches every player of the speaker's copy, 0 the chat type's range (25/300 yd).
- [x] Evade (`_EnterEvadeMode`): auras removed except `AURA_STAYS_AFTER_COMBAT`, cast cancelled, cooldowns reset, full health; at home mana is full and addon auras return. An evading creature does not evade again.
- [x] `REACT_PASSIVE`: no aggro on sight, no victim selection, attack stopped; back to aggressive resumes attacking the victim.
- [x] Summons (`SummonCreature`, manual despawn): never respawn, go with their corpse or the summoner's `SummonList`; `MovePoint` at walk or run speed.

### Creature spells
- [x] Creature casts: no GCD; no explicit target → the victim when a unit target is needed; triggered casts skip cast time, costs, cooldowns and caster auras; `DoCast(spellId)` targets per `FillAISpellInfo` (self/victim/enemy/debuff).
- [x] A creature's school damage and periodic damage base points are `BasePoints`% of ExpectedStat `CreatureSpellDamage` at its level (`CreatureAutoAttackDps` with `SCALES_WITH_CREATURE_LEVEL`), rounded; weapon % damage uses its ExpectedStat weapon damage.
- [x] Creatures of mana classes (2, 8) have `basemana × ManaModifier` mana; with `UNIT_FLAG2_REGENERATE_POWER` they regain 22 mana / 2 s except while evading.
- [x] A casting creature neither chases (if movement interrupts the cast) nor swings; a stunned unit does not swing; stunned/rooted creatures do not chase and chase speed follows speed auras.
- [x] A creature's melee-class spell misses hostile units by 5% − MOD_HIT_CHANCE.
- [x] Effects/targets added: 27 PERSISTENT_AREA_AURA (dynamic object; enemies inside carry the aura with the object's expiry and tick phase, leavers lose it), 252 TELEPORT_UNITS, targets 28 DEST_DYNOBJ_ENEMY, 65 DEST_TARGET_BACK, 77 UNIT_CHANNEL_TARGET.
- [x] Auras added: 18 MOD_INVISIBILITY (hidden from observers without matching [typed detection](invisibility-detection.md)), 93 MOD_UNATTACKABLE (`NON_ATTACKABLE_2`, attack stopped in a dungeon), 54 MOD_HIT_CHANCE, 252 MOD_SPEED_SLOW_ALL (attack and cast time ×(100−amount)/100).

### SmartAI combat casts
- [x] Appearance/respawn initializes event timers, dispatches source `SMART_EVENT_RESPAWN`11 with authored map/zone conditions, then resets script timers; ordinary Reset/evade does not replay respawn actions (TC SmartAI.cpp494–519). Source cast flags are unchanged.
- [x] `SMART_EVENT_UPDATE_IC` advances initial and repeat min/max timers only while engaged; a due cast waits for any active cast to finish.
- [x] `SMART_TARGET_VICTIM` resolves the current victim. `SMARTCAST_COMBAT_MOVE` disables chase after an accepted cast, including between casts; a rejected cast restores chase/melee and retries on the next update without rerolling event chance.
- [x] New AI starts with combat movement enabled; Reset restores initial timers without changing movement permission. Real Kobold Geomancer 476 rows cast Fireball 20793, with a 0 ms initial delay and 3000–5000 ms repeat.

### Wailing Caverns — bounded Cobrahn slice
- [x] Lord Cobrahn3669 (map43, Normal1, encounter586, instance data0 from local SmartAI death action34) participates in copy-scoped encounter start/end and boss frames through the existing `BossAi` machinery. Evade fails the encounter; death succeeds.
- [x] Its abilities are loaded, not hardcoded: SmartAI row2 casts Lightning Bolt20805 at its victim, initial0ms/repeat6000–8000ms. AGGRO4 / TALK1 speaks source group0.
- [x] Remaining authentically bound WC bosses receive the data-selected lifecycle below.
- [ ] Naralex escort, door progression and unsupported SmartAI events/actions remain gaps. No absent C++ mechanics or coefficients are inferred.
- [ ] Full playable both-skin acceptance remains blocked: native entry/pull/death/credit/frame clearing observed in both skins, but display4213 has no imported appearance and the population copy loses its authored Normal loot binding. Population/gate dependency belongs to `dungeon-population`; no DB writes or fabricated bindings. Normal has no scheduled lockout; existing encounter credit/loot paths are reused.

### Encounter timeline
- [x] SmartAI boss UPDATE_IC cast rows publish their already-selected initial/repeat timer and duration, on changes and at most one second between refreshes, with a UTC sample timestamp so reliable delivery delay cannot restart a selected countdown. Aura/health/linked conditions without a known deadline do not invent a timer. Chance, target, phase and casting conditions remain conditional; scheduling does not pre-roll chance or change AI behavior.
- [x] Script BossAi publishes selected EventMap spell deadlines through each script's literal event→spell binding. Speech/movement events remain absent. Stockade spell mappings reuse the existing constants and schedule; no new boss abilities.
- [x] Accepted `Ctx::do_cast_at` calls for active instance bosses send cast warnings (Low without authored severity); real SET_EVENT_PHASE changes send phase numbers only while that boss's encounter is active. Reset cannot recreate a phase warning after EncounterEnd. All events use the reliable ordered encounter channel, restricted to players in that map copy; reset/end clears native state. Server behavioral fixtures cover Cobrahn randomized selected deadline→real cast, no premature warning, copy isolation/evade and Hogger script deadlines. [Native UI contract and live acceptance](../../../game-engine/docs/specs/encounter-timeline.md) owns rendering/settings/sounds/positioning and PNG proof.

### The Stockade
- [x] Hogger (46254, boss 2, encounter 1144): pull yell; Maddening Call 1-2 s then every 15-20 s; Vicious Slice 3-4 s then every 10-14 s; once below 30%: boss emote "Hogger enrages!" and Enrage (+30% melee haste, +10% physical damage); death yell, Warden Thelwater summoned, runs to (152.0, 106.2) and speaks at +1/+5/+8 s.
- [x] Lord Overheat (46264, boss 1, encounter 1145): pull and death yells; Fireball 2 s then every 2 s; Overheat on a random player 9-11 s then 9-10 s; Rain of Fire at the victim 10-13 s then 15-20 s.
- [x] Randolph Moloch (46383, boss 0, encounter 1146): pull yell; Sweep 2-3 s then 6-7 s; Wildly Stabbing channel 4-5 s then 8-12 s; hits taking him below 71% (from above 59%) and below 41% (from above 29%) each vanish him once 1 s later: boss emote, auras cleared, Vanish, passive; 2 s later attempts native Shadowstep behind a random player and becomes aggressive; native target checks still apply (triggered FULL_MASK does not bypass them). With55964 type4 invisibility, the player needs reciprocal type4 detection flags; insufficient detection amount still keeps Moloch hidden. Without those flags the source attempt fails and does not retry; death yell, Mortimer Moloch walks to him, yells at +4 s and collapses (emote, dies) at +7 s.
- [x] A wipe resets every Stockade boss (deviation: TrinityCore's Hogger and Moloch `Reset` skip `_Reset`).

### Grim Batol
- [x] `instance_grim_batol` (map 670): 4 bosses (General Umbriss 39625, Forgemaster Throngus 40177, Drahga Shadowburner 40319, Erudax 40484; encounters 1051, 1050, 1048, 1049), `LoadObjectData(creatureData)`.
- [x] Deviation: TrinityCore has no Grim Batol creature scripts, so no kill there sets a boss state or locks anyone. Each `creatureData` boss runs a spell-less `BossAI`: engage → `IN_PROGRESS`, wipe → `NOT_STARTED`, death → `DONE` (then the instance lock, docs/specs/instances.md). No boss abilities.
- [x] An instance script created for a copy with an instance lock starts with the lock's `DONE` bosses (`InstanceScript::Load`); encounter messages carry the copy's difficulty.

## Cobrahn behavioral acceptance

`cargo test --locked -p server --bin game-server scripted_ai::` on c7b792a:20 passed, zero failed, one ignored; native build, cargo fmt check and cargo check pass without compiler warnings. Canonical Normal loot roll yields one authored group1 item and87–168 copper, excluding the quest-only gem. Explicit population-copy gate:

```text
DUNGEON_ENCOUNTER_POPULATION_DB=<prepared-copy> cargo test --locked -p server --bin game-server cobrahn_population_copy_normal_loot -- --ignored --nocapture
```

The existing prepared population.sqlite fails this gate with empty items/copper0; this is a real acceptance blocker, not a waived failure. The test opens data read-only. Independent lead review and repaired-copy/live-loot proof remain required; no master merge or deployment claimed.

## Cobrahn native proof — 2026-10-11

Private UDP5700, fresh redb, `fb_dungeon_encounter` characters Cobrmodern/Cobrforever; server integration e77ba78 with population prepared copy. Both skins entered WC through production far transfer/ack, pulled the authentic spawn, received yell/Lightning Bolt20805 and compact boss frame, clicked that frame to target, killed with ordinary Fireball133, cleared the frame and marked Lord Cobrahn complete in the existing dungeon tracker. Setup used level80, periodic admin player-health restoration (not injected boss damage), and an authored Charged Invoker222450 on Forever. This is transport/encounter proof, not normal solo balance.

All13 PNGs under `/syncthing/AgentShared/2026-10-11/dungeon-encounter/` inspected; logs/JSON under engine `data/diagnostics/dungeon-encounter-2026-10-11/`. Modern01 documents void death at the outside portal setup coordinate; physical portal traversal is unproved. The actor/owned spell attachments are absent because display4213 lies outside imported appearance coverage. Loot07/08 and Forever05 explicitly show the failure, not successful looting: prepared `content_creature_template_difficulty`3669/1 has lootid/gold0; raw TDB Normal authors LootID3669/Gold87–168. The population importer owns restoration of that binding. No encounter journal UX added; existing tracker credit and Normal's no-lockout behavior are reused.

## How it works
- `docs/wiki/systems/boss-scripts.md`

## Implementation inventory
- `crates/server/src/scripted_ai/mod.rs` — AI component, hooks, dispatch, `Ctx` (ScriptedAI helpers)
- `scripted_ai/event_map.rs`, `boss_ai.rs`, `instance_script.rs`, `creature_text.rs`, `movement.rs`, `the_stockade.rs`
- `spell_cast/creature.rs` (creature casts, AI targets, spell misses), `spell_cast/area_aura.rs` (dynamic objects), `spell_cast/effects.rs` (ExpectedStat base value, teleport), `spell_cast/targets.rs`
- `auras/unit_state.rs` (invisibility, unattackable), `auras/stat_mods.rs`
- `creature_aggro.rs` (engage/evade hooks, react state), `creature_ai.rs` (chase gating, home), `combat.rs` (swing gating, JustDied, kill, summons' corpses), `zones.rs` (summons, creature mana)
- `scripts/import-creature-text.sh`, `scripts/retail_content.py` `build_creature_text`

## Tests asserting this spec
`cargo test -p server --bin game-server scripted_ai::` (EventMap, texts, all three bosses on real world.db), plus `combat::`, `creature_aggro::`, `spell_cast::`, `auras::`, `maps::` regression suites.

## Known gaps (current cycle)
- [ ] Creature auto-attacks still never miss; dodge/parry/block of creature attacks and magic spell misses are not modelled.
- [ ] Channel targets out of range do not end a channel; `MovePositionToFirstCollision` has no collision.
- [ ] Creature text sounds and emotes (`Sound`, `Emote`, BroadcastText SoundKit) are not played; ranges other than 0/3 are not ported.
- [ ] Player casters' damage base points are not ExpectedStat-scaled.

## Out of scope
- Dungeon Finder (phase 5), doors (the Stockade has none), `FAIL`/`_DespawnAtEvade` (unused by these scripts), Grim Batol boss abilities.

## Live evidence (2026-09-27)
Own server :5085 (world.db = `world-stockadebosses-20260927.db`, copied game.redb), one headless client (Xpbar, level 80 mage; bosses hit with `game-server-admin damage`), evidence in game-engine `data/diagnostics/stockadebosses-20260927/` (`server-run3-hogger.log` for Hogger, `server.log` for the others):
- Hogger: pull yell, EncounterStart 1144 and Boss1TargetFrame (`02`, `ui-02`); Maddening Call and Vicious Slice debuffs on the player (`03`); a hit to 29% gave the center-screen "Hogger enrages!" (`04`); leaving him reset the encounter (EncounterEnd success false, frame hidden, full health, no enrage; `06`); death: "Yiiipe!", EncounterEnd success true, Warden Thelwater ran to (152.0, 106.2) and said all three lines (`07`, `08`, `ui-08`).
- Lord Overheat: pull yell, Fireball chain, Overheat, Rain of Fire's dynamic object at the player, mana 4177/4313 on his boss frame (`11`-`13`); leaving reset him, a new pull and his death gave "FIRE... EXTINGUISHED!" and DONE (`14`).
- Randolph Moloch: pull yell, Sweep and Wildly Stabbing (`20`, `21`); a hit to 70% gave "Randolph Moloch vanishes!", he left the client, Shadowstep brought him back beside the player (`22`, `23`); death yell, DONE, Mortimer walked to him, yelled and collapsed (`24`-`26`).
- Not shown live: the second vanish, Rain of Fire's radius against a second player (unit tests cover both).

## Authentic data registration — 2026-10-11

- Every unambiguous single-creature JournalEncounter/JournalEncounterCreature binding in the imported catalog automatically receives copy-scoped BossAi start/end, boss frames, evade/repull, kill credit and existing lock saving. Registration must not require a per-dungeon Rust module or known ScriptName.
- A bound creature with authored SmartAI rows keeps their timers, spells and targets; a bound creature without rows adds only lifecycle to existing melee. Existing authored Stockade Rust mechanics remain unchanged.
- Stockade, Grim Batol and Cobrahn saved boss ordinals remain compatible. Compatibility bindings are inspectable authored data, not alternate runtime registration code.
- Import preparation includes dungeons and raids, accepts only unique same-map creature/display joins, and records ambiguous/multi-actor gaps without fabricating completion rules. Publication of prepared SQL remains separate from this feature.
- Report per-bound-creature full/partial/none SmartAI coverage. Full means all authored entry-keyed rows are within current event/action/target/flag support, not complete Retail encounter mechanics or spell-effect/numeric parity. None means no authored cast rows; existing Stockade script mechanics are the explicit preservation exception.

Proof: targeted legacy regressions plus real spawn/pull/evade/repull/ability/death/save fixtures; private both-skin Skum3674/encounter589 lifecycle captures at UDP5830. Controlled admin damage exercises production damage/death boundaries, not ordinary solo balance. Skum display4203 remains outside imported appearance coverage; no replacement model or mechanics are introduced.

- Existing BossAi summon ownership hooks are preserved: JustSummoned tracks owned summons; evade/death cleanup despawns them, and SummonedCreatureDespawn removes ownership. This adds no summon ability.
