//! The eye table: the eye point of every creature display and playable character model, per stand state (`crate::eye`), and the character display of each playable race
//! and sex.

use std::collections::HashMap;
use std::path::Path;

use glam::Vec3;

use super::chunks::Result;
use super::m2::{GEOSET_GROUP_EYES, M2File, Skeleton};
use super::primary_skin;
use crate::eye::{EyeRow, EyeSource, STAND_EYE_STATES, STAND_STATE_ANIMS};

#[derive(Default, Debug)]
pub struct EyeStats {
    /// Displays with rows.
    pub displays: usize,
    /// Displays by eye source.
    pub from_eyes: usize,
    pub from_head: usize,
    /// Displays whose model has neither eye geometry nor a head bone: the bounding box
    /// top.
    pub from_bounds: usize,
    /// (display, stand state) pairs whose model has no sequence for the state: they hold
    /// the Stand eye.
    pub stand_pose: usize,
    pub rows: usize,
    /// Optional displays without rows, and why.
    pub skipped: Vec<(u32, String)>,
    /// Required displays that could not be baked.
    pub gaps: Vec<(u32, String)>,
}

/// One model's eyes: where they come from and the eye point per stand state; `None` when
/// the state's `.anim` file is not extracted.
struct ModelEyes {
    source: EyeSource,
    eyes: Vec<(u8, Option<Vec3>)>,
    /// States whose sequence the model lacks.
    stand_pose: usize,
}

struct Display {
    model_fdid: u32,
    scale: f32,
}

/// Rows for the displays spawned creatures show and the `ChrModel.csv` character
/// displays, which must all bake, and for every other creature template display
/// (summons and transforms), skipped with why when it cannot; read from `data`
/// (game-engine `data/`).
pub fn eye_rows(data: &Path, world_db: &Path) -> Result<(Vec<EyeRow>, EyeStats)> {
    let required = required_display_ids(data, world_db)?;
    let optional = template_display_ids(world_db)?;
    eye_rows_with_gaps(data, world_db, &required, &optional)
}

/// Rows for `required` (an error when any cannot bake) and `optional` displays.
pub fn eye_rows_of(
    data: &Path,
    world_db: &Path,
    required: &[u32],
    optional: &[u32],
) -> Result<(Vec<EyeRow>, EyeStats)> {
    let (rows, stats) = eye_rows_with_gaps(data, world_db, required, optional)?;
    if !stats.gaps.is_empty() {
        return Err(format!("eye table incomplete: {:?}", stats.gaps));
    }
    Ok((rows, stats))
}

/// Bake resolvable rows and report each required display that has no rows.
pub fn eye_rows_with_gaps(
    data: &Path,
    world_db: &Path,
    required: &[u32],
    optional: &[u32],
) -> Result<(Vec<EyeRow>, EyeStats)> {
    let mut ids: Vec<(u32, bool)> = required.iter().map(|&id| (id, true)).collect();
    ids.extend(
        optional
            .iter()
            .filter(|id| !required.contains(id))
            .map(|&id| (id, false)),
    );
    ids.sort_unstable();
    ids.dedup_by_key(|(id, _)| *id);
    let displays = displays(world_db, ids.iter().map(|&(id, _)| id))?;
    let mut stats = EyeStats::default();
    let mut models: HashMap<u32, Result<ModelEyes>> = HashMap::new();
    let mut rows = Vec::new();
    for (id, needed) in ids {
        let display_rows = displays
            .get(&id)
            .ok_or_else(|| "no CreatureDisplayInfo row with a model".to_string())
            .and_then(|display| {
                let model = models
                    .entry(display.model_fdid)
                    .or_insert_with(|| extracted_model_eyes(data, display.model_fdid));
                let model = model.as_ref().map_err(|err| err.clone())?;
                rows_of(id, display, model)
            });
        match display_rows {
            Ok((display_rows, model_source, stand_pose)) => {
                match model_source {
                    EyeSource::Eyes => stats.from_eyes += 1,
                    EyeSource::Head => stats.from_head += 1,
                    EyeSource::Bounds => stats.from_bounds += 1,
                }
                stats.stand_pose += stand_pose;
                stats.displays += 1;
                rows.extend(display_rows);
            }
            Err(why) if needed => stats.gaps.push((id, why)),
            Err(why) => stats.skipped.push((id, why)),
        }
    }
    stats.rows = rows.len();
    Ok((rows, stats))
}

/// Model `fdid`'s eyes; an error naming the model when it is not extracted or cannot
/// be read.
fn extracted_model_eyes(data: &Path, fdid: u32) -> Result<ModelEyes> {
    if !data.join("models").join(format!("{fdid}.m2")).is_file() {
        return Err(format!("model {fdid} not extracted"));
    }
    model_eyes(data, fdid).map_err(|err| format!("model {fdid}: {err}"))
}

/// Display `id`'s row per stand state, its eye source and how many states hold the
/// Stand eye; an error when a state's `.anim` file is not extracted.
fn rows_of(
    id: u32,
    display: &Display,
    model: &ModelEyes,
) -> Result<(Vec<EyeRow>, EyeSource, usize)> {
    let rows = model
        .eyes
        .iter()
        .map(|&(state, eye)| {
            let eye = eye.ok_or_else(|| {
                format!(
                    "model {}: stand state {state} .anim not extracted",
                    display.model_fdid
                )
            })?;
            Ok(EyeRow {
                display: id,
                stand_state: state,
                source: model.source,
                eye: (eye * display.scale).to_array(),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok((rows, model.source, model.stand_pose))
}

/// The eye point of model `fdid` per stand state: the centroid of its eye geometry, else
/// its head bone, posed in the state's animation; a state whose sequence the model lacks
/// holds the Stand pose. Without eye geometry or a head bone, the top centre of its
/// bounding box in every state; an error without that either.
fn model_eyes(data: &Path, fdid: u32) -> Result<ModelEyes> {
    let models = data.join("models");
    let bytes = std::fs::read(models.join(format!("{fdid}.m2"))).map_err(|err| err.to_string())?;
    let m2 = M2File::parse(&bytes)?;
    let skin = primary_skin(&models, fdid, m2.skin_fdid)?;
    let eye_vertices = m2.geoset_vertices(&skin, GEOSET_GROUP_EYES)?;
    let skel_bytes;
    // The extraction names the SKID skeleton after its model, as the client loads it
    // (`<model>.skel`).
    let skeleton = match m2.skeleton_fdid {
        Some(skel) => {
            skel_bytes = std::fs::read(models.join(format!("{fdid}.skel")))
                .map_err(|err| format!("skeleton {skel} ({fdid}.skel): {err}"))?;
            Skeleton::from_skel(&skel_bytes)?
        }
        None => Skeleton::from_md20(&m2)?,
    };
    let head = skeleton.head_bone()?;
    let source = match (eye_vertices.is_empty(), head) {
        (false, _) => EyeSource::Eyes,
        (true, Some(_)) => EyeSource::Head,
        (true, None) => {
            let top = m2.bounds_top()?;
            return Ok(ModelEyes {
                source: EyeSource::Bounds,
                eyes: STAND_STATE_ANIMS
                    .iter()
                    .map(|&(state, _)| state)
                    .chain(STAND_EYE_STATES)
                    .map(|state| (state, Some(top)))
                    .collect(),
                stand_pose: 0,
            });
        }
    };
    let mut load_anim =
        |anim_fdid: u32| std::fs::read(models.join(format!("{anim_fdid}.anim"))).ok();
    let mut eye_in = |anim: u16| -> Result<Option<Option<Vec3>>> {
        if !skeleton.has_sequence(anim) {
            return Ok(None);
        }
        let Some(mut pose) = skeleton.pose(anim, &mut load_anim)? else {
            return Ok(Some(None));
        };
        let eye = match head {
            Some(head) if eye_vertices.is_empty() => pose.bone_position(head)?,
            _ => {
                let mut sum = Vec3::ZERO;
                for vertex in &eye_vertices {
                    sum += pose.skin(vertex)?;
                }
                sum / eye_vertices.len() as f32
            }
        };
        Ok(Some(Some(eye)))
    };
    let Some(stand) = eye_in(0)? else {
        return Err("no Stand sequence".into());
    };
    let mut eyes = vec![(0, stand)];
    let mut stand_pose = 0;
    for &(state, anim) in &STAND_STATE_ANIMS[1..] {
        let eye = eye_in(anim)?;
        stand_pose += usize::from(eye.is_none());
        eyes.push((state, eye.unwrap_or(stand)));
    }
    eyes.extend(STAND_EYE_STATES.map(|state| (state, stand)));
    Ok(ModelEyes {
        source,
        eyes,
        stand_pose,
    })
}

fn open_db(world_db: &Path) -> Result<rusqlite::Connection> {
    rusqlite::Connection::open_with_flags(world_db, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|err| format!("{}: {err}", world_db.display()))
}

/// The displays spawned creatures show (their templates' models and spawn overrides)
/// and the `ChrModel.csv` character displays.
fn required_display_ids(data: &Path, world_db: &Path) -> Result<Vec<u32>> {
    let mut ids = query_ids(
        world_db,
        "SELECT m.CreatureDisplayID FROM content_creature c
           JOIN content_creature_template_model m ON m.CreatureID = c.id1
         UNION SELECT CAST(modelid AS INTEGER) FROM content_creature WHERE modelid != 0",
    )?;
    ids.extend(chr_model_displays(data)?.into_values());
    ids.sort_unstable();
    ids.dedup();
    Ok(ids)
}

/// Every creature template display: summons and transforms show displays no spawn does.
fn template_display_ids(world_db: &Path) -> Result<Vec<u32>> {
    query_ids(
        world_db,
        "SELECT DISTINCT CreatureDisplayID FROM content_creature_template_model",
    )
}

fn query_ids(world_db: &Path, sql: &str) -> Result<Vec<u32>> {
    open_db(world_db)?
        .prepare(sql)
        .and_then(|mut statement| {
            statement
                .query_map([], |row| row.get::<_, u32>(0))?
                .collect::<rusqlite::Result<Vec<u32>>>()
        })
        .map_err(|err| format!("display ids: {err}"))
}

/// The model and scale of each of `ids` that has a `CreatureDisplayInfo` row with a
/// model.
fn displays(world_db: &Path, ids: impl Iterator<Item = u32>) -> Result<HashMap<u32, Display>> {
    let db = open_db(world_db)?;
    let mut statement = db
        .prepare(
            "SELECT d.ModelID, d.CreatureModelScale, m.FileDataID, m.ModelScale
               FROM creature_display_info d JOIN creature_model_data m ON m.ID = d.ModelID
              WHERE d.ID = ?1",
        )
        .map_err(|err| err.to_string())?;
    let mut displays = HashMap::new();
    for id in ids {
        let row = statement.query_row([id], |row| {
            Ok((
                row.get::<_, u32>(2)?,
                row.get::<_, f64>(1)?,
                row.get::<_, f64>(3)?,
            ))
        });
        match row {
            Ok((model_fdid, display_scale, model_scale)) if model_fdid != 0 => {
                displays.insert(
                    id,
                    Display {
                        model_fdid,
                        scale: (display_scale * model_scale) as f32,
                    },
                );
            }
            Ok(_) | Err(rusqlite::Error::QueryReturnedNoRows) => {}
            Err(err) => return Err(format!("display {id}: {err}")),
        }
    }
    Ok(displays)
}

/// `((race, sex), display)` of each playable race and sex: `ChrRaceXChrModel.csv` names
/// the race's `ChrModel` per sex, whose `DisplayID` the character renders with.
pub fn player_displays(data: &Path) -> Result<Vec<((u8, u8), u32)>> {
    let models = chr_model_displays(data)?;
    let mut players: Vec<((u8, u8), u32)> = csv_rows(data, "ChrRaceXChrModel.csv")?
        .iter()
        .filter_map(|row| {
            let race = row.get("ChrRacesID")?.parse().ok()?;
            let sex = row.get("Sex")?.parse().ok()?;
            let model: u32 = row.get("ChrModelID")?.parse().ok()?;
            Some(((race, sex), *models.get(&model)?))
        })
        .collect();
    players.sort_unstable();
    players.dedup_by_key(|(key, _)| *key);
    Ok(players)
}

/// `ChrModel.csv` `DisplayID` by `ID`: the playable race/sex character displays.
fn chr_model_displays(data: &Path) -> Result<HashMap<u32, u32>> {
    Ok(csv_rows(data, "ChrModel.csv")?
        .iter()
        .filter_map(|row| {
            Some((
                row.get("ID")?.parse().ok()?,
                row.get("DisplayID")?.parse().ok()?,
            ))
        })
        .collect())
}

/// The rows of a DB2 CSV export in `data`, by column name.
fn csv_rows(data: &Path, name: &str) -> Result<Vec<HashMap<String, String>>> {
    let path = data.join(name);
    let text =
        std::fs::read_to_string(&path).map_err(|err| format!("{}: {err}", path.display()))?;
    let mut lines = text.lines();
    let header: Vec<&str> = lines.next().unwrap_or_default().split(',').collect();
    Ok(lines
        .map(|line| {
            header
                .iter()
                .zip(line.split(','))
                .map(|(column, value)| (column.to_string(), value.to_string()))
                .collect()
        })
        .collect())
}
