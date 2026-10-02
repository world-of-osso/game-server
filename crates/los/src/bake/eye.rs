//! The eye-height table: the Head attachment of every spawned creature display and
//! playable character model, per stand state (`crate::eye`).

use std::collections::HashMap;
use std::path::Path;

use super::chunks::Result;
use super::m2::{ATTACHMENT_HEAD, M2File, Skeleton};
use crate::eye::STAND_STATE_ANIMS;

#[derive(Default, Debug)]
pub struct EyeStats {
    pub displays: usize,
    /// Displays whose model is not extracted.
    pub missing_model: usize,
    /// Models that do not parse, with why.
    pub errors: Vec<String>,
    /// Displays whose model has no Head attachment.
    pub no_head: usize,
    /// (display, stand state) pairs whose model lacks the state's animation.
    pub missing_pose: usize,
    pub rows: usize,
}

/// `((display id, stand state), Head position)`, as `crate::eye::EyeTable` reads it.
pub type EyeRow = ((u32, u8), [f32; 3]);
/// Posed Head per animation of one model.
type ModelPoses = HashMap<u16, Option<[f32; 3]>>;

struct Display {
    id: u32,
    model_fdid: u32,
    scale: f32,
}

/// Rows `((display, stand state), head)` for the displays `world_db` spawns and the
/// `ChrModel.csv` character displays, read from `data` (game-engine `data/`).
pub fn eye_rows(data: &Path, world_db: &Path) -> Result<(Vec<EyeRow>, EyeStats)> {
    let displays = displays(data, world_db)?;
    let mut stats = EyeStats {
        displays: displays.len(),
        ..EyeStats::default()
    };
    // Posed Head per model and animation: `Err` when the model is not extracted (empty
    // message) or does not parse.
    let mut models: HashMap<u32, Result<ModelPoses>> = HashMap::new();
    let models_dir = data.join("models");
    let mut rows = Vec::new();
    for display in displays {
        let fdid = display.model_fdid;
        let poses = models.entry(fdid).or_insert_with(|| {
            if !models_dir.join(format!("{fdid}.m2")).is_file() {
                return Err(String::new());
            }
            model_poses(data, fdid).map_err(|err| {
                stats.errors.push(format!("model {fdid}: {err}"));
                err
            })
        });
        let Ok(poses) = poses else {
            if poses.as_ref().is_err_and(String::is_empty) {
                stats.missing_model += 1;
            }
            continue;
        };
        if poses.is_empty() {
            stats.no_head += 1;
            continue;
        }
        for (state, anim) in STAND_STATE_ANIMS {
            match poses.get(&anim).copied().flatten() {
                Some(head) => {
                    rows.push(((display.id, state), head.map(|axis| axis * display.scale)))
                }
                None => stats.missing_pose += 1,
            }
        }
    }
    rows.sort_by_key(|&(key, _)| key);
    stats.rows = rows.len();
    Ok((rows, stats))
}

/// Head per stand-state animation of model `fdid`; empty without a Head attachment.
fn model_poses(data: &Path, fdid: u32) -> Result<ModelPoses> {
    let models = data.join("models");
    let bytes = std::fs::read(models.join(format!("{fdid}.m2"))).map_err(|err| err.to_string())?;
    let m2 = M2File::parse(&bytes)?;
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
    let mut load_anim =
        |anim_fdid: u32| std::fs::read(models.join(format!("{anim_fdid}.anim"))).ok();
    let mut poses = HashMap::new();
    if !skeleton.has_attachment(ATTACHMENT_HEAD) {
        return Ok(poses);
    }
    for (_, anim) in STAND_STATE_ANIMS {
        let head = skeleton.posed_attachment(ATTACHMENT_HEAD, anim, &mut load_anim)?;
        poses.insert(anim, head);
    }
    Ok(poses)
}

fn displays(data: &Path, world_db: &Path) -> Result<Vec<Display>> {
    let db =
        rusqlite::Connection::open_with_flags(world_db, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|err| format!("{}: {err}", world_db.display()))?;
    let mut ids: Vec<u32> = db
        .prepare(
            "SELECT m.CreatureDisplayID FROM content_creature c
               JOIN content_creature_template_model m ON m.CreatureID = c.id1
             UNION SELECT CAST(modelid AS INTEGER) FROM content_creature WHERE modelid != 0",
        )
        .and_then(|mut statement| {
            statement
                .query_map([], |row| row.get::<_, u32>(0))?
                .collect::<rusqlite::Result<Vec<u32>>>()
        })
        .map_err(|err| format!("spawned displays: {err}"))?;
    ids.extend(chr_model_displays(data)?);
    ids.sort_unstable();
    ids.dedup();
    let mut statement = db
        .prepare(
            "SELECT d.ModelID, d.CreatureModelScale, m.FileDataID, m.ModelScale
               FROM creature_display_info d JOIN creature_model_data m ON m.ID = d.ModelID
              WHERE d.ID = ?1",
        )
        .map_err(|err| err.to_string())?;
    let mut displays = Vec::new();
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

/// `ChrModel.csv` `DisplayID` column: the playable race/sex character displays.
fn chr_model_displays(data: &Path) -> Result<Vec<u32>> {
    let path = data.join("ChrModel.csv");
    let text =
        std::fs::read_to_string(&path).map_err(|err| format!("{}: {err}", path.display()))?;
    let mut lines = text.lines();
    let header: Vec<&str> = lines.next().unwrap_or_default().split(',').collect();
    let column = header
        .iter()
        .position(|&name| name == "DisplayID")
        .ok_or("ChrModel.csv has no DisplayID column")?;
    Ok(lines
        .filter_map(|line| line.split(',').nth(column)?.parse().ok())
        .collect())
}
