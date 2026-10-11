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
    public: PathBuf,
    opt_outs: PathBuf,
    textures: PathBuf,
    output: PathBuf,
    realm: String,
    realm_name: String,
    as_of: i64,
}
impl Options {
    fn parse(args: Vec<String>) -> Result<Self> {
        if args.len() != 18 {
            return Err("usage: game-server-site-export --redb COPY --world-db WORLD --public-characters IDS.json --opt-outs IDS.json --textures CACHE --output NEW_DIR --realm SLUG --realm-name NAME --as-of UNIX_SECONDS".into());
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
        let as_of = get("--as-of")?.parse()?;
        if as_of < 0 {
            return Err("as-of must be nonnegative Unix seconds".into());
        }
        Ok(Self {
            redb: get("--redb")?.into(),
            world: get("--world-db")?.into(),
            public: get("--public-characters")?.into(),
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
    if args.first().map(String::as_str) == Some("--fixture-allowlist") {
        return fixture_allowlist(&args);
    }
    let options = Options::parse(args)?;
    if options.output.exists() {
        return Err(format!(
            "output {} already exists; publish a fresh snapshot",
            options.output.display()
        )
        .into());
    }
    let public = ids(&options.public)?;
    let opted_out = ids(&options.opt_outs)?;
    let selected: BTreeSet<_> = public.difference(&opted_out).copied().collect();
    let db = ReadOnlyDatabase::open(&options.redb)?;
    let txn = db.begin_read()?;
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
fn fixture_allowlist(args: &[String]) -> Result<()> {
    if args.len() != 3 {
        return Err("usage: --fixture-allowlist PRIVATE_DIAGNOSTICS_REDB NEW_IDS_JSON".into());
    }
    let source = std::fs::canonicalize(&args[1])?;
    let diagnostics = std::fs::canonicalize(
        "/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/site-data-2026-10-11",
    )?;
    if !source.starts_with(diagnostics) {
        return Err(
            "fixture allowlist requires an owned site-data diagnostic copy, never the source redb"
                .into(),
        );
    }
    // Explicit private-copy preparation: recover a crash-closed proof copy before read-only export.
    let db = redb::Database::open(source)?;
    let txn = db.begin_read()?;
    let accounts = txn.open_table(TableDefinition::<&str, u64>::new("username_accounts"))?;
    let mut selected = BTreeSet::new();
    for row in accounts.iter()? {
        let (name, id) = row?;
        if name.value().starts_with("fb_") {
            let characters: Vec<u64> =
                store::decode(&txn, "account_characters", id.value())?.unwrap_or_default();
            selected.extend(characters);
        }
    }
    let bytes =
        serde_json::to_vec_pretty(&selected.iter().map(u64::to_string).collect::<Vec<_>>())?;
    use std::io::Write;
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args[2])?
        .write_all(&bytes)?;
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
        let lifetime = raids::read_rankings(self.txn, self.world, selected, self.options.as_of)?;
        let mythic = mythic::read_rankings(self.txn, self.world, selected, self.options.as_of)?;
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
            "race_names_and_faction_catalog_unavailable".into(),
            "complete_effective_equipment_item_level_unavailable".into(),
            "character_render_unavailable".into(),
            "legacy_mythic_runs_not_attributable_or_backfilled".into(),
            "pre_deploy_raid_history_not_backfilled".into(),
        ];
        page["equipped_items"] = self.read_equipment(id, &character, &mut gaps)?;
        page["talents"] = self.read_talents(id, progression.as_ref())?;
        let (points, recent) = self.read_achievements(id, &mut gaps)?;
        page["achievement_points"] = points;
        page["recent_achievements"] = recent;
        page["mythic_plus_best"] = json!(mythic::character_best(mythic, id));
        page["render_url"] = Value::Null;
        page["pvp"] = self.read_pvp(id)?;
        let saved_progression = query_saved_progression(self.world, locks, id, self.options.as_of)?;
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
    ) -> Result<Value> {
        let owned: Vec<store::Equipped> =
            store::decode(self.txn, "character_equipment", id)?.unwrap_or_default();
        let source_table = match self
            .txn
            .open_table(TableDefinition::<u64, u8>::new("item_definition_sources"))
        {
            Ok(t) => Some(t),
            Err(redb::TableError::TableDoesNotExist(_)) => None,
            Err(e) => return Err(e.into()),
        };
        let mut items = Vec::new();
        for equipped in owned {
            let source = source_table
                .as_ref()
                .map(|t| t.get(equipped.item.item_guid))
                .transpose()?
                .flatten()
                .map(|r| r.value())
                .unwrap_or(0);
            let (level, fdid) = catalog::item(self.world, equipped.item.item_id, source)?;
            let icon_url = catalog::icon(&mut self.bundle, &self.options.textures, fdid)?;
            if icon_url.is_none() {
                gaps.push(format!("item_{}_icon_unavailable", equipped.item.item_id));
            }
            let appearance = character
                .equipment_appearance
                .entries
                .iter()
                .find(|e| e.slot == equipped.slot);
            let transmog: Option<store::Transmog> =
                store::decode(self.txn, "item_transmogs", equipped.item.item_guid)?;
            items.push(json!({"slot":format!("{:?}",equipped.slot),"item_id":equipped.item.item_id,"definition_source":source,"name":equipped.item.name,"quality":equipped.item.quality,"item_level":level,"icon_fdid":fdid,"icon_url":icon_url,"appearance":{"display_info_id":appearance.and_then(|e|e.display_info_id),"hidden":appearance.is_some_and(|e|e.hidden)},"transmog":transmog.map(|t|json!({"appearance_id":t.source_id,"display_info_id":t.display_id,"hidden":t.hidden}))}));
        }
        items.sort_by_key(|item| item["slot"].as_str().unwrap_or("").to_owned());
        Ok(json!(items))
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
        let mut value = json!({});
        for key in ["honor", "honor_max", "conquest", "conquest_max", "brackets"] {
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
            let earned_at = saved["earned_at"]
                .get(id.to_string())
                .cloned()
                .unwrap_or(Value::Null);
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
    let guild_value = guild.map(|g| json!({"id":g.guild_id.to_string(),"name":g.guild_name}));
    let mut search = vec![character.name.to_lowercase()];
    if let Some(guild) = guild {
        search.push(guild.guild_name.to_lowercase());
    }
    Ok(
        json!({"id":id.to_string(),"name":character.name,"race":{"id":character.race,"name":null},"class":catalog::label(world,"chr_classes","Name_lang",u32::from(character.class))?,"spec":spec,"level":character.level,"faction":null,"guild":guild_value,"item_level":null,"search_keys":search,"url":format!("characters/{id}.json")}),
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
        std::fs::write(root.join("public.json"), "[\"1\",\"2\"]").unwrap();
        std::fs::write(root.join("opt-outs.json"), "[\"2\"]").unwrap();
        std::fs::create_dir(root.join("textures")).unwrap();
    }
    fn arguments(root: &Path, output: &str) -> Vec<String> {
        [
            "--redb",
            root.join("private.redb").to_str().unwrap(),
            "--world-db",
            root.join("world.db").to_str().unwrap(),
            "--public-characters",
            root.join("public.json").to_str().unwrap(),
            "--opt-outs",
            root.join("opt-outs.json").to_str().unwrap(),
            "--realm",
            "test",
            "--realm-name",
            "Private Proof",
            "--as-of",
            "100",
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
    fn site_snapshot_public_allowlist_opt_out_and_determinism() {
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
        assert_eq!(index["characters"].as_array().unwrap().len(), 1);
        assert_eq!(index["characters"][0]["id"], "1");
        assert_eq!(
            index["characters"][0]["search_keys"],
            serde_json::json!(["fbalpha"])
        );
        assert!(!dir.path().join("a/characters/2.json").exists());
        assert!(!dir.path().join("a/characters/3.json").exists());
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
    }
    #[test]
    fn site_snapshot_raid_lifetime_timestamp_ranks_and_guild_privacy() {
        let dir = tempfile::tempdir().unwrap();
        seed(dir.path());
        std::fs::write(dir.path().join("public.json"), "[\"1\",\"2\",\"3\"]").unwrap();
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
                        .map(|id| server::raid_history_store::Participant {
                            character_id: id.to_owned(),
                            guild_id: None,
                            guild_name: None,
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
                    completed_at: 99,
                    party: party
                        .into_iter()
                        .map(|id| server::mythic_plus_history::Participant {
                            character_id: id.into(),
                            name: format!("Fb{id}"),
                            class_id: 1,
                            spec_id: None,
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
        let page: Value = serde_json::from_slice(
            &std::fs::read(dir.path().join("ranks/characters/1.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(page["mythic_plus_best"].as_array().unwrap().len(), 1);
        assert_eq!(page["mythic_plus_best"][0]["id"], "11");
        assert!(page["mythic_plus_best"][0].get("rating").is_none());
    }
    #[test]
    fn site_snapshot_rejects_missing_public_character_and_existing_output() {
        let dir = tempfile::tempdir().unwrap();
        seed(dir.path());
        std::fs::write(dir.path().join("public.json"), "[\"999\"]").unwrap();
        assert!(
            run(arguments(dir.path(), "a"))
                .unwrap_err()
                .to_string()
                .contains("999")
        );
        assert!(!dir.path().join("a").exists());
        std::fs::write(dir.path().join("public.json"), "[\"1\"]").unwrap();
        run(arguments(dir.path(), "a")).unwrap();
        assert!(
            run(arguments(dir.path(), "a"))
                .unwrap_err()
                .to_string()
                .contains("exists")
        );
    }
}
