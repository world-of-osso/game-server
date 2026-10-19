//! Geometry selection and first-frame eye posing over shared M2/skin readers.
use std::borrow::Cow;
use std::collections::HashMap;

use glam::{Mat4, Quat, Vec3};
use wow_chunk::{read_m2_array_header, read_vec3};
use wow_m2::{m2_anim, m2_collision, parser};

use super::chunks::{Result, find};

const BLEND_OPAQUE: u16 = 0;
const KEY_BONE_HEAD: i32 = 6;
pub const GEOSET_GROUP_EYES: u16 = 17;

/// File references and the MD20 payload. Raw MD20 and chunked MD21 remain supported.
pub struct M2File<'a> {
    pub md20: &'a [u8],
    pub skin_fdid: Option<u32>,
    pub skeleton_fdid: Option<u32>,
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
        let chunks = parser::parse_chunks(data)?;
        Ok(Self {
            md20: chunks.md20,
            skin_fdid: chunks.sfid.first().copied(),
            skeleton_fdid: chunks.skid,
            anim_files: chunks.afid,
        })
    }

    /// The authored collision hull, in model-local engine axes, never the render mesh.
    pub fn collision_triangles(&self) -> Result<Vec<[Vec3; 3]>> {
        let Some(mesh) = m2_collision::parse_collision_mesh(self.md20)? else {
            return Ok(Vec::new());
        };
        Ok(mesh
            .indices
            .as_chunks::<3>()
            .0
            .iter()
            .map(|triangle| {
                triangle.map(|index| super::adt::file_to_bevy(mesh.vertices[usize::from(index)]))
            })
            .collect())
    }

    pub fn collision_normals(&self) -> Result<Vec<Vec3>> {
        Ok(m2_collision::parse_collision_normals(self.md20)?
            .into_iter()
            .map(super::adt::file_to_bevy)
            .collect())
    }

    pub fn bounds_top(&self) -> Result<Vec3> {
        let min = Vec3::from(read_vec3(self.md20, wow_m2::MD20_BOUNDING_BOX_MIN_OFFSET)?);
        let max = Vec3::from(read_vec3(self.md20, wow_m2::MD20_BOUNDING_BOX_MAX_OFFSET)?);
        if !(max.cmpge(min).all() && min.is_finite() && max.is_finite()) {
            return Err(format!("empty bounding box {min}..{max}"));
        }
        Ok(Vec3::new(
            (min.x + max.x) * 0.5,
            (min.y + max.y) * 0.5,
            max.z,
        ))
    }

    /// Base opaque batches only, preserving authored skin ranges and dropped-face counts.
    pub fn opaque_triangles(&self, skin: &[u8]) -> Result<(Vec<[[f32; 3]; 3]>, usize)> {
        let positions = parser::parse_vertices(self.md20)?;
        let materials = parser::parse_materials(self.md20)?;
        let skin = parser::parse_skin_full(skin)?;
        let mut opaque = Vec::new();
        let mut dropped = 0;
        for (submesh_index, submesh) in skin.submeshes.iter().enumerate() {
            let Some(batch) = skin.batches.iter().find(|batch| {
                usize::from(batch.submesh_index) == submesh_index && batch.material_layer == 0
            }) else {
                continue;
            };
            let blend = materials
                .get(usize::from(batch.render_flags_index))
                .ok_or_else(|| {
                    format!(
                        "batch material {} past {} materials",
                        batch.render_flags_index,
                        materials.len()
                    )
                })?
                .blend_mode;
            let count = usize::from(submesh.triangle_count);
            if blend != BLEND_OPAQUE {
                dropped += count / 3;
                continue;
            }
            let start = submesh.raw_triangle_start as usize;
            let range = skin.indices.get(start..start + count).ok_or_else(|| {
                format!(
                    "submesh indices {start}+{count} past {}",
                    skin.indices.len()
                )
            })?;
            for triangle in range.as_chunks::<3>().0 {
                let corner = |index: u16| -> Result<[f32; 3]> {
                    let vertex = *skin.lookup.get(usize::from(index)).ok_or_else(|| {
                        format!("skin index {index} past {} lookups", skin.lookup.len())
                    })?;
                    positions
                        .get(usize::from(vertex))
                        .map(|vertex| vertex.position)
                        .ok_or_else(|| format!("skin vertex {vertex} past {}", positions.len()))
                };
                opaque.push([
                    corner(triangle[0])?,
                    corner(triangle[1])?,
                    corner(triangle[2])?,
                ]);
            }
        }
        Ok((opaque, dropped))
    }

    pub fn geoset_vertices(&self, skin: &[u8], group: u16) -> Result<Vec<SkinnedVertex>> {
        let skin = parser::parse_skin_full(skin)?;
        let positions = parser::parse_vertices(self.md20)?;
        let mut vertices = Vec::new();
        for section in &skin.submeshes {
            if section.mesh_part_id / 100 != group {
                continue;
            }
            let start = usize::from(section.vertex_start);
            let len = usize::from(section.vertex_count);
            let range = skin.lookup.get(start..start + len).ok_or_else(|| {
                format!("submesh vertices {start}+{len} past {}", skin.lookup.len())
            })?;
            vertices.extend(range.iter().map(|&vertex| usize::from(vertex)));
        }
        vertices.sort_unstable();
        vertices.dedup();
        vertices
            .into_iter()
            .map(|index| {
                let vertex = positions
                    .get(index)
                    .ok_or_else(|| format!("skin vertex {index} past {}", positions.len()))?;
                Ok(SkinnedVertex {
                    position: Vec3::from(vertex.position),
                    weights: vertex.bone_weights,
                    bones: vertex.bone_indices,
                })
            })
            .collect()
    }
}

pub struct SkinnedVertex {
    pub position: Vec3,
    pub weights: [u8; 4],
    pub bones: [u8; 4],
}

/// First-frame posing policy stays server-side; record and track decoding is shared.
pub struct Skeleton<'a> {
    bones_blob: &'a [u8],
    bones_offset: usize,
    bones: Vec<m2_anim::M2Bone>,
    sequences: Vec<m2_anim::M2AnimSequence>,
    aliases: Vec<u16>,
    anim_files: Vec<(u16, u16, u32)>,
    anim_tag: &'static [u8; 4],
}

impl<'a> Skeleton<'a> {
    pub fn from_md20(m2: &M2File<'a>) -> Result<Self> {
        let (bone_count, bones_offset) =
            read_m2_array_header(m2.md20, wow_m2::MD20_BONES_COUNT_OFFSET)?;
        let (sequence_count, sequence_offset) =
            read_m2_array_header(m2.md20, wow_m2::MD20_SEQUENCES_COUNT_OFFSET)?;
        Ok(Self {
            bones_blob: m2.md20,
            bones_offset,
            bones: m2_anim::parse_bones_at(m2.md20, bones_offset, bone_count)?,
            sequences: m2_anim::parse_sequences_at(m2.md20, sequence_offset, sequence_count)?,
            aliases: m2_anim::parse_sequence_aliases_at(m2.md20, sequence_offset, sequence_count)?,
            anim_files: m2.anim_files.clone(),
            anim_tag: b"AFM2",
        })
    }

    pub fn from_skel(skel: &'a [u8]) -> Result<Self> {
        let skb1 = find(skel, b"SKB1").ok_or("skeleton has no SKB1")?;
        let sks1 = find(skel, b"SKS1").ok_or("skeleton has no SKS1")?;
        let (bone_count, bones_offset) = read_m2_array_header(skb1, 0)?;
        let (sequence_count, sequence_offset) = read_m2_array_header(sks1, 8)?;
        Ok(Self {
            bones_blob: skb1,
            bones_offset,
            bones: m2_anim::parse_bones_at(skb1, bones_offset, bone_count)?,
            sequences: m2_anim::parse_sequences_at(sks1, sequence_offset, sequence_count)?,
            aliases: m2_anim::parse_sequence_aliases_at(sks1, sequence_offset, sequence_count)?,
            anim_files: parser::skeleton_afid(skel)?,
            anim_tag: b"AFSB",
        })
    }

    pub fn head_bone(&self) -> Result<Option<usize>> {
        Ok(self
            .bones
            .iter()
            .position(|bone| bone.key_bone_id == KEY_BONE_HEAD))
    }

    pub fn pose(
        &self,
        anim: u16,
        load_anim: &mut dyn FnMut(u32) -> Option<Vec<u8>>,
    ) -> Result<Option<Pose<'_>>> {
        let Some(sequence) = self.sequence_index(anim) else {
            return Ok(None);
        };
        let seq = &self.sequences[sequence];
        let elements = if seq.flags & m2_anim::M2_SEQUENCE_IN_FILE != 0 {
            Cow::Borrowed(self.bones_blob)
        } else {
            let Some(&(_, _, fdid)) = self
                .anim_files
                .iter()
                .find(|&&(id, variation, _)| (id, variation) == (seq.id, seq.variation_id))
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

    fn sequence_index(&self, anim: u16) -> Option<usize> {
        let mut index = self
            .sequences
            .iter()
            .position(|seq| seq.id == anim && seq.variation_id == 0)?;
        for _ in 0..self.sequences.len() {
            if self.sequences[index].flags & m2_anim::M2_SEQUENCE_ALIAS == 0 {
                return Some(index);
            }
            index = usize::from(self.aliases[index]);
            self.sequences.get(index)?;
        }
        None
    }
}

pub struct Pose<'s> {
    skeleton: &'s Skeleton<'s>,
    sequence: usize,
    elements: Cow<'s, [u8]>,
    world: HashMap<usize, Mat4>,
}

impl Pose<'_> {
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

    pub fn bone_position(&mut self, bone: usize) -> Result<Vec3> {
        let pivot = Vec3::from(
            self.skeleton
                .bones
                .get(bone)
                .ok_or_else(|| format!("bone {bone} past {} bones", self.skeleton.bones.len()))?
                .pivot,
        );
        Ok(self.bone(bone)?.transform_point3(pivot))
    }

    fn bone(&mut self, bone: usize) -> Result<Mat4> {
        if let Some(matrix) = self.world.get(&bone) {
            return Ok(*matrix);
        }
        let skeleton = self.skeleton;
        let metadata = skeleton
            .bones
            .get(bone)
            .ok_or_else(|| format!("bone {bone} past {} bones", skeleton.bones.len()))?;
        let keys = m2_anim::first_bone_transform(
            skeleton.bones_blob,
            skeleton.bones_offset,
            bone,
            self.sequence,
            &self.elements,
        )?;
        let parent = metadata.parent_bone_id;
        let pivot = Vec3::from(metadata.pivot);
        let translation = keys.translation.map_or(Vec3::ZERO, Vec3::from);
        let rotation = keys
            .rotation
            .map(|packed| Quat::from_array(packed.map(m2_anim::unpack_quat_component)).normalize())
            .unwrap_or(Quat::IDENTITY);
        let scale = keys.scale.map_or(Vec3::ONE, Vec3::from);
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

fn anim_track_chunk<'b>(file: &'b [u8], tag: &[u8; 4]) -> &'b [u8] {
    if !matches!(file.get(..4), Some(b"AFM2" | b"AFSA" | b"AFSB")) {
        return file;
    }
    find(file, tag).unwrap_or_default()
}
