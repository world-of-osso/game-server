//! The eye table: the eye point of every spawned creature display and playable character
//! model, per stand state (`crate::eye`), and the character display of each playable race
//! and sex.

use std::collections::HashMap;
use std::path::Path;

use glam::Vec3;

use super::chunks::Result;
use super::m2::{GEOSET_GROUP_EYES, M2File, Skeleton};
use super::primary_skin;
use crate::eye::{EyeRow, EyeSource, STAND_STATE_ANIMS};

#[derive(Default, Debug)]
pub struct EyeStats {
    pub displays: usize,
    /// Displays whose model is not extracted.
    pub missing_model: usize,
    /// Models that do not parse, with why.
    pub errors: Vec<String>,
    /// Displays by eye source.
    pub from_eyes: usize,
    pub from_head: usize,
    /// Displays whose model has neither eye geometry nor a head bone: the bounding box
    /// top.
    pub from_bounds: usize,
    /// (display, stand state) pairs whose model has no sequence for the state: they hold
    /// the Stand eye.
    pub stand_pose: usize,
    /// (display, stand state) pairs whose sequence's `.anim` file is not extracted.
    pub missing_anim: usize,
    pub rows: usize,
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
    id: u32,
    model_fdid: u32,
    scale: f32,
}

/// Rows for the displays `world_db` spawns and the `ChrModel.csv` character displays,
/// read from `data` (game-engine `data/`).
pub fn eye_rows(data: &Path, world_db: &Path) -> Result<(Vec<EyeRow>, EyeStats)> {
    eye_rows_of(data, world_db, &baked_display_ids(data, world_db)?)
}

/// Rows for `display_ids`.
pub fn eye_rows_of(
    data: &Path,
    world_db: &Path,
    display_ids: &[u32],
) -> Result<(Vec<EyeRow>, EyeStats)> {
    let displays = displays(world_db, display_ids)?;
    let mut stats = EyeStats {
        displays: displays.len(),
        ..EyeStats::default()
    };
    // `Err` with an empty message: the model is not extracted.
    let mut models: HashMap<u32, Result<ModelEyes>> = HashMap::new();
    let models_dir = data.join("models");
    let mut rows = Vec::new();
    for display in displays {
        let fdid = display.model_fdid;
        let eyes = models.entry(fdid).or_insert_with(|| {
            if !models_dir.join(format!("{fdid}.m2")).is_file() {
                return Err(String::new());
            }
            model_eyes(data, fdid).map_err(|err| {
                stats.errors.push(format!("model {fdid}: {err}"));
                err
            })
        });
        let model = match eyes {
            Ok(model) => model,
            Err(err) => {
                stats.missing_model += usize::from(err.is_empty());
                continue;
            }
        };
        match model.source {
            EyeSource::Eyes => stats.from_eyes += 1,
            EyeSource::Head => stats.from_head += 1,
            EyeSource::Bounds => stats.from_bounds += 1,
        }
        stats.stand_pose += model.stand_pose;
        for &(state, eye) in &model.eyes {
            let Some(eye) = eye else {
                stats.missing_anim += 1;
                continue;
            };
            rows.push(EyeRow {
                display: display.id,
                stand_state: state,
                source: model.source,
                eye: (eye * display.scale).to_array(),
            });
        }
    }
    rows.sort_by_key(|row| (row.display, row.stand_state));
    stats.rows = rows.len();
    let mut states: HashMap<u32, usize> = HashMap::new();
    for row in &rows {
        *states.entry(row.display).or_default() += 1;
    }
    let mut incomplete: Vec<u32> = display_ids
        .iter()
        .copied()
        .filter(|id| states.get(id).copied().unwrap_or(0) < STAND_STATE_ANIMS.len())
        .collect();
    incomplete.sort_unstable();
    incomplete.dedup();
    if !incomplete.is_empty() || !stats.errors.is_empty() {
        // A unit without an eye row could not be LOS-checked: no partial table.
        return Err(format!(
            "eye table incomplete: displays without every stand state {incomplete:?}; model \
             not extracted: {}, .anim not extracted: {} (run scripts/los-extract-models.py); \
             unreadable models: {:?}",
            stats.missing_model, stats.missing_anim, stats.errors
        ));
    }
    Ok((rows, stats))
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
                    .map(|&(state, _)| (state, Some(top)))
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

/// Every creature template display (summons and transforms show displays no spawn
/// does), spawn display overrides and the `ChrModel.csv` character displays.
fn baked_display_ids(data: &Path, world_db: &Path) -> Result<Vec<u32>> {
    let mut ids: Vec<u32> = open_db(world_db)?
        .prepare(
            "SELECT CreatureDisplayID FROM content_creature_template_model
             UNION SELECT CAST(modelid AS INTEGER) FROM content_creature WHERE modelid != 0",
        )
        .and_then(|mut statement| {
            statement
                .query_map([], |row| row.get::<_, u32>(0))?
                .collect::<rusqlite::Result<Vec<u32>>>()
        })
        .map_err(|err| format!("spawned displays: {err}"))?;
    ids.extend(chr_model_displays(data)?.into_values());
    ids.sort_unstable();
    ids.dedup();
    Ok(ids)
}

fn displays(world_db: &Path, ids: &[u32]) -> Result<Vec<Display>> {
    let db = open_db(world_db)?;
    let mut statement = db
        .prepare(
            "SELECT d.ModelID, d.CreatureModelScale, m.FileDataID, m.ModelScale
               FROM creature_display_info d JOIN creature_model_data m ON m.ID = d.ModelID
              WHERE d.ID = ?1",
        )
        .map_err(|err| err.to_string())?;
    let mut displays = Vec::new();
    for &id in ids {
        let row = statement.query_row([id], |row| {
            Ok((
                row.get::<_, u32>(2)?,
                row.get::<_, f64>(1)?,
                row.get::<_, f64>(3)?,
            ))
        });
        match row {
            Ok((model_fdid, display_scale, model_scale)) if model_fdid != 0 => {
                displays.push(Display {
                    id,
                    model_fdid,
                    scale: (display_scale * model_scale) as f32,
                })
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
