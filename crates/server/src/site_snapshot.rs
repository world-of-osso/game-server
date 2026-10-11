use redb::{ReadOnlyDatabase, ReadTransaction, ReadableDatabase, ReadableTable, TableDefinition};
use rusqlite::{Connection, OpenFlags, OptionalExtension};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
type Bundle = BTreeMap<String, Vec<u8>>;
#[path = "site_snapshot_catalog.rs"]
mod catalog;
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
    let bundle = compose(&options, &txn, &world, &selected)?;
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
    let diagnostics =
        std::fs::canonicalize("/home/osso/Projects/world-of-osso/game-engine/data/diagnostics")?;
    if !source.starts_with(diagnostics) {
        return Err("fixture source must be a private diagnostics redb".into());
    }
    let db = ReadOnlyDatabase::open(source)?;
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
fn compose(
    options: &Options,
    txn: &ReadTransaction,
    world: &Connection,
    selected: &BTreeSet<u64>,
) -> Result<Bundle> {
    let mut bundle = Bundle::new();
    let mut summaries = Vec::new();
    let mut guilds: BTreeMap<String, Value> = BTreeMap::new();
    let mut rankings = Vec::new();
    let locks = store::locks(txn)?;
    for &id in selected {
        let data = store::bytes(txn, "characters", id)?
            .ok_or_else(|| format!("public character {id} missing"))?;
        let character = crate::character_data::CharacterData::decode_compatible(&data)?;
        let mut gaps = vec![
            "race_names_and_faction_catalog_unavailable".into(),
            "complete_effective_equipment_item_level_unavailable".into(),
            "character_render_unavailable".into(),
            "mythic_history_has_no_participant_attribution".into(),
            "raid_first_kill_and_lifetime_history_unavailable".into(),
        ];
        let guild: Option<store::Guild> = store::decode(txn, "character_guilds", id)?;
        let guild_value = guild
            .as_ref()
            .map(|g| json!({"id":g.guild_id.to_string(),"name":g.guild_name}));
        let progression: Option<store::Progression> =
            store::decode(txn, "character_progression", id)?;
        let spec = progression
            .as_ref()
            .map(|p| catalog::label(world, "chr_specialization", "Name_lang", p.spec_id))
            .transpose()?;
        let mut search = vec![character.name.to_lowercase()];
        if let Some(g) = &guild {
            search.push(g.guild_name.to_lowercase());
        }
        let summary = json!({"id":id.to_string(),"name":character.name,"race":{"id":character.race,"name":null},"class":catalog::label(world,"chr_classes","Name_lang",u32::from(character.class))?,"spec":spec,"level":character.level,"faction":null,"guild":guild_value,"item_level":null,"search_keys":search,"url":format!("characters/{id}.json")});
        let mut page = summary.clone();
        page["equipped_items"] =
            equipment(options, txn, world, id, &character, &mut bundle, &mut gaps)?;
        page["talents"] = talents(txn, id, progression.as_ref())?;
        let (points, recent) = achievements(options, txn, world, id, &mut bundle, &mut gaps)?;
        page["achievement_points"] = points;
        page["recent_achievements"] = recent;
        page["mythic_plus_best"] = json!([]);
        page["render_url"] = Value::Null;
        page["pvp"] = pvp(txn, id)?;
        let mut raid_progression = Vec::new();
        if let Some(saved) = locks.locks.get(&id) {
            for lock in saved
                .values()
                .filter(|lock| !lock.is_new && (lock.expiry > options.as_of || lock.extended))
            {
                if let Some(mut raid) = catalog::raid(world, lock)? {
                    if raid["kills"].as_u64().unwrap_or(0) > 0 {
                        raid["character_id"] = json!(id.to_string());
                        rankings.push(raid.clone());
                    }
                    raid_progression.push(raid);
                }
            }
        }
        raid_progression.sort_by_key(|r| (r["raid"]["id"].as_u64(), r["difficulty_id"].as_u64()));
        page["raid_progression"] = json!(raid_progression);
        add_json(
            &mut bundle,
            format!("characters/{id}.json"),
            options.document(page, gaps),
        )?;
        if let Some(g) = guild {
            let entry=guilds.entry(g.guild_id.to_string()).or_insert_with(||json!({"id":g.guild_id.to_string(),"name":g.guild_name,"roster":[],"raid_progression":[]}));
            entry["roster"]
                .as_array_mut()
                .ok_or("invalid guild roster")?
                .push(summary.clone());
        }
        summaries.push(summary);
    }
    let mut guild_index = Vec::new();
    for (id, guild) in guilds {
        guild_index.push(json!({"id":id,"name":guild["name"],"url":format!("guilds/{id}.json")}));
        add_json(
            &mut bundle,
            format!("guilds/{id}.json"),
            options.document(guild, vec!["guild_at_kill_attribution_unavailable".into()]),
        )?;
    }
    rank_raids(&mut rankings);
    add_json(
        &mut bundle,
        "index.json".into(),
        options.document(
            json!({"realm_name":options.realm_name,"characters":summaries,"guilds":guild_index}),
            vec!["public_membership_and_opt_outs_operator_supplied".into()],
        ),
    )?;
    add_json(&mut bundle,"rankings/raids.json".into(),options.document(json!({"policy":"kills_per_difficulty_then_first_kill","scope":"saved_lock","characters":rankings,"guilds":[]}),vec!["raid_first_kill_and_lifetime_history_unavailable".into(),"guild_at_kill_attribution_unavailable".into()]))?;
    validate_mythic_history(txn)?;
    add_json(
        &mut bundle,
        "rankings/mythic-plus.json".into(),
        options.document(
            json!({"policy":"highest_timed_key_then_clear_time","runs":[]}),
            vec![
                "mythic_history_has_no_participant_attribution".into(),
                "mythic_history_has_no_season_or_completion_timestamp".into(),
            ],
        ),
    )?;
    Ok(bundle)
}
fn validate_mythic_history(txn: &ReadTransaction) -> Result<()> {
    let table = match txn.open_table(TableDefinition::<u64, &[u8]>::new("mythic_run_history")) {
        Ok(t) => t,
        Err(redb::TableError::TableDoesNotExist(_)) => return Ok(()),
        Err(e) => return Err(e.into()),
    };
    for row in table.iter()? {
        let (_, bytes) = row?;
        let _: shared::protocol::ChallengeRun = bitcode::decode(bytes.value())?;
    }
    Ok(())
}
fn rank_raids(rows: &mut [Value]) {
    rows.sort_by_key(|r| {
        (
            r["raid"]["id"].as_u64(),
            r["difficulty_id"].as_u64(),
            std::cmp::Reverse(r["kills"].as_u64()),
            r["character_id"]
                .as_str()
                .and_then(|s| s.parse::<u64>().ok()),
        )
    });
    let mut ranks = BTreeMap::new();
    for row in rows {
        let rank = ranks
            .entry((row["raid"]["id"].as_u64(), row["difficulty_id"].as_u64()))
            .or_insert(0);
        *rank += 1;
        row["rank"] = json!(rank);
    }
}
fn equipment(
    options: &Options,
    txn: &ReadTransaction,
    world: &Connection,
    id: u64,
    character: &crate::character_data::CharacterData,
    bundle: &mut Bundle,
    gaps: &mut Vec<String>,
) -> Result<Value> {
    let owned: Vec<store::Equipped> =
        store::decode(txn, "character_equipment", id)?.unwrap_or_default();
    let source_table =
        match txn.open_table(TableDefinition::<u64, u8>::new("item_definition_sources")) {
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
        let (level, fdid) = catalog::item(world, equipped.item.item_id, source)?;
        let icon_url = catalog::icon(bundle, &options.textures, fdid)?;
        if icon_url.is_none() {
            gaps.push(format!("item_{}_icon_unavailable", equipped.item.item_id));
        }
        let appearance = character
            .equipment_appearance
            .entries
            .iter()
            .find(|e| e.slot == equipped.slot);
        let transmog: Option<store::Transmog> =
            store::decode(txn, "item_transmogs", equipped.item.item_guid)?;
        items.push(json!({"slot":format!("{:?}",equipped.slot),"item_id":equipped.item.item_id,"definition_source":source,"name":equipped.item.name,"quality":equipped.item.quality,"item_level":level,"icon_fdid":fdid,"icon_url":icon_url,"appearance":{"display_info_id":appearance.and_then(|e|e.display_info_id),"hidden":appearance.is_some_and(|e|e.hidden)},"transmog":transmog.map(|t|json!({"appearance_id":t.source_id,"display_info_id":t.display_id,"hidden":t.hidden}))}));
    }
    items.sort_by_key(|item| item["slot"].as_str().unwrap_or("").to_owned());
    Ok(json!(items))
}
fn talents(
    txn: &ReadTransaction,
    id: u64,
    progression: Option<&store::Progression>,
) -> Result<Value> {
    let Some(progression) = progression else {
        return Ok(Value::Null);
    };
    let Some(traits) = store::decode::<store::Traits>(txn, "character_trait_loadouts_v1", id)?
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
    let mut loadouts =
        vec![json!({"id":0,"name":null,"selected":spec.selected_id==0,"purchases":spec.default})];
    loadouts.extend(spec.configs.into_iter().map(|c|json!({"id":c.id,"name":c.name,"selected":c.id==spec.selected_id,"purchases":c.purchases})));
    loadouts.sort_by_key(|c| c["id"].as_u64());
    Ok(json!({"spec_id":spec.spec_id,"selected_loadout_id":spec.selected_id,"loadouts":loadouts}))
}
fn pvp(txn: &ReadTransaction, id: u64) -> Result<Value> {
    let Some(saved) = store::json(txn, "pvp_states", id)? else {
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
fn achievements(
    options: &Options,
    txn: &ReadTransaction,
    world: &Connection,
    id: u64,
    bundle: &mut Bundle,
    gaps: &mut Vec<String>,
) -> Result<(Value, Value)> {
    let Some(saved) = store::json(txn, "achievements", id)? else {
        return Ok((Value::Null, json!([])));
    };
    let earned = saved["earned_ids"]
        .as_array()
        .ok_or("achievement earned_ids must be an array")?;
    let mut total = Some(0_u64);
    let mut recent = Vec::new();
    for id in earned {
        let id = id.as_u64().ok_or("achievement id must be integer")?;
        let metadata: Option<(String, u32, u32)> = world
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
        recent.push(json!({"id":id,"name":name,"points":points,"earned_at":earned_at,"icon_fdid":fdid,"icon_url":catalog::icon(bundle,&options.textures,Some(fdid).filter(|id|*id!=0))?}));
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
