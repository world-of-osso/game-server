# Installed race/class eligibility

Contract: [race/class eligibility](../../specs/race-class-eligibility.md). Source and scoped regression gates verified2026-10-15; live native Dwarf login remains unclaimed.

## Source and lifetime

`race_class.rs` reads `data/db2/active/ChrRaces.csv` and `CharBaseInfo.csv` once per process. Race metadata provides the race-mask bit; supported class memberships populate ordered class sets. Non-playable metadata and unconverted Adventurer14 do not become offered player pairs. The process retains that snapshot until restart. Deploy already ships the entire active DB2 publication; container test inputs also require both CSVs. Cargo tests execute from the crate directory, so test builds anchor the same installed publication to CARGO_MANIFEST_DIR/../../data rather than interpreting the service's working-directory path. Runtime uses only data/db2/active; neither environment probes an alternate source on failure.

The old Retail permission slice is removed. Missing/invalid CSVs, unsupported mask bits, unknown playable-race references, duplicate race metadata and an empty Retail result fail catalog loading. There is no static Retail permission path after source failure.

The public `validate_race_class`, `playable_pairs` and mask helpers keep their existing interfaces. Creation kits and authentic TDB starts already consume those helpers, so newly admitted pairs enter the same source-backed creation path. Start-level and visage policies are unchanged.

## Neutral Pandaren Death Knight creation — 2026-10-16

Installed70391 CharBaseInfo421 lists24/6, and ChrClassRaceSex152/153 provides both sexes with Flags0. ChrRaces24 has PlayableRaceBit23 and Alliance2 (neutral);25/26 have Alliance0/1. These records describe the valid model/class, not a neutral creation start. Retail `Blizzard_CharacterCreate.lua:GetCreateCharacterFaction` (1407–1410) says an earlyFactionChoice class (DK) always needs the selected faction: “the creation process will fail if we try to create a Neutral character of this class”. The server's creation API already receives a race identity, so eligibility admits the chosen25/6 or26/6 identity and rejects raw neutral24/6; it never guesses a faction.

Authentic TDB1210.26091 playercreateinfo has25/6 and26/6 at map2297,481.803,-2124.66,840.857,orientation3.11049, intro scene2394; no24/6 row. Both the source dump and imported table were checked. No start data changed or alternate location was added. Neutral Monk24/10 and Dwarf3/Shaman7 remain offered; Forever95/96 gates are unchanged. Source evidence: neutral-dk-source-proof.json beneath the durable bot-harness evidence root.

Master bfe7d7a6's exact existing all-pairs creation test reproduced RED with “race24 class6 has no authored playercreateinfo start” before implementation (neutral-dk-allpairs-red.log). Creation memberships now apply the explicit neutral-DK policy using installed ChrRaces.Alliance. Exact all-pairs creation plus source policy/missing-source controls gate the fix; runtime measurements are a separate follow-up.

## Separate Forever identity

The [Skyborne95/96 contract](skyborne.md#race-identity-and-creation) remains explicit and source-pinned: ten authored pairs and bits32/33. Installed Retail names those IDs as NPC rows and provides no CharBaseInfo memberships. Empty Retail race entries are removed before the separate Forever entries are added; a playable Retail collision fails rather than overwriting source identity. This is not an alternative lookup after a missing Retail CSV. Raw Forever ChrRaces/CharBaseInfo graph bytes were unavailable in this checkout; no new decode or metadata-completeness claim follows.

## Observed failure and proof boundary

The private bot driver received `Class 7 is not available for race 3` for normal Dwarf/Shaman creation while installed12.1.5.70391 CharBaseInfo explicitly contains3/7. The prior static Dwarf list also omitted installed Mage8 and Warlock9. That real admin rejection is retained in engine `data/diagnostics/bot-harness-2026-10-15/native-source-gear1/263/setup-error.json`; the root-cause change does not alter class stats or bypass creation.

The installed snapshot contains283 distinct supported Retail model memberships, of which282 are creatable race/class identities after the neutral-DK faction policy below. New regressions check those actual rows and actual Dwarf/Shaman admin creation, source starting location/equipment and saved character data. Existing mask, class rejection, start-level and Forever tests remain controls. Targeted installed/missing-source/admin creation tests pass3/3. After master integration, race eligibility/mask/start-level/Forever controls pass10/10, scoped server check and workspace format check pass. The real creation test loads the authentic kit, invokes the admin creation handler and reopens saved character/inventory records; it does not establish a rendered Dwarf login. Evidence: race-source-targeted-green.log, race-source-controls.log, race-source-check.log and race-source-fmt.log beneath the bot harness durable evidence root.

The [bot fixture](../../../../game-engine/docs/wiki/systems/bot-harness.md#native-players-and-source-inputs) separately authors Draenei11 for Shaman before provisioning. That choice does not establish Dwarf eligibility or replace this server fix. The full class/rotation and all-pairs intro/asset gates remain separate.
