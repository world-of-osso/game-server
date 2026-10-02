//! M2 opaque geometry (primary skin), and the eye geometry or head bone posed by a
//! sequence.

use std::borrow::Cow;
use std::collections::HashMap;

use glam::{Mat4, Quat, Vec3};

use super::chunks::{
    Result, array_at, bytes, f32x3_at, find, i16_at, records, u16_at, u32_at, u32s,
};

const MD20_SEQUENCES: usize = 0x1C;
const MD20_BONES: usize = 0x2C;
const MD20_VERTICES: usize = 0x3C;
const MD20_MATERIALS: usize = 0x70;
/// `CAaBox bounding_box`: min then max corner, model file coordinates.
const MD20_BOUNDING_BOX: usize = 0xA0;
const VERTEX_SIZE: usize = 48;
const SUBMESH_SIZE: usize = 48;
const BATCH_SIZE: usize = 24;
const BONE_SIZE: usize = 88;
const SEQUENCE_SIZE: usize = 64;
/// M2 material blend mode of an opaque batch (`M2BLEND_OPAQUE`); 1 is alpha-keyed, the
/// rest blend.
const BLEND_OPAQUE: u16 = 0;
/// M2CompBone `key_bone_id` of the head (wowdev key bone table).
const KEY_BONE_HEAD: i32 = 6;
/// Geoset group of the eyes (`skinSectionId / 100`; wowdev geoset 17xx "eyeglows").
pub const GEOSET_GROUP_EYES: u16 = 17;
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

    /// The top centre of the model's bounding box (a flat model's box has no height); an
    /// error for an inverted box, which a model without geometry has.
    pub fn bounds_top(&self) -> Result<Vec3> {
        let min = Vec3::from(f32x3_at(self.md20, MD20_BOUNDING_BOX)?);
        let max = Vec3::from(f32x3_at(self.md20, MD20_BOUNDING_BOX + 12)?);
        if !(max.cmpge(min).all() && min.is_finite() && max.is_finite()) {
            return Err(format!("empty bounding box {min}..{max}"));
        }
        Ok(Vec3::new(
            (min.x + max.x) * 0.5,
            (min.y + max.y) * 0.5,
            max.z,
        ))
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
    /// The vertices of the skin's submeshes in geoset `group` (`skinSectionId / 100`),
    /// each once, in model file coordinates with their bone influences.
    pub fn geoset_vertices(&self, skin: &[u8], group: u16) -> Result<Vec<SkinnedVertex>> {
        if !skin.starts_with(b"SKIN") {
            return Err("skin has no SKIN magic".into());
        }
        let lookup = records(skin, array_at(skin, 4)?, 2, u16_at)?;
        let submeshes = records(skin, array_at(skin, 28)?, SUBMESH_SIZE, |data, offset| {
            Ok((
                u16_at(data, offset)?,
                usize::from(u16_at(data, offset + 4)?),
                usize::from(u16_at(data, offset + 6)?),
            ))
        })?;
        let (count, offset) = array_at(self.md20, MD20_VERTICES)?;
        let mut vertices: Vec<usize> = Vec::new();
        for &(section, start, len) in &submeshes {
            if section / 100 != group {
                continue;
            }
            let range = lookup
                .get(start..start + len)
                .ok_or_else(|| format!("submesh vertices {start}+{len} past {}", lookup.len()))?;
            vertices.extend(range.iter().map(|&vertex| usize::from(vertex)));
        }
        vertices.sort_unstable();
        vertices.dedup();
        vertices
            .into_iter()
            .map(|vertex| {
                if vertex >= count {
                    return Err(format!("skin vertex {vertex} past {count}"));
                }
                let base = offset + vertex * VERTEX_SIZE;
                Ok(SkinnedVertex {
                    position: Vec3::from(f32x3_at(self.md20, base)?),
                    weights: bytes(self.md20, base + 12)?,
                    bones: bytes(self.md20, base + 16)?,
                })
            })
            .collect()
    }
}

/// A model vertex and the bones that move it.
pub struct SkinnedVertex {
    pub position: Vec3,
    /// Out of 255.
    pub weights: [u8; 4],
    pub bones: [u8; 4],
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

/// Bones and sequences: from the MD20, or from the `.skel` its SKID names.
pub struct Skeleton<'a> {
    /// The blob the bone and track offsets point into.
    bones_blob: &'a [u8],
    bones_offset: usize,
    bone_count: usize,
    sequences: Vec<Sequence>,
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
            anim_files: m2.anim_files.clone(),
            anim_tag: b"AFM2",
        })
    }

    pub fn from_skel(skel: &'a [u8]) -> Result<Self> {
        let skb1 = find(skel, b"SKB1").ok_or("skeleton has no SKB1")?;
        let sks1 = find(skel, b"SKS1").ok_or("skeleton has no SKS1")?;
        let (bone_count, bones_offset) = array_at(skb1, 0)?;
        Ok(Self {
            bones_blob: skb1,
            bones_offset,
            bone_count,
            sequences: sequences(sks1, array_at(sks1, 8)?)?,
            anim_files: afid(skel),
            anim_tag: b"AFSB",
        })
    }

    /// The bone whose `key_bone_id` is the head.
    pub fn head_bone(&self) -> Result<Option<usize>> {
        for bone in 0..self.bone_count {
            let key = u32_at(self.bones_blob, self.bones_offset + bone * BONE_SIZE)? as i32;
            if key == KEY_BONE_HEAD {
                return Ok(Some(bone));
            }
        }
        Ok(None)
    }

    /// The skeleton at the first frame of animation `anim` (variation 0); `None` when the
    /// model has no such sequence or its `.anim` file is not extracted.
    pub fn pose(
        &self,
        anim: u16,
        load_anim: &mut dyn FnMut(u32) -> Option<Vec<u8>>,
    ) -> Result<Option<Pose<'_>>> {
        let Some(sequence) = self.sequence_index(anim) else {
            return Ok(None);
        };
        let seq = &self.sequences[sequence];
        let elements = if seq.flags & SEQUENCE_IN_FILE != 0 {
            Cow::Borrowed(self.bones_blob)
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
            Cow::Owned(anim_track_chunk(&file, self.anim_tag).to_vec())
        };
        Ok(Some(Pose {
            skeleton: self,
            sequence,
            elements,
            world: HashMap::new(),
        }))
    }

    pub fn has_sequence(&self, anim: u16) -> bool {
        self.sequence_index(anim).is_some()
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

    fn pivot(&self, bone: usize) -> Result<Vec3> {
        f32x3_at(self.bones_blob, self.bones_offset + bone * BONE_SIZE + 0x4C).map(Vec3::from)
    }
}

/// A skeleton posed at one sequence's first frame.
pub struct Pose<'s> {
    skeleton: &'s Skeleton<'s>,
    sequence: usize,
    /// Where the sequence's keyframes live: the bones blob or the `.anim` track chunk.
    elements: Cow<'s, [u8]>,
    world: HashMap<usize, Mat4>,
}

impl Pose<'_> {
    /// Where `vertex` is drawn, in model file coordinates.
    pub fn skin(&mut self, vertex: &SkinnedVertex) -> Result<Vec3> {
        let mut posed = Vec3::ZERO;
        let mut total = 0.0;
        for (&bone, &weight) in vertex.bones.iter().zip(&vertex.weights) {
            if weight == 0 {
                continue;
            }
            let weight = f32::from(weight) / 255.0;
            posed += self
                .bone(usize::from(bone))?
                .transform_point3(vertex.position)
                * weight;
            total += weight;
        }
        if total == 0.0 {
            return Err("vertex has no bone weights".into());
        }
        Ok(posed / total)
    }

    /// Where `bone`'s pivot is, in model file coordinates.
    pub fn bone_position(&mut self, bone: usize) -> Result<Vec3> {
        let pivot = self.skeleton.pivot(bone)?;
        Ok(self.bone(bone)?.transform_point3(pivot))
    }

    /// Model-space matrix of `bone`:
    /// `parent * T(pivot) * T(translation) * R(rotation) * S(scale) * T(-pivot)`.
    fn bone(&mut self, bone: usize) -> Result<Mat4> {
        if let Some(matrix) = self.world.get(&bone) {
            return Ok(*matrix);
        }
        let skeleton = self.skeleton;
        if bone >= skeleton.bone_count {
            return Err(format!("bone {bone} past {} bones", skeleton.bone_count));
        }
        let base = skeleton.bones_offset + bone * BONE_SIZE;
        let blob = skeleton.bones_blob;
        let (sequence, elements) = (self.sequence, &*self.elements);
        let parent = i16_at(blob, base + 0x08)?;
        let pivot = skeleton.pivot(bone)?;
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
            self.bone(parent as usize)? * local
        } else {
            local
        };
        self.world.insert(bone, matrix);
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

/// The `tag` chunk of a chunked `.anim` file; a chunkless (pre-Legion) file is its own
/// track data.
fn anim_track_chunk<'b>(file: &'b [u8], tag: &[u8; 4]) -> &'b [u8] {
    if !matches!(file.get(..4), Some(b"AFM2" | b"AFSA" | b"AFSB")) {
        return file;
    }
    find(file, tag).unwrap_or_default()
}
