use redb::{ReadOnlyDatabase, ReadTransaction, ReadableDatabase, ReadableTable, TableDefinition};
use rusqlite::{Connection, OpenFlags, OptionalExtension};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
type Bundle = BTreeMap<String, Vec<u8>>;
#[path = "site_snapshot_catalog.rs"]
mod catalog;
#[path = "site_snapshot_mythic.rs"]
mod mythic;
#[path = "site_snapshot_raids.rs"]
mod raids;
#[path = "site_snapshot_store.rs"]
mod store;

struct Options {
    redb: PathBuf,
    world: PathBuf,
    opt_outs: PathBuf,
    textures: PathBuf,
    output: PathBuf,
    realm: String,
    realm_name: String,
    as_of: i64,
}
impl Options {
    fn parse(args: Vec<String>, as_of: i64) -> Result<Self> {
        if args.len() != 14 {
            return Err("usage: game-server-site-export --redb COPY --world-db WORLD --opt-outs IDS.json --textures CACHE --output NEW_DIR --realm SLUG --realm-name NAME; repair: --repair-copy SOURCE NEW_PRIVATE_COPY".into());
        }
        let mut fields = BTreeMap::new();
        for pair in args.chunks_exact(2) {
            if fields.insert(pair[0].as_str(), pair[1].as_str()).is_some() {
                return Err(format!("duplicate option {}", pair[0]).into());
            }
        }
        let get = |key| {
            fields
                .get(key)
                .copied()
                .ok_or_else(|| format!("missing option {key}"))
        };
        let realm = get("--realm")?.to_owned();
        if realm.is_empty()
            || !realm
                .bytes()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
        {
            return Err("realm must be a lowercase ASCII slug".into());
        }
        Ok(Self {
            redb: get("--redb")?.into(),
            world: get("--world-db")?.into(),
            opt_outs: get("--opt-outs")?.into(),
            textures: get("--textures")?.into(),
            output: get("--output")?.into(),
            realm,
            realm_name: get("--realm-name")?.into(),
            as_of,
        })
    }
    fn document(&self, mut value: Value, gaps: Vec<String>) -> Value {
        value["schema_version"] = json!(1);
        value["realm"] = json!(self.realm);
        value["as_of"] = json!(self.as_of);
        value["gaps"] = json!(gaps);
        value
    }
}
fn ids(path: &Path) -> Result<BTreeSet<u64>> {
    let values: Vec<String> = serde_json::from_slice(&std::fs::read(path)?)?;
    values
        .into_iter()
        .map(|s| s.parse().map_err(Into::into))
        .collect()
}
fn add_json(bundle: &mut Bundle, path: String, value: Value) -> Result<()> {
    let mut bytes = serde_json::to_vec_pretty(&value)?;
    bytes.push(b'\n');
    bundle.insert(path, bytes);
    Ok(())
}
pub fn run(args: Vec<String>) -> Result<()> {
    if args.first().map(String::as_str) == Some("--repair-copy") {
        return repair_private_copy(&args);
    }
    let elapsed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?;
    run_at(args, i64::try_from(elapsed.as_secs())?)
}
fn run_at(args: Vec<String>, as_of: i64) -> Result<()> {
    let options = Options::parse(args, as_of)?;
    if options.output.exists() {
        return Err(format!(
            "output {} already exists; publish a fresh snapshot",
            options.output.display()
        )
        .into());
    }
    let opted_out = ids(&options.opt_outs)?;
    let db = ReadOnlyDatabase::open(&options.redb)?;
    let txn = db.begin_read()?;
    let characters = txn.open_table(TableDefinition::<u64, &[u8]>::new("characters"))?;
    let mut selected = BTreeSet::new();
    for row in characters.iter()? {
        let (id, _) = row?;
        if !opted_out.contains(&id.value()) {
            selected.insert(id.value());
        }
    }
    let world = Connection::open_with_flags(&options.world, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    world.execute_batch("BEGIN")?;
    let bundle = SnapshotReader {
        options: &options,
        txn: &txn,
        world: &world,
        bundle: Bundle::new(),
    }
    .read_snapshot(&selected)?;
    // All reads/decodes/conversions finish before any destination mutation.
    std::fs::create_dir(&options.output)?;
    for (relative, bytes) in bundle {
        let target = options.output.join(relative);
        std::fs::create_dir_all(target.parent().ok_or("output has no parent")?)?;
        std::fs::write(target, bytes)?;
    }
    Ok(())
}
fn repair_private_copy(args: &[String]) -> Result<()> {
    if args.len() != 3 {
        return Err(
            "usage: --repair-copy SOURCE NEW_PRIVATE_COPY (source is never repaired)".into(),
        );
    }
    let mut source = std::fs::File::open(&args[1])?;
    // Exclusive creation rejects source aliases, symlinks and existing destinations.
    let mut destination = std::fs::OpenOptions::new();
    destination.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        destination.mode(0o600);
    }
    let mut copy = destination.open(&args[2])?;
    std::io::copy(&mut source, &mut copy)?;
    copy.sync_all()?;
    drop(copy);
    // Writable recovery is explicit and only ever opens the newly created private copy.
    drop(redb::Database::open(&args[2])?);
    eprintln!("repaired private copy {} (source unchanged)", args[2]);
    Ok(())
}
struct SnapshotReader<'a> {
    options: &'a Options,
    txn: &'a ReadTransaction,
    world: &'a Connection,
    bundle: Bundle,
}
struct CharacterIndex {
    summary: Value,
    guild: Option<store::Guild>,
    saved_progression: Vec<Value>,
}
impl SnapshotReader<'_> {
    fn read_snapshot(mut self, selected: &BTreeSet<u64>) -> Result<Bundle> {
        let lifetime = raids::read_rankings(
            self.txn,
            self.world,
            selected,
            self.options.as_of,
            &self.options.realm,
        )?;
        let mythic = mythic::read_rankings(
            self.txn,
            self.world,
            selected,
            self.options.as_of,
            &self.options.realm,
        )?;
        let locks = store::locks(self.txn)?;
        let mut guilds = history_guilds(&lifetime.guilds)?;
        let mut summaries = Vec::new();
        let mut saved = Vec::new();
        for &id in selected {
            let character = self.read_character(id, &locks, &lifetime.characters, &mythic)?;
            append_guild_roster(&mut guilds, character.guild, &character.summary)?;
            summaries.push(character.summary);
            saved.extend(character.saved_progression);
        }
        let guild_index = self.write_guilds(guilds, &lifetime.guilds)?;
        self.write_rankings(lifetime, mythic, saved)?;
        add_json(&mut self.bundle,"index.json".into(),self.options.document(json!({"realm_name":self.options.realm_name,"characters":summaries,"guilds":guild_index}),vec!["public_membership_and_opt_outs_operator_supplied".into()]))?;
        Ok(self.bundle)
    }
    fn read_character(
        &mut self,
        id: u64,
        locks: &store::Locks,
        lifetime: &[Value],
        mythic: &[Value],
    ) -> Result<CharacterIndex> {
        let data = store::bytes(self.txn, "characters", id)?
            .ok_or_else(|| format!("public character {id} missing"))?;
        let character = crate::character_data::CharacterData::decode_compatible(&data)?;
        let guild = store::decode::<store::Guild>(self.txn, "character_guilds", id)?;
        let progression =
            store::decode::<store::Progression>(self.txn, "character_progression", id)?;
        let summary = query_character_summary(
            self.world,
            id,
            &character,
            guild.as_ref(),
            progression.as_ref(),
        )?;
        let mut page = summary.clone();
        let mut gaps = vec![
            "complete_effective_equipment_item_level_unavailable".into(),
            "character_render_unavailable".into(),
            "legacy_mythic_runs_not_attributable_or_backfilled".into(),
            "pre_deploy_raid_history_not_backfilled".into(),
        ];
        if summary["race"]["name"].is_null() || summary["faction"].is_null() {
            gaps.push("race_names_and_faction_catalog_unavailable".into());
        }
        let (items, slots) = self.read_equipment(id, &character, &mut gaps)?;
        page["equipped_items"] = items;
        page["equipment_slots"] = slots;
        page["talents"] = self.read_talents(id, progression.as_ref())?;
        let (points, recent) = self.read_achievements(id, &mut gaps)?;
        page["achievement_points"] = points;
        page["recent_achievements"] = recent;
        page["mythic_plus_best"] = json!(mythic::character_best(mythic, id));
        page["render_url"] = Value::Null;
        page["pvp"] = self.read_pvp(id)?;
        if !page["pvp"].is_null() {
            gaps.push("pvp_dynamic_conquest_cap_unavailable".into());
            gaps.push("pvp_rating_policy_not_source_qualified".into());
        }
        let mut saved_progression =
            query_saved_progression(self.world, locks, id, self.options.as_of)?;
        for row in &mut saved_progression {
            row["name"] = json!(character.name);
            row["realm"] = json!(self.options.realm);
        }
        page["saved_lock_progression"] = json!(saved_progression);
        page["raid_progression"] = json!(
            lifetime
                .iter()
                .filter(|r| r["character_id"] == id.to_string())
                .collect::<Vec<_>>()
        );
        add_json(
            &mut self.bundle,
            format!("characters/{id}.json"),
            self.options.document(page, gaps),
        )?;
        Ok(CharacterIndex {
            summary,
            guild,
            saved_progression,
        })
    }
    fn write_guilds(
        &mut self,
        guilds: BTreeMap<String, Value>,
        progression: &[Value],
    ) -> Result<Vec<Value>> {
        let mut index = Vec::new();
        for (id, mut guild) in guilds {
            guild["raid_progression"] = json!(
                progression
                    .iter()
                    .filter(|r| r["guild_id"] == id)
                    .collect::<Vec<_>>()
            );
            index.push(json!({"id":id,"name":guild["name"],"url":format!("guilds/{id}.json")}));
            add_json(
                &mut self.bundle,
                format!("guilds/{id}.json"),
                self.options
                    .document(guild, vec!["pre_deploy_raid_history_not_backfilled".into()]),
            )?;
        }
        Ok(index)
    }
    fn write_rankings(
        &mut self,
        lifetime: raids::Rankings,
        mythic: Vec<Value>,
        mut saved: Vec<Value>,
    ) -> Result<()> {
        saved.retain(|r| r["kills"].as_u64().is_some_and(|n| n > 0));
        saved.sort_by_key(|r| {
            (
                r["character_id"].as_str().map(str::to_owned),
                r["raid"]["id"].as_u64(),
                r["difficulty_id"].as_u64(),
            )
        });
        add_json(&mut self.bundle,"rankings/raids.json".into(),self.options.document(json!({"policy":"kills_per_difficulty_then_first_kill","scope":"post_deploy_lifetime","characters":lifetime.characters,"guilds":lifetime.guilds,"saved_lock_characters":saved}),vec!["pre_deploy_raid_history_not_backfilled".into()]))?;
        add_json(
            &mut self.bundle,
            "rankings/mythic-plus.json".into(),
            self.options.document(
                json!({"policy":"highest_timed_key_then_clear_time","runs":mythic}),
                vec![
                    "legacy_mythic_runs_not_attributable_or_backfilled".into(),
                    "active_mythic_season_unavailable".into(),
                ],
            ),
        )?;
        Ok(())
    }
    fn read_equipment(
        &mut self,
        id: u64,
        character: &crate::character_data::CharacterData,
        gaps: &mut Vec<String>,
    ) -> Result<(Value, Value)> {
        let owned: Vec<store::Equipped> =
            store::decode(self.txn, "character_equipment", id)?.unwrap_or_default();
        let bags: Vec<store::EquippedBag> =
            store::decode(self.txn, "character_bag_slots", id)?.unwrap_or_default();
        let mut equipped: Vec<_> = owned
            .into_iter()
            .map(|entry| (format!("{:?}", entry.slot), entry.item))
            .collect();
        for bag in bags {
            let slot = match bag.bag {
                1..=4 => format!("Bag{}", bag.bag),
                5 => "ReagentBag".into(),
                _ => return Err(format!("invalid equipped bag slot {}", bag.bag).into()),
            };
            equipped.push((slot, bag.item));
        }
        gaps.push("neck_finger_trinket_ranged_storage_unavailable".into());
        gaps.push("effective_item_level_modifiers_not_persisted".into());
        let source_table = match self
            .txn
            .open_table(TableDefinition::<u64, u8>::new("item_definition_sources"))
        {
            Ok(t) => Some(t),
            Err(redb::TableError::TableDoesNotExist(_)) => None,
            Err(e) => return Err(e.into()),
        };
        let mut items = Vec::new();
        for (slot, item) in equipped {
            let source = source_table
                .as_ref()
                .map(|t| t.get(item.item_guid))
                .transpose()?
                .flatten()
                .map(|r| r.value())
                .unwrap_or(0);
            let (level, fdid) = catalog::item(self.world, item.item_id, source)?;
            let icon_url = catalog::icon(&mut self.bundle, &self.options.textures, fdid)?;
            if icon_url.is_none() {
                gaps.push(format!("item_{}_icon_unavailable", item.item_id));
            }
            let appearance = character
                .equipment_appearance
                .entries
                .iter()
                .find(|e| format!("{:?}", e.slot) == slot);
            let transmog: Option<store::Transmog> =
                store::decode(self.txn, "item_transmogs", item.item_guid)?;
            items.push(json!({"slot":slot,"item_id":item.item_id,"definition_source":source,"name":item.name,"quality":item.quality,"item_level":level,"item_level_kind":"authored_base","effective_item_level":null,"icon_fdid":fdid,"icon_url":icon_url,"appearance":{"display_info_id":appearance.and_then(|e|e.display_info_id),"hidden":appearance.is_some_and(|e|e.hidden)},"transmog":transmog.map(|t|json!({"appearance_id":t.source_id,"display_info_id":t.display_id,"hidden":t.hidden}))}));
        }
        items.sort_by_key(|item| item["slot"].as_str().unwrap_or("").to_owned());
        let slots = equipment_slot_states(&items);
        Ok((json!(items), json!(slots)))
    }
    fn read_talents(&self, id: u64, progression: Option<&store::Progression>) -> Result<Value> {
        let Some(progression) = progression else {
            return Ok(Value::Null);
        };
        let Some(traits) =
            store::decode::<store::Traits>(self.txn, "character_trait_loadouts_v1", id)?
        else {
            return Ok(Value::Null);
        };
        let Some(spec) = traits
            .specs
            .into_iter()
            .find(|s| s.spec_id == progression.spec_id)
        else {
            return Ok(Value::Null);
        };
        let mut loadouts = vec![
            json!({"id":0,"name":null,"selected":spec.selected_id==0,"purchases":spec.default}),
        ];
        loadouts.extend(spec.configs.into_iter().map(|c|json!({"id":c.id,"name":c.name,"selected":c.id==spec.selected_id,"purchases":c.purchases})));
        loadouts.sort_by_key(|c| c["id"].as_u64());
        Ok(
            json!({"spec_id":spec.spec_id,"selected_loadout_id":spec.selected_id,"loadouts":loadouts}),
        )
    }
    fn read_pvp(&self, id: u64) -> Result<Value> {
        let Some(saved) = store::json(self.txn, "pvp_states", id)? else {
            return Ok(Value::Null);
        };
        let mut value = json!({"conquest_max":null});
        for key in ["honor", "honor_max", "conquest", "brackets"] {
            value[key] = saved
                .get(key)
                .ok_or_else(|| format!("pvp state missing {key}"))?
                .clone();
        }
        // Explicit field allowlist, including each bracket; never serialize future private fields.
        let brackets = value["brackets"]
            .as_array_mut()
            .ok_or("pvp brackets must be an array")?;
        for bracket in brackets {
            let mut clean = json!({});
            for key in [
                "bracket",
                "rating",
                "season_wins",
                "season_losses",
                "weekly_wins",
                "weekly_losses",
            ] {
                clean[key] = bracket
                    .get(key)
                    .ok_or_else(|| format!("pvp bracket missing {key}"))?
                    .clone();
            }
            let wins = clean["season_wins"].as_u64().ok_or("invalid season wins")?;
            let losses = clean["season_losses"]
                .as_u64()
                .ok_or("invalid season losses")?;
            let games = wins
                .checked_add(losses)
                .ok_or("season game count overflow")?;
            clean["games_played"] = json!(games);
            clean["state"] = json!(if games == 0 { "unrated" } else { "rated" });
            if games == 0 {
                clean["rating"] = Value::Null;
            }
            *bracket = clean;
        }
        Ok(value)
    }
    fn read_achievements(&mut self, id: u64, gaps: &mut Vec<String>) -> Result<(Value, Value)> {
        let Some(saved) = store::json(self.txn, "achievements", id)? else {
            return Ok((Value::Null, json!([])));
        };
        let earned = saved["earned_ids"]
            .as_array()
            .ok_or("achievement earned_ids must be an array")?;
        let mut total = Some(0_u64);
        let mut recent = Vec::new();
        for id in earned {
            let id = id.as_u64().ok_or("achievement id must be integer")?;
            let earned_at = saved["earned_at"]
                .get(id.to_string())
                .cloned()
                .unwrap_or(Value::Null);
            if earned_at
                .as_i64()
                .is_some_and(|timestamp| timestamp > self.options.as_of)
            {
                continue;
            }
            let metadata: Option<(String, u32, u32)> = self
                .world
                .query_row(
                    "SELECT Title_lang,Points,IconFileID FROM achievement WHERE ID=?1",
                    [id],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                )
                .optional()?;
            let Some((name, points, fdid)) = metadata else {
                total = None;
                gaps.push(format!("achievement_{id}_catalog_unavailable"));
                continue;
            };
            total = total.map(|sum| sum + u64::from(points));
            recent.push(json!({"id":id,"name":name,"points":points,"earned_at":earned_at,"icon_fdid":fdid,"icon_url":catalog::icon(&mut self.bundle,&self.options.textures,Some(fdid).filter(|id|*id!=0))?}));
        }
        recent.sort_by_key(|e| {
            (
                std::cmp::Reverse(e["earned_at"].as_i64().unwrap_or(i64::MIN)),
                e["id"].as_u64(),
            )
        });
        recent.truncate(10);
        Ok((json!(total), json!(recent)))
    }
}
fn equipment_slot_states(items: &[Value]) -> Vec<Value> {
    use shared::protocol::EquipmentSlot;
    let paperdoll = EquipmentSlot::ALL.iter().map(|slot| format!("{slot:?}"));
    let bags = ["Bag1", "Bag2", "Bag3", "Bag4", "ReagentBag"]
        .into_iter()
        .map(str::to_owned);
    paperdoll
        .chain(bags)
        .map(|slot| {
            let item = items.iter().find(|item| item["slot"] == slot);
            let unsupported = matches!(
                slot.as_str(),
                "Neck" | "Finger1" | "Finger2" | "Trinket1" | "Trinket2" | "Ranged"
            );
            let state = if item.is_some() {
                "equipped"
            } else if unsupported {
                "unavailable"
            } else {
                "empty"
            };
            json!({"slot":slot,"state":state,"item_id":item.map(|i|&i["item_id"])})
        })
        .collect()
}
fn history_guilds(progression: &[Value]) -> Result<BTreeMap<String, Value>> {
    let mut guilds = BTreeMap::new();
    for row in progression {
        let id = row["guild_id"]
            .as_str()
            .ok_or("guild history missing identity")?;
        guilds.entry(id.to_owned()).or_insert_with(
            || json!({"id":id,"name":row["guild_name"],"roster":[],"raid_progression":[]}),
        );
    }
    Ok(guilds)
}
fn append_guild_roster(
    guilds: &mut BTreeMap<String, Value>,
    guild: Option<store::Guild>,
    summary: &Value,
) -> Result<()> {
    let Some(guild) = guild else {
        return Ok(());
    };
    let id = guild.guild_id.to_string();
    let entry = guilds.entry(id.clone()).or_insert_with(
        || json!({"id":id,"name":guild.guild_name,"roster":[],"raid_progression":[]}),
    );
    entry["name"] = json!(guild.guild_name);
    entry["roster"]
        .as_array_mut()
        .ok_or("invalid guild roster")?
        .push(summary.clone());
    Ok(())
}
fn query_character_summary(
    world: &Connection,
    id: u64,
    character: &crate::character_data::CharacterData,
    guild: Option<&store::Guild>,
    progression: Option<&store::Progression>,
) -> Result<Value> {
    let spec = progression
        .map(|p| catalog::label(world, "chr_specialization", "Name_lang", p.spec_id))
        .transpose()?;
    let guild_value = guild.map_or_else(
        || json!({"state":"none"}),
        |g| json!({"state":"member","id":g.guild_id.to_string(),"name":g.guild_name}),
    );
    let mut search = vec![character.name.to_lowercase()];
    if let Some(guild) = guild {
        search.push(guild.guild_name.to_lowercase());
    }
    let (race, faction) = catalog::race(world, character.race)?;
    Ok(
        json!({"id":id.to_string(),"name":character.name,"race":race,"class":catalog::label(world,"chr_classes","Name_lang",u32::from(character.class))?,"spec":spec,"level":character.level,"faction":faction,"guild":guild_value,"item_level":null,"search_keys":search,"url":format!("characters/{id}.json")}),
    )
}
fn query_saved_progression(
    world: &Connection,
    locks: &store::Locks,
    id: u64,
    as_of: i64,
) -> Result<Vec<Value>> {
    let Some(saved) = locks.locks.get(&id) else {
        return Ok(Vec::new());
    };
    let mut progression = Vec::new();
    for lock in saved
        .values()
        .filter(|lock| !lock.is_new && (lock.expiry > as_of || lock.extended))
    {
        if let Some(mut raid) = catalog::raid(world, lock)? {
            raid["character_id"] = json!(id.to_string());
            progression.push(raid);
        }
    }
    progression.sort_by_key(|r| (r["raid"]["id"].as_u64(), r["difficulty_id"].as_u64()));
    Ok(progression)
}
#[cfg(test)]
mod tests {
    use super::*;
    include!("site_snapshot_blp_qualification.rs");
    fn run(args: Vec<String>) -> Result<()> {
        super::run_at(args, 100)
    }
    use crate::character_data::CharacterData;
    use redb::{Database, TableDefinition};
    use serde_json::Value;
    use shared::components::{CharacterAppearance, EquipmentAppearance};

    const CHARACTERS: TableDefinition<u64, &[u8]> = TableDefinition::new("characters");

    fn character(name: &str) -> CharacterData {
        CharacterData {
            name: name.into(),
            x: 1.0,
            y: 2.0,
            z: 3.0,
            health: 100.0,
            max_health: 100.0,
            race: 1,
            class: 1,
            level: 80,
            appearance: CharacterAppearance::default(),
            equipment_appearance: EquipmentAppearance::default(),
            durability: vec![],
        }
    }

    fn seed(root: &Path) {
        let db = Database::create(root.join("private.redb")).unwrap();
        let txn = db.begin_write().unwrap();
        {
            let mut table = txn.open_table(CHARACTERS).unwrap();
            for (id, name) in [(1, "FbAlpha"), (2, "FbHidden"), (3, "FbOtherRealm")] {
                table
                    .insert(id, character(name).encode_stored().as_slice())
                    .unwrap();
            }
        }
        txn.commit().unwrap();
        let world = rusqlite::Connection::open(root.join("world.db")).unwrap();
        world.execute_batch("CREATE TABLE chr_classes(ID INTEGER,Name_lang TEXT); INSERT INTO chr_classes VALUES(1,'Warrior'); CREATE TABLE chr_specialization(ID INTEGER,Name_lang TEXT); CREATE TABLE item_sparse(ID INTEGER,ItemLevel INTEGER); CREATE TABLE item(ID INTEGER,IconFileDataID INTEGER); CREATE TABLE achievement(ID INTEGER,Title_lang TEXT,Points INTEGER,IconFileID INTEGER); CREATE TABLE map(ID INTEGER,MapName_lang TEXT,InstanceType INTEGER); CREATE TABLE dungeon_encounter(ID INTEGER,Name_lang TEXT,MapID INTEGER,DifficultyID INTEGER,Bit INTEGER);").unwrap();
        std::fs::write(root.join("opt-outs.json"), "[\"2\"]").unwrap();
        std::fs::create_dir(root.join("textures")).unwrap();
    }
    fn arguments(root: &Path, output: &str) -> Vec<String> {
        [
            "--redb",
            root.join("private.redb").to_str().unwrap(),
            "--world-db",
            root.join("world.db").to_str().unwrap(),
            "--opt-outs",
            root.join("opt-outs.json").to_str().unwrap(),
            "--realm",
            "test",
            "--realm-name",
            "Private Proof",
            "--textures",
            root.join("textures").to_str().unwrap(),
            "--output",
            root.join(output).to_str().unwrap(),
        ]
        .into_iter()
        .map(str::to_owned)
        .collect()
    }
    #[test]
    fn site_snapshot_all_characters_minus_opt_out_and_determinism() {
        let dir = tempfile::tempdir().unwrap();
        seed(dir.path());
        let before = std::fs::read(dir.path().join("private.redb")).unwrap();
        run(arguments(dir.path(), "a")).unwrap();
        run(arguments(dir.path(), "b")).unwrap();
        assert_eq!(
            before,
            std::fs::read(dir.path().join("private.redb")).unwrap()
        );
        let index: Value =
            serde_json::from_slice(&std::fs::read(dir.path().join("a/index.json")).unwrap())
                .unwrap();
        assert_eq!(index["characters"].as_array().unwrap().len(), 2);
        assert_eq!(index["characters"][1]["id"], "3");
        assert_eq!(index["characters"][0]["id"], "1");
        assert_eq!(
            index["characters"][0]["search_keys"],
            serde_json::json!(["fbalpha"])
        );
        assert!(!dir.path().join("a/characters/2.json").exists());
        assert!(dir.path().join("a/characters/3.json").exists());
        let bytes = std::fs::read(dir.path().join("a/characters/1.json")).unwrap();
        assert_eq!(
            bytes,
            std::fs::read(dir.path().join("b/characters/1.json")).unwrap()
        );
        let page: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(page["class"]["name"], "Warrior");
        assert!(page["pvp"].is_null());
        assert!(page["item_level"].is_null());
        for forbidden in ["account_id", "email", "password", "x", "y", "z"] {
            assert!(page.get(forbidden).is_none());
        }
        std::fs::write(dir.path().join("opt-outs.json"), "[]").unwrap();
        run(arguments(dir.path(), "all")).unwrap();
        let all: Value =
            serde_json::from_slice(&std::fs::read(dir.path().join("all/index.json")).unwrap())
                .unwrap();
        assert_eq!(all["characters"].as_array().unwrap().len(), 3);
    }
    #[test]
    #[ignore = "subprocess fixture: exits without closing redb"]
    fn site_snapshot_crash_writer() {
        let path = std::env::var("SITE_SNAPSHOT_CRASH_REDB").unwrap();
        let db = Database::create(path).unwrap();
        let txn = db.begin_write().unwrap();
        txn.open_table(CHARACTERS)
            .unwrap()
            .insert(1, character("CrashProof").encode_stored().as_slice())
            .unwrap();
        txn.commit().unwrap();
        std::process::exit(0);
    }

    #[test]
    fn site_snapshot_repairs_only_new_private_copy_after_process_crash() {
        let dir = tempfile::tempdir().unwrap();
        seed(dir.path());
        let source = dir.path().join("crashed.redb");
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "site_snapshot::tests::site_snapshot_crash_writer",
                "--ignored",
            ])
            .env("SITE_SNAPSHOT_CRASH_REDB", &source)
            .status()
            .unwrap();
        assert!(status.success());
        assert!(ReadOnlyDatabase::open(&source).is_err());
        let before = std::fs::read(&source).unwrap();
        let copy = dir.path().join("repaired.redb");
        let repair_args = vec![
            "--repair-copy".into(),
            source.to_str().unwrap().into(),
            copy.to_str().unwrap().into(),
        ];
        super::run(repair_args.clone()).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&copy).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        assert_eq!(before, std::fs::read(&source).unwrap());
        assert!(super::run(repair_args).is_err());
        assert!(
            super::run(vec![
                "--repair-copy".into(),
                source.to_str().unwrap().into(),
                source.to_str().unwrap().into()
            ])
            .is_err()
        );
        let db = ReadOnlyDatabase::open(&copy).unwrap();
        assert!(
            db.begin_read()
                .unwrap()
                .open_table(CHARACTERS)
                .unwrap()
                .get(1)
                .unwrap()
                .is_some()
        );
        drop(db);
        let mut args = arguments(dir.path(), "recovered");
        args[1] = copy.to_str().unwrap().into();
        run(args).unwrap();
        assert!(dir.path().join("recovered/characters/1.json").exists());
        assert_eq!(before, std::fs::read(&source).unwrap());
    }

    #[test]
    fn site_snapshot_uses_actual_export_clock() {
        let dir = tempfile::tempdir().unwrap();
        seed(dir.path());
        let before = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        super::run(arguments(dir.path(), "clock")).unwrap();
        let after = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let index: Value =
            serde_json::from_slice(&std::fs::read(dir.path().join("clock/index.json")).unwrap())
                .unwrap();
        let page: Value = serde_json::from_slice(
            &std::fs::read(dir.path().join("clock/characters/1.json")).unwrap(),
        )
        .unwrap();
        let as_of = index["as_of"].as_u64().unwrap();
        assert!(as_of >= before && as_of <= after);
        assert_eq!(page["as_of"], index["as_of"]);
    }
    #[test]
    fn site_snapshot_contract_slots_guild_race_pvp_and_future_achievements() {
        let dir = tempfile::tempdir().unwrap();
        seed(dir.path());
        let world = rusqlite::Connection::open(dir.path().join("world.db")).unwrap();
        world.execute_batch("CREATE TABLE chr_races(ID INTEGER,Name_lang TEXT,FactionID INTEGER); INSERT INTO chr_races VALUES(1,'Human',1); CREATE TABLE faction_template(ID INTEGER,FactionGroup INTEGER); INSERT INTO faction_template VALUES(1,3); INSERT INTO achievement VALUES(6,'Future Kill',10,0),(7,'Past Kill',20,0);").unwrap();
        let db = Database::open(dir.path().join("private.redb")).unwrap();
        let txn = db.begin_write().unwrap();
        {
            txn.open_table(TableDefinition::<u64, &[u8]>::new("achievements"))
                .unwrap()
                .insert(
                    1,
                    serde_json::to_vec(&json!({"earned_ids":[6,7],"earned_at":{"6":101,"7":99}}))
                        .unwrap()
                        .as_slice(),
                )
                .unwrap();
            txn.open_table(TableDefinition::<u64,&[u8]>::new("pvp_states")).unwrap().insert(1,serde_json::to_vec(&json!({"honor":0,"honor_max":15000,"conquest":0,"conquest_max":1800,"brackets":[{"bracket":"2v2","rating":1500,"season_wins":0,"season_losses":0,"weekly_wins":0,"weekly_losses":0}]})).unwrap().as_slice()).unwrap();
        }
        txn.commit().unwrap();
        drop(db);
        run(arguments(dir.path(), "contract")).unwrap();
        let page: Value = serde_json::from_slice(
            &std::fs::read(dir.path().join("contract/characters/1.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(page["guild"], json!({"state":"none"}));
        assert_eq!(page["race"]["name"], "Human");
        assert_eq!(page["faction"]["name"], "Alliance");
        assert_eq!(page["equipment_slots"].as_array().unwrap().len(), 24);
        assert_eq!(
            page["equipment_slots"]
                .as_array()
                .unwrap()
                .iter()
                .find(|r| r["slot"] == "Neck")
                .unwrap()["state"],
            "unavailable"
        );
        assert!(page["pvp"]["conquest_max"].is_null());
        assert!(page["pvp"]["brackets"][0]["rating"].is_null());
        assert_eq!(page["pvp"]["brackets"][0]["games_played"], 0);
        assert!(
            page["gaps"]
                .as_array()
                .unwrap()
                .contains(&json!("pvp_dynamic_conquest_cap_unavailable"))
        );
        assert_eq!(page["achievement_points"], 20);
        assert_eq!(page["recent_achievements"][0]["name"], "Past Kill");
        assert_eq!(page["recent_achievements"].as_array().unwrap().len(), 1);
    }
    #[test]
    fn site_snapshot_raid_lifetime_timestamp_ranks_and_guild_privacy() {
        let dir = tempfile::tempdir().unwrap();
        seed(dir.path());
        let world = rusqlite::Connection::open(dir.path().join("world.db")).unwrap();
        world.execute_batch("INSERT INTO map VALUES(409,'Molten Core',2); INSERT INTO dungeon_encounter VALUES(663,'Lucifron',409,0,0),(664,'Magmadar',409,0,1);").unwrap();
        let db = Database::open(dir.path().join("private.redb")).unwrap();
        let txn = db.begin_write().unwrap();
        {
            let mut table = txn
                .open_table(server::raid_history_store::FIRST_KILLS)
                .unwrap();
            for (subject, id, encounter, time, participants) in [
                ("character", "1", 663, 50, vec!["1", "3"]),
                ("character", "1", 664, 80, vec!["1", "3"]),
                ("character", "3", 663, 10, vec!["1", "3"]),
                ("character", "3", 664, 70, vec!["1", "3"]),
                ("character", "2", 663, 1, vec!["2"]),
                ("guild", "7", 663, 40, vec!["1", "3"]),
                ("guild", "8", 663, 20, vec!["1", "2"]),
            ] {
                let row = server::raid_history_store::FirstKill {
                    character_id: (subject == "character").then(|| id.into()),
                    guild_id: (subject == "guild").then(|| id.into()),
                    guild_name: (subject == "guild").then(|| "Proof Guild".into()),
                    map_id: 409,
                    difficulty_id: 9,
                    encounter_id: encounter,
                    first_kill_at: time,
                    participants: participants
                        .into_iter()
                        .map(|member_id| server::raid_history_store::Participant {
                            character_id: member_id.to_owned(),
                            guild_id: (subject == "guild").then(|| id.to_owned()),
                            guild_name: (subject == "guild").then(|| "Proof Guild".into()),
                        })
                        .collect(),
                    guild_group_credit: subject == "guild",
                };
                let key = format!("{subject}:{id}:409:9:{encounter}");
                table
                    .insert(key.as_str(), serde_json::to_vec(&row).unwrap().as_slice())
                    .unwrap();
            }
        }
        txn.commit().unwrap();
        drop(db);
        run(arguments(dir.path(), "raid")).unwrap();
        let ranks: Value = serde_json::from_slice(
            &std::fs::read(dir.path().join("raid/rankings/raids.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(ranks["characters"].as_array().unwrap().len(), 2);
        assert_eq!(ranks["characters"][0]["character_id"], "3");
        assert_eq!(ranks["characters"][0]["kills"], 2);
        assert_eq!(ranks["characters"][0]["total"], 2);
        assert_eq!(ranks["characters"][0]["progression_at"], 70);
        assert_eq!(ranks["guilds"].as_array().unwrap().len(), 1);
        assert_eq!(ranks["guilds"][0]["guild_id"], "7");
        assert_eq!(ranks["guilds"][0]["guild_group_credit"], true);
        assert!(!dir.path().join("raid/guilds/8.json").exists());
        let page: Value = serde_json::from_slice(
            &std::fs::read(dir.path().join("raid/characters/1.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(page["raid_progression"][0]["kills"], 2);
        assert_eq!(page["raid_progression"][0]["scope"], "post_deploy_lifetime");
        assert!(
            page["saved_lock_progression"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }
    #[test]
    fn site_snapshot_mythic_ranking_best_and_private_party_filter() {
        let dir = tempfile::tempdir().unwrap();
        seed(dir.path());
        let world = rusqlite::Connection::open(dir.path().join("world.db")).unwrap();
        world.execute_batch("ALTER TABLE chr_specialization ADD COLUMN Role INTEGER; INSERT INTO chr_specialization VALUES(73,'Protection',0); CREATE TABLE keystone_affix(ID INTEGER,Name_lang TEXT); INSERT INTO keystone_affix VALUES(9,'Tyrannical');").unwrap();
        let db = Database::open(dir.path().join("private.redb")).unwrap();
        let txn = db.begin_write().unwrap();
        {
            let mut table = txn
                .open_table(server::mythic_plus_history::COMPLETED_RUNS)
                .unwrap();
            for (id, level, elapsed, party) in [
                (10, 12, 90, vec!["1"]),
                (11, 12, 80, vec!["1"]),
                (12, 13, 110, vec!["1"]),
                (13, 20, 50, vec!["1", "2"]),
                (14, 30, 40, vec!["1"]),
            ] {
                let record = server::mythic_plus_history::CompletedRun {
                    id: id.to_string(),
                    season_id: None,
                    challenge_id: 401,
                    map_id: 2515,
                    name: "The Azure Vault".into(),
                    level,
                    affixes: vec![9],
                    elapsed_ms: elapsed,
                    timer_ms: 100,
                    time_lost_ms: 0,
                    in_time: elapsed <= 100,
                    completed_at: if id == 14 { 101 } else { 99 },
                    party: party
                        .into_iter()
                        .map(|id| server::mythic_plus_history::Participant {
                            character_id: id.into(),
                            name: format!("Fb{id}"),
                            class_id: 1,
                            spec_id: Some(73),
                        })
                        .collect(),
                };
                table
                    .insert(id, serde_json::to_vec(&record).unwrap().as_slice())
                    .unwrap();
            }
        }
        txn.commit().unwrap();
        drop(db);
        run(arguments(dir.path(), "ranks")).unwrap();
        let ranks: Value = serde_json::from_slice(
            &std::fs::read(dir.path().join("ranks/rankings/mythic-plus.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            ranks["runs"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| r["id"].as_str().unwrap())
                .collect::<Vec<_>>(),
            vec!["11", "10", "12"]
        );
        assert_eq!(ranks["runs"][0]["rank"], 1);
        assert_eq!(ranks["runs"][2]["in_time"], false);
        assert!(ranks["runs"][0]["season_id"].is_null());
        assert_eq!(ranks["runs"][0]["party"][0]["realm"], "test");
        assert_eq!(ranks["runs"][0]["party"][0]["class_name"], "Warrior");
        assert_eq!(ranks["runs"][0]["party"][0]["spec_name"], "Protection");
        assert_eq!(ranks["runs"][0]["party"][0]["role"], "tank");
        assert_eq!(ranks["runs"][0]["affixes"][0]["name"], "Tyrannical");
        assert_eq!(ranks["runs"][0]["dungeon"]["map_id"], 2515);
        assert!(ranks["runs"][0].get("map_id").is_none());
        let page: Value = serde_json::from_slice(
            &std::fs::read(dir.path().join("ranks/characters/1.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(page["mythic_plus_best"].as_array().unwrap().len(), 1);
        assert_eq!(page["mythic_plus_best"][0]["id"], "11");
        assert!(page["mythic_plus_best"][0].get("rating").is_none());
    }
    #[test]
    fn site_snapshot_rejects_invalid_opt_outs_and_existing_output() {
        let dir = tempfile::tempdir().unwrap();
        seed(dir.path());
        std::fs::write(dir.path().join("opt-outs.json"), "[\"not-an-id\"]").unwrap();
        assert!(run(arguments(dir.path(), "a")).is_err());
        assert!(!dir.path().join("a").exists());
        std::fs::write(dir.path().join("opt-outs.json"), "[\"2\",\"999\"]").unwrap();
        run(arguments(dir.path(), "a")).unwrap();
        assert!(
            run(arguments(dir.path(), "a"))
                .unwrap_err()
                .to_string()
                .contains("exists")
        );
    }
}
