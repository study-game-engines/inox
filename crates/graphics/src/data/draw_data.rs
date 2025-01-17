use inox_bitmask::bitmask;
use inox_math::{Mat4Ops, Matrix4};
use inox_serialize::{Deserialize, Serialize};

use crate::{
    MaterialAlphaMode, TextureType, VertexBufferLayoutBuilder, VertexFormat, INVALID_INDEX,
    MAX_TEXTURE_COORDS_SETS,
};

// Pipeline has a list of meshes to process
// Meshes can switch pipeline at runtime
// Material doesn't know pipeline anymore
// Material is now generic data for several purposes

#[bitmask]
pub enum DrawCommandType {
    PerMeshlet,
    PerTriangle,
}

#[repr(C)]
#[derive(Default, Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Debug)]
#[serde(crate = "inox_serialize")]
pub struct DrawIndexedCommand {
    pub vertex_count: u32,
    pub instance_count: u32,
    pub base_index: u32,
    pub vertex_offset: i32,
    pub base_instance: u32,
}

#[repr(C, align(4))]
#[derive(Default, Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Debug)]
#[serde(crate = "inox_serialize")]
pub struct DrawCommand {
    pub vertex_count: u32,
    pub instance_count: u32,
    pub base_vertex: u32,
    pub base_instance: u32,
}

#[repr(C, align(4))]
#[derive(Serialize, Deserialize, PartialEq, Clone, Copy, Debug)]
#[serde(crate = "inox_serialize")]
pub struct DrawMesh {
    pub vertex_offset: u32,
    pub indices_offset: u32,
    pub material_index: i32,
    pub bhv_index: u32,
    pub position: [f32; 3],
    pub meshlets_offset: u32,
    pub scale: [f32; 3],
    pub meshlets_count: u32,
    pub orientation: [f32; 4],
}

impl Default for DrawMesh {
    fn default() -> Self {
        Self {
            vertex_offset: 0,
            indices_offset: 0,
            material_index: INVALID_INDEX,
            bhv_index: 0,
            position: [0.; 3],
            meshlets_offset: 0,
            scale: [1.; 3],
            meshlets_count: 0,
            orientation: [0., 0., 0., 1.],
        }
    }
}

impl DrawMesh {
    pub fn transform(&self) -> Matrix4 {
        Matrix4::from_translation_orientation_scale(
            self.position.into(),
            self.orientation.into(),
            self.scale.into(),
        )
    }
}

#[repr(C, align(4))]
#[derive(Default, Serialize, Deserialize, PartialEq, Clone, Copy, Debug)]
#[serde(crate = "inox_serialize")]
pub struct ConeCulling {
    pub center: [f32; 3],
    pub cone_axis_cutoff: [i8; 4],
}

#[repr(C, align(4))]
#[derive(Default, Serialize, Deserialize, PartialEq, Clone, Copy, Debug)]
#[serde(crate = "inox_serialize")]
pub struct DrawMeshlet {
    pub mesh_index: u32,
    pub indices_offset: u32,
    pub indices_count: u32,
    pub bvh_index: u32,
}

impl DrawMeshlet {
    pub fn descriptor<'a>(starting_location: u32) -> VertexBufferLayoutBuilder<'a> {
        let mut layout_builder = VertexBufferLayoutBuilder::instance();
        layout_builder.starting_location(starting_location);
        layout_builder.add_attribute::<u32>(VertexFormat::Uint32.into());
        layout_builder.add_attribute::<u32>(VertexFormat::Uint32.into());
        layout_builder.add_attribute::<u32>(VertexFormat::Uint32.into());
        layout_builder.add_attribute::<u32>(VertexFormat::Uint32.into());
        layout_builder
    }
}

#[repr(C, align(4))]
#[derive(Default, Serialize, Deserialize, PartialEq, Clone, Copy, Debug)]
#[serde(crate = "inox_serialize")]
pub struct DrawBHVNode {
    pub min: [f32; 3],
    pub miss: i32,
    pub max: [f32; 3],
    pub reference: i32,
}

#[repr(C, align(4))]
#[derive(Serialize, Deserialize, PartialEq, Clone, Copy, Debug)]
#[serde(crate = "inox_serialize")]
pub struct DrawMaterial {
    pub textures_indices: [i32; TextureType::Count as _],
    pub textures_coord_set: [u32; TextureType::Count as _],
    pub roughness_factor: f32,
    pub metallic_factor: f32,
    pub alpha_cutoff: f32,
    pub alpha_mode: u32,
    pub base_color: [f32; 4],
    pub emissive_color: [f32; 3],
    pub occlusion_strength: f32,
    pub diffuse_color: [f32; 4],
    pub specular_color: [f32; 4],
}

impl Default for DrawMaterial {
    fn default() -> Self {
        Self {
            textures_indices: [INVALID_INDEX; TextureType::Count as _],
            textures_coord_set: [0; TextureType::Count as _],
            roughness_factor: 0.,
            metallic_factor: 0.,
            alpha_cutoff: 1.,
            alpha_mode: MaterialAlphaMode::Opaque.into(),
            base_color: [1.; 4],
            emissive_color: [1.; 3],
            occlusion_strength: 0.0,
            diffuse_color: [1.; 4],
            specular_color: [1.; 4],
        }
    }
}

#[repr(C, align(4))]
#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Debug)]
#[serde(crate = "inox_serialize")]
pub struct DrawVertex {
    pub position_and_color_offset: u32,
    pub normal_offset: i32,
    pub tangent_offset: i32,
    pub mesh_index: u32,
    pub uv_offset: [i32; MAX_TEXTURE_COORDS_SETS],
}

impl Default for DrawVertex {
    fn default() -> Self {
        Self {
            position_and_color_offset: 0,
            normal_offset: INVALID_INDEX,
            tangent_offset: INVALID_INDEX,
            mesh_index: 0,
            uv_offset: [INVALID_INDEX; MAX_TEXTURE_COORDS_SETS],
        }
    }
}

impl DrawVertex {
    pub fn descriptor<'a>(starting_location: u32) -> VertexBufferLayoutBuilder<'a> {
        let mut layout_builder = VertexBufferLayoutBuilder::vertex();
        layout_builder.starting_location(starting_location);
        layout_builder.add_attribute::<u32>(VertexFormat::Uint32.into());
        layout_builder.add_attribute::<i32>(VertexFormat::Sint32.into());
        layout_builder.add_attribute::<i32>(VertexFormat::Sint32.into());
        layout_builder.add_attribute::<u32>(VertexFormat::Uint32.into());
        layout_builder
            .add_attribute::<[i32; MAX_TEXTURE_COORDS_SETS]>(VertexFormat::Sint32x4.into());
        layout_builder
    }
}

#[repr(C, align(4))]
#[derive(Default, Serialize, Deserialize, PartialEq, Clone, Copy, Debug)]
#[serde(crate = "inox_serialize")]
pub struct DrawRay {
    pub origin: [f32; 3],
    pub t_min: f32,
    pub direction: [f32; 3],
    pub t_max: f32,
}
