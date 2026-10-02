//! M2 opaque geometry (primary skin) and the Head attachment posed by a sequence.

use std::collections::HashMap;

use glam::{Mat4, Quat, Vec3};

use super::chunks::{Result, array_at, f32x3_at, find, i16_at, records, u16_at, u32_at, u32s};

const MD20_SEQUENCES: usize = 0x1C;
const MD20_BONES: usize = 0x2C;
const MD20_VERTICES: usize = 0x3C;
const MD20_MATERIALS: usize = 0x70;
const MD20_ATTACHMENTS: usize = 0xF0;
const VERTEX_SIZE: usize = 48;
const SUBMESH_SIZE: usize = 48;
const BATCH_SIZE: usize = 24;
const BONE_SIZE: usize = 88;
const SEQUENCE_SIZE: usize = 64;
const ATTACHMENT_SIZE: usize = 40;
/// M2 material blend mode of an opaque batch (`M2BLEND_OPAQUE`); 1 is alpha-keyed, the
/// rest blend.
const BLEND_OPAQUE: u16 = 0;
/// Attachment id `Head` (wowdev M2 attachment table; 11 is the helm).
pub const ATTACHMENT_HEAD: u32 = 20;
/// M2Sequence flags: keyframes in this file (else in the `.anim` file), alias of
/// `aliasNext`.
const SEQUENCE_IN_FILE: u32 = 0x20;
const SEQUENCE_ALIAS: u32 = 0x40;

/// The MD20 blob and the chunks around it.
pub struct M2File<'a> {
    pub md20: &'a [u8],
    pub skin_fdid: Option<u32>,
    pub skeleton_fdid: Option<u32>,
    /// `(animation id, variation, .anim FDID)`.
    pub anim_files: Vec<(u16, u16, u32)>,
}

impl<'a> M2File<'a> {
    pub fn parse(data: &'a [u8]) -> Result<Self> {
        if data.starts_with(b"MD20") {
            return Ok(Self {
                md20: data,
                skin_fdid: None,
                skeleton_fdid: None,
                anim_files: Vec::new(),
            });
        }
        Ok(Self {
            md20: find(data, b"MD21").ok_or("M2 has no MD21")?,
            skin_fdid: find(data, b"SFID").and_then(|sfid| u32s(sfid).first().copied()),
            skeleton_fdid: find(data, b"SKID").and_then(|skid| u32s(skid).first().copied()),
            anim_files: afid(data),
        })
    }

    /// Triangles of the skin's submeshes whose base batch is opaque, in model file
    /// coordinates, and the triangle count of the alpha-keyed or blended ones.
    pub fn opaque_triangles(&self, skin: &[u8]) -> Result<(Vec<[[f32; 3]; 3]>, usize)> {
        let positions = records(
            self.md20,
            array_at(self.md20, MD20_VERTICES)?,
            VERTEX_SIZE,
            f32x3_at,
        )?;
        let blend_modes = records(
            self.md20,
            array_at(self.md20, MD20_MATERIALS)?,
            4,
            |data, offset| u16_at(data, offset + 2),
        )?;
        if !skin.starts_with(b"SKIN") {
            return Err("skin has no SKIN magic".into());
        }
        let lookup = records(skin, array_at(skin, 4)?, 2, u16_at)?;
        let indices = records(skin, array_at(skin, 12)?, 2, u16_at)?;
        let submeshes = records(skin, array_at(skin, 28)?, SUBMESH_SIZE, |data, offset| {
            // `level` carries the index start's high bits.
            let start =
                u32::from(u16_at(data, offset + 8)?) | u32::from(u16_at(data, offset + 2)?) << 16;
            Ok((start as usize, usize::from(u16_at(data, offset + 10)?)))
        })?;
        let batches = records(skin, array_at(skin, 36)?, BATCH_SIZE, |data, offset| {
            Ok((
                usize::from(u16_at(data, offset + 4)?),
                usize::from(u16_at(data, offset + 10)?),
                u16_at(data, offset + 12)?,
            ))
        })?;
        let mut opaque = Vec::new();
        let mut dropped = 0;
        for (submesh_index, &(start, count)) in submeshes.iter().enumerate() {
            // The submesh's base layer decides; a submesh without batches is not drawn.
            let Some(&(_, material, _)) = batches
                .iter()
                .find(|&&(submesh, _, layer)| submesh == submesh_index && layer == 0)
            else {
                continue;
            };
            let blend = *blend_modes.get(material).ok_or_else(|| {
                format!(
                    "batch material {material} past {} materials",
                    blend_modes.len()
                )
            })?;
            if blend != BLEND_OPAQUE {
                dropped += count / 3;
                continue;
            }
            let range = indices
                .get(start..start + count)
                .ok_or_else(|| format!("submesh indices {start}+{count} past {}", indices.len()))?;
            for tri in range.as_chunks::<3>().0.iter() {
                let corner = |index: u16| -> Result<[f32; 3]> {
                    let vertex = *lookup.get(usize::from(index)).ok_or_else(|| {
                        format!("skin index {index} past {} lookups", lookup.len())
                    })?;
                    positions
                        .get(usize::from(vertex))
                        .copied()
                        .ok_or_else(|| format!("skin vertex {vertex} past {}", positions.len()))
                };
                opaque.push([corner(tri[0])?, corner(tri[1])?, corner(tri[2])?]);
            }
        }
        Ok((opaque, dropped))
    }
}

fn afid(data: &[u8]) -> Vec<(u16, u16, u32)> {
    find(data, b"AFID")
        .unwrap_or_default()
        .as_chunks::<8>()
        .0
        .iter()
        .map(|entry| {
            (
                u16::from_le_bytes([entry[0], entry[1]]),
                u16::from_le_bytes([entry[2], entry[3]]),
                u32::from_le_bytes(entry[4..8].try_into().unwrap()),
            )
        })
        .collect()
}

struct Sequence {
    id: u16,
    variation: u16,
    flags: u32,
    alias_next: u16,
}

struct Attachment {
    id: u32,
    bone: usize,
    position: [f32; 3],
}

/// Bones, sequences and attachments: from the MD20, or from the `.skel` its SKID names.
pub struct Skeleton<'a> {
    /// The blob the bone and track offsets point into.
    bones_blob: &'a [u8],
    bones_offset: usize,
    bone_count: usize,
    sequences: Vec<Sequence>,
    attachments: Vec<Attachment>,
    anim_files: Vec<(u16, u16, u32)>,
    /// Track chunk of `.anim` files: `AFM2` for MD20 bones, `AFSB` for skeleton bones.
    anim_tag: &'static [u8; 4],
}

impl<'a> Skeleton<'a> {
    pub fn from_md20(m2: &M2File<'a>) -> Result<Self> {
        let (bone_count, bones_offset) = array_at(m2.md20, MD20_BONES)?;
        Ok(Self {
            bones_blob: m2.md20,
            bones_offset,
            bone_count,
            sequences: sequences(m2.md20, array_at(m2.md20, MD20_SEQUENCES)?)?,
            attachments: attachments(m2.md20, array_at(m2.md20, MD20_ATTACHMENTS)?)?,
            anim_files: m2.anim_files.clone(),
            anim_tag: b"AFM2",
        })
    }

    pub fn from_skel(skel: &'a [u8]) -> Result<Self> {
        let skb1 = find(skel, b"SKB1").ok_or("skeleton has no SKB1")?;
        let sks1 = find(skel, b"SKS1").ok_or("skeleton has no SKS1")?;
        let (bone_count, bones_offset) = array_at(skb1, 0)?;
        let attachments = match find(skel, b"SKA1") {
            Some(ska1) => attachments(ska1, array_at(ska1, 0)?)?,
            None => Vec::new(),
        };
        Ok(Self {
            bones_blob: skb1,
            bones_offset,
            bone_count,
            sequences: sequences(sks1, array_at(sks1, 8)?)?,
            attachments,
            anim_files: afid(skel),
            anim_tag: b"AFSB",
        })
    }

    pub fn has_attachment(&self, attachment: u32) -> bool {
        self.attachments.iter().any(|found| found.id == attachment)
    }

    /// The `attachment`'s position in model file coordinates, posed at the first frame of
    /// animation `anim` (variation 0); `None` when the model has no such attachment or
    /// sequence, or the sequence's `.anim` file is not extracted.
    pub fn posed_attachment(
        &self,
        attachment: u32,
        anim: u16,
        load_anim: &mut dyn FnMut(u32) -> Option<Vec<u8>>,
    ) -> Result<Option<[f32; 3]>> {
        let Some(attachment) = self.attachments.iter().find(|found| found.id == attachment) else {
            return Ok(None);
        };
        let Some(sequence) = self.sequence_index(anim) else {
            return Ok(None);
        };
        let seq = &self.sequences[sequence];
        let anim_data;
        let elements: &[u8] = if seq.flags & SEQUENCE_IN_FILE != 0 {
            self.bones_blob
        } else {
            let Some(&(_, _, fdid)) = self
                .anim_files
                .iter()
                .find(|&&(id, variation, _)| (id, variation) == (seq.id, seq.variation))
            else {
                return Ok(None);
            };
            let Some(file) = load_anim(fdid) else {
                return Ok(None);
            };
            anim_data = anim_track_chunk(&file, self.anim_tag).to_vec();
            &anim_data
        };
        let mut world: HashMap<usize, Mat4> = HashMap::new();
        let matrix = self.bone_world(attachment.bone, sequence, elements, &mut world)?;
        Ok(Some(
            matrix
                .transform_point3(Vec3::from(attachment.position))
                .to_array(),
        ))
    }

    /// Sequence index of `anim`'s variation 0, through aliases.
    fn sequence_index(&self, anim: u16) -> Option<usize> {
        let mut index = self
            .sequences
            .iter()
            .position(|seq| seq.id == anim && seq.variation == 0)?;
        for _ in 0..self.sequences.len() {
            let seq = &self.sequences[index];
            if seq.flags & SEQUENCE_ALIAS == 0 {
                return Some(index);
            }
            index = usize::from(seq.alias_next);
            self.sequences.get(index)?;
        }
        None
    }

    /// Model-space matrix of `bone` at the sequence's first frame:
    /// `parent * T(pivot) * T(translation) * R(rotation) * S(scale) * T(-pivot)`.
    fn bone_world(
        &self,
        bone: usize,
        sequence: usize,
        elements: &[u8],
        world: &mut HashMap<usize, Mat4>,
    ) -> Result<Mat4> {
        if let Some(matrix) = world.get(&bone) {
            return Ok(*matrix);
        }
        if bone >= self.bone_count {
            return Err(format!("bone {bone} past {} bones", self.bone_count));
        }
        let base = self.bones_offset + bone * BONE_SIZE;
        let blob = self.bones_blob;
        let parent = i16_at(blob, base + 0x08)?;
        let pivot = Vec3::from(f32x3_at(blob, base + 0x4C)?);
        let translation = first_key(blob, base + 0x10, sequence, elements, 12, f32x3_at)?
            .map_or(Vec3::ZERO, Vec3::from);
        let rotation = first_key(blob, base + 0x24, sequence, elements, 8, |data, offset| {
            let component =
                |index: usize| i16_at(data, offset + index * 2).map(unpack_quat_component);
            Ok(Quat::from_xyzw(
                component(0)?,
                component(1)?,
                component(2)?,
                component(3)?,
            ))
        })?
        .map_or(Quat::IDENTITY, Quat::normalize);
        let scale = first_key(blob, base + 0x38, sequence, elements, 12, f32x3_at)?
            .map_or(Vec3::ONE, Vec3::from);
        let local = Mat4::from_translation(pivot + translation)
            * Mat4::from_scale_rotation_translation(scale, rotation, Vec3::ZERO)
            * Mat4::from_translation(-pivot);
        let matrix = if parent >= 0 && (parent as usize) != bone {
            self.bone_world(parent as usize, sequence, elements, world)? * local
        } else {
            local
        };
        world.insert(bone, matrix);
        Ok(matrix)
    }
}

/// The first keyframe of `sequence` in the M2Track at `track` (a global-sequence track:
/// its single entry). Inner array headers are in `blob`; their elements in `elements`.
fn first_key<T>(
    blob: &[u8],
    track: usize,
    sequence: usize,
    elements: &[u8],
    size: usize,
    read: impl Fn(&[u8], usize) -> Result<T>,
) -> Result<Option<T>> {
    let global_sequence = i16_at(blob, track + 2)?;
    let (outer_count, outer_offset) = array_at(blob, track + 12)?;
    let entry = if global_sequence >= 0 { 0 } else { sequence };
    if entry >= outer_count {
        return Ok(None);
    }
    let (count, offset) = array_at(blob, outer_offset + entry * 8)?;
    if count == 0 {
        return Ok(None);
    }
    let source = if global_sequence >= 0 { blob } else { elements };
    records(source, (1, offset), size, read).map(|mut keys| keys.pop())
}

/// WoW's packed quaternion component (wowdev M2CompQuat).
fn unpack_quat_component(raw: i16) -> f32 {
    if raw < 0 {
        (f32::from(raw) + 32768.0) / 32767.0
    } else {
        (f32::from(raw) - 32767.0) / 32767.0
    }
}

fn sequences(data: &[u8], array: (usize, usize)) -> Result<Vec<Sequence>> {
    records(data, array, SEQUENCE_SIZE, |data, offset| {
        Ok(Sequence {
            id: u16_at(data, offset)?,
            variation: u16_at(data, offset + 2)?,
            flags: u32_at(data, offset + 0x0C)?,
            alias_next: u16_at(data, offset + 0x3E)?,
        })
    })
}

fn attachments(data: &[u8], array: (usize, usize)) -> Result<Vec<Attachment>> {
    records(data, array, ATTACHMENT_SIZE, |data, offset| {
        Ok(Attachment {
            id: u32_at(data, offset)?,
            bone: usize::from(u16_at(data, offset + 4)?),
            position: f32x3_at(data, offset + 8)?,
        })
    })
}

/// The `tag` chunk of a chunked `.anim` file; a chunkless (pre-Legion) file is its own
/// track data.
fn anim_track_chunk<'b>(file: &'b [u8], tag: &[u8; 4]) -> &'b [u8] {
    if !matches!(file.get(..4), Some(b"AFM2" | b"AFSA" | b"AFSB")) {
        return file;
    }
    find(file, tag).unwrap_or_default()
}
