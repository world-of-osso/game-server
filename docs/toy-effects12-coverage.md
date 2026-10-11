# Toy-effects12 non-summon coverage

Verified: 2026-10-11. **Server-preflight/CPU behavior, not rendered Retail parity.**

## Coverage and ranking

Prepared census **130→134/1166**, **+4**, zero removals. Canonical database untouched; no importer, engine change, live account or UDP listener needed. Implementation `bed4de7` extends the existing complete-zero cosmetic audit; it does not add a blanket aura4/effect3 handler or any of the11 refused control/title categories.

Ranking uses the actual baseline census: exclude item/condition/specialization-unavailable rows and every graph with unsupported effect28. Count remaining items sharing a refusal, not raw DB2 records; counts overlap. All1166 identities remain visible.

|Ranked handler/refusal|Before|After|Action|
|---|---:|---:|---|
|aura4 dummy|35|32|Source-complete sob, ghost and fist cosmetics admitted first; other scripts remain refused.|
|effect3 dummy|31|30|Source-complete Abyss horn cast art admitted next; no script selection invented.|
|effect77 script|16|16|Descriptions require rolls, teleport decisions, appearance selection, returning balls or other actions not defined by a generic script executor.|
|effect50 transmitted objects|11|11|Existing handler; missing templates or specialized interaction semantics, not an absent universal spawner.|
|aura56 transform|8|8|All8 have MiscValue0=0/MiscValue1=8; DB2 supplies no creature identity. No guessed disguise display.|

## Newly usable identities and exact source behavior

|Item / spell|Source contract|Local visual chain|
|---|---|---|
|50471 The Heartbreaker /71909|Friendly target21;60000ms aura. Description: “induces a brief but pitiful display of miserable, wracking sobs.” No damage, stun or forced movement.|visual15196 → animations1684/3924 (LoopAnimID84/77), sound8693, attachment285863 → effect-name3014/model166313.|
|129149 Death's Door Charm /193333|Caster1;1800000ms aura. “Take on your Helheim Spirit form.” Cosmetic ghost-state art, not faction/model replacement or death.|visual32651 → attachment27559 → effect-name24912/model1249985 (`spells/7fx_ghost_blue_state.m2`), effect-type1/4986 (local SpellProceduralEffect Type22), sound104543.|
|205045 B.B.F. Fist /408067|Selected target25;86400000ms aura, exact DB2 duration rather than an invented short gesture timer. “Bump fists with your best brul!” Normal target validation/cooldown/cancellation.|visual128508 → attachment494037 → effect-name20314/model978213 (`creature/fomorhand/fomorhandright.m2`), animations398982/398986 (LoopAnimID121/68), authored sounds.|
|211931 Abyss Caller Horn /430760|Caster1; authentic cast time and SpellGo; no aura duration. “Conjure a body of water.” The complete DB2 payload is cast art, not terrain water, swimming/collision, damage or a summoned entity.|visual137241 → attachments518271–518275/518296/518297, animation403644/403651 (AnimKit13838/19690), sounds261418/261417. Water/globule/ritual models4290516/4290518/4290525/4485229/4397697.|

Each complete spell has a DifficultyID0 visual binding and only the existing admitted zero-base/misc/trigger/item/mechanic dummy shape. No generic inference from a visual binding. The actual local TDB1210 archive has0 selected spell_script_names bindings among4066 parsed rows; this absence alone does not establish cosmetics. Descriptions and complete authored visual chains do.

## Exact retained gaps

The following are **not admitted**. A known runtime refusal is not proof that the action is fundamentally underivable: deferred groups still need deeper source-specific derivation. Underivable selections and missing data are distinguished below. Full baseline/after item→spell→refusal records are retained in the census receipts.

|Items|Exact reason / required behavior|
|---|---|
|1973,33079,35275,53057,54651,54653,115506,249713|Zero-entry aura56 has no source creature/display selection. Do not replace opposing-faction, race/gender, tree or Broker selection with an arbitrary model;115506 also promises combat exit and Draenor-only use.|
|133997,134007|Black Ice203533/ring203820 have no SpellXSpellVisual binding in either local DB2 or SQLite; zero dummy shape does not supply the visible equipment.|
|199902|Compass388275 resolves effect-name41550 Type2/GenericID110576 with positioner1937, not a ModelFileDataID. Existing engine code has no positioner/GenericID consumer; north-pointing behavior is not proved by aura replication.|
|115503|Pendant171352 lava art is derivable, but Draenor-only gate is not closed by the cosmetic predicate; do not silently allow every map.|
|133998|Rainbow203534 art is derivable; description promises reduced Darkspear Dash cooldown. Holiday-dependent reduction is not present in the zero dummy effect payload; no coefficient invented.|
|138873|Frosh217597 includes effect-type1/7382/7383/7385 and visual effect-type7/238; cannot assert complete appearance from ordinary attachment alone.|
|13379|Piccolo182346 is raid-area aura65/dummy4, not an ordinary caster-only cosmetic. Dance propagation/range/recipient semantics remain unimplemented.|
|37254,43499|Simian48332 and Iron Boot58501 mix dummy4 with transforms/triggered or proc effects. Existing whole-spell cosmetic predicate intentionally refuses mixed gameplay chains; no dummy suppression or blanket admission.|
|37460,44820,263871|Pet leashes target90; companion selection/tether behavior is not an ordinary player visual.|
|48933|Northrend67833 dummy aura has no DB2 menu/destination selection; wormhole travel action remains unbound.|
|88377,88417|Paint127803 and shell128150 have nonzero dummy amounts; shell also triggers128151/128152. Tint/stand/movement-cancel contracts need specific derivation, not zero-valued cosmetic admission.|
|88589|Cremate128275 requires fallen-ally corpse action; no corpse-state mutation inferred from fire art.|
|116856,116888,116889,116890,116891|Contender costumes share dummy4/effect3/aura483 descendants; descriptions require special battle abilities and100% healing reduction. Costuming alone cannot close those contracts.|
|119092|Moroes176438 mixes root dummy4 with two dummy descendants130157/176443 and equipment-material changes; no global armor hiding from ordinary attachment replication.|
|163607|Needle279508 triggers279509 transform and279545; witch detection/conditional result not established merely by known transform data.|
|166544|Dark Ranger291148 has effect77 plus dummy4; race/gender appearance selection is not carried by a zero dummy payload.|
|183856|Meditation343141 also needs aura106; levitation semantics not implemented by cosmetic aura4.|
|188680|Chorus100343 has seasonal gating and aura285-triggered effects128/129. Singing art alone is insufficient.|
|134021,192443|Rocket helmets retain launch/jump effect98 chains, not cosmetic-only actions.|
|210974|Eyes427779 reaches dummy aura429097; party-only viewer filtering cannot be inferred from a screen-effect aura.|
|223312|Hat448938 triggers448937 with dummy4 **and item-creation24**; handing another toy item to an ally is not cosmetic-only.|
|224643|Candle453621 needs pet target150 and aura395/cooking behavior, not player attachment-only replication.|
|249468|Training guide1249102 reaches dummy298624 and aura430; Tic-Tac-Toe state/actions are not in a generic zero dummy executor.|
|18986,95567,95568,136849,141605|Gadgetzan, Isle-of-Thunder beacons, Nature's Beacon and Flight Master's Whistle: root effect3 provides no implemented destination-choice/region/flight-master action. Source-specific teleport derivation deferred, not claimed impossible.|
|30690,38578,88385,130102,275683|Construct destruction, corpse flag, critter explosion, ranged skinning and corpse-loot/goo need actual target/gameplay actions. Their script roots are not cosmetic no-ops; unsupported target8/93 remains where present.|
|45984,97921|Unusual Compass64385 and Color-Seein'141930 remain script effect3; random heading/vision selection is not supplied by a generic dummy action.|
|118937,118938|Gamon176179/Manastorm176180 have script-only appearance selection; no arbitrary display substitution.|
|131717|Starlight196535 requires area membership/flight permission and Broken-Isles gate, not just outline art.|
|142528,142529,142530,143662|Bobber replacements retain aura258 and exact summoned-object overrides in census; transmitted fishing replacement/lifecycle not implemented.|
|150743,150746|Scouting maps360598/360599 must reveal maps/flight points; effect3 alone does not perform those persistent changes.|
|191925,201930|Falling Star Flinger44012 requires repeated-use role/actions; H.E.L.P.385602 requires battleground/faction-area flare cooldown. Script roots alone do not define these stateful contracts.|
|35227,36862,36863,44430|Weather machine, two dice and coin toss: effect77 does not define a generic choice/random roll/result-message action. No probabilities, die sides or weather selection invented.|
|43824,110560|Dalaran mastery and Garrison Hearthstone effect77 need destination/context state. Ordinary teleport coordinates alone cannot implement toggle/garrison decisions.|
|44606,52253|Train retains custom gameobject193963 script; music box needs target7/43 and Lamenter behavior. Decoration/audio alone is insufficient.|
|86589,119134|Skymirror needs target appearance copying; Sargerei disguise needs a source-bound appearance choice, not generic effect77.|
|104323,104324,153182|Three shared returning-ball actions; return-to-sender decisions are absent from a generic effect77 executor.|
|119093,191937|Aviana retains launch/flight/parachute chains and Draenor gate; Star Catcher requires repeated-use knockback logic.|
|33219,33927,34480,70722,116757,153004,193032|Existing effect50 refusals: respectively gameobjects186472/186738 type22,187267 type10,208935/236110 type8,277200 type10,377204 type3. Serving/picnic/fire-focus/portal/chest interaction requires its actual runtime.|
|199767,216881,219403|Missing runtime object templates381317/428166/452051. Archive-template/display/script audit and any copy-only importer are deferred; no dummy object substituted or shared DB written.|

## Proof ledger

- `f1b303a` plus tests, before implementation: targeted `cargo test --locked -p server --bin game-server toyfx12 -- --nocapture` **RED5/5**, genuine UseToy aura4/effect3 refusals, baseline130. No compilation/fixture failure accepted as RED.
- `bed4de7`: same targeted command **GREEN5/5**. Actual physical-item learning, UseToy, SpellGo, target-vs-caster ownership/replication, unchanged health/model, exact aura durations, near-expiry existence and production expiry. Virtual clock advances, never patched aura timers. Horn casts without gameplay aura, creature or object. Prepared census134, exact4 added/0 removed.
- Final bounded regression/format/check receipts recorded in the handoff after execution; no unsupported renderer/live acceptance implied.

Durable receipts: server `data/diagnostics/toy-effects12-2026-10-11/` (`visual-source.json`, `model-paths.json`, `archive-script-bindings.json`, `red.log`, `red-census.json`, `green.log`, `after-focused.json`, `delta.json`). [Contract](specs/toys.md), [system](wiki/systems/toys.md#toy-effects12-non-summon-cosmetics-2026-10-11). Canonical database opened read-only; temporary prepared database removed by fixture ownership. No branch merge to master.
