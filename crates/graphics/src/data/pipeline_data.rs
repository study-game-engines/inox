use std::path::PathBuf;

use inox_filesystem::convert_from_local_path;
use inox_math::Matrix4;
use inox_resources::Data;
use inox_serialize::{Deserialize, Serialize, SerializeFile};

use crate::{
    BindingDataType, LightData, ShaderMaterialData, TextureAtlas, TextureData, VertexFormat,
};

#[derive(Serialize, Deserialize, Debug, PartialOrd, PartialEq, Copy, Clone)]
#[serde(crate = "inox_serialize")]
pub enum PolygonModeType {
    Fill,
    Line,
    Point,
}

#[derive(Serialize, Deserialize, Debug, PartialOrd, PartialEq, Copy, Clone)]
#[serde(crate = "inox_serialize")]
pub enum CullingModeType {
    None,
    Back,
    Front,
}

#[derive(Serialize, Deserialize, Debug, PartialOrd, PartialEq, Copy, Clone)]
#[serde(crate = "inox_serialize")]
pub enum BlendFactor {
    Zero,
    One,
    SrcColor,
    OneMinusSrcColor,
    DstColor,
    OneMinusDstColor,
    SrcAlpha,
    OneMinusSrcAlpha,
    DstAlpha,
    OneMinusDstAlpha,
    ConstantColor,
    OneMinusConstantColor,
    ConstantAlpha,
    OneMinusConstantAlpha,
    SrcAlphaSaturate,
}

impl From<BlendFactor> for wgpu::BlendFactor {
    fn from(blend_factor: BlendFactor) -> Self {
        match blend_factor {
            BlendFactor::Zero => wgpu::BlendFactor::Zero,
            BlendFactor::One => wgpu::BlendFactor::One,
            BlendFactor::SrcColor => wgpu::BlendFactor::Src,
            BlendFactor::OneMinusSrcColor => wgpu::BlendFactor::OneMinusSrc,
            BlendFactor::DstColor => wgpu::BlendFactor::Dst,
            BlendFactor::OneMinusDstColor => wgpu::BlendFactor::OneMinusDst,
            BlendFactor::SrcAlpha => wgpu::BlendFactor::SrcAlpha,
            BlendFactor::OneMinusSrcAlpha => wgpu::BlendFactor::OneMinusSrcAlpha,
            BlendFactor::DstAlpha => wgpu::BlendFactor::DstAlpha,
            BlendFactor::OneMinusDstAlpha => wgpu::BlendFactor::OneMinusDstAlpha,
            BlendFactor::ConstantColor => wgpu::BlendFactor::Constant,
            BlendFactor::OneMinusConstantColor => wgpu::BlendFactor::OneMinusConstant,
            BlendFactor::ConstantAlpha => wgpu::BlendFactor::Constant,
            BlendFactor::OneMinusConstantAlpha => wgpu::BlendFactor::OneMinusConstant,
            BlendFactor::SrcAlphaSaturate => wgpu::BlendFactor::SrcAlphaSaturated,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialOrd, PartialEq, Copy, Clone)]
#[serde(crate = "inox_serialize")]
pub enum DrawMode {
    Batch,
    Single,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(crate = "inox_serialize")]
pub struct PipelineData {
    pub vertex_shader: PathBuf,
    pub fragment_shader: PathBuf,
    pub vertex_format: Vec<VertexFormat>,
    pub binding_data: Vec<BindingDataType>,
    pub culling: CullingModeType,
    pub mode: PolygonModeType,
    pub depth_write_enabled: bool,
    pub src_color_blend_factor: BlendFactor,
    pub dst_color_blend_factor: BlendFactor,
    pub src_alpha_blend_factor: BlendFactor,
    pub dst_alpha_blend_factor: BlendFactor,
}

impl SerializeFile for PipelineData {
    fn extension() -> &'static str {
        "pipeline"
    }
}

impl Default for PipelineData {
    fn default() -> Self {
        Self {
            vertex_shader: PathBuf::new(),
            fragment_shader: PathBuf::new(),
            vertex_format: Vec::new(),
            binding_data: Vec::new(),
            culling: CullingModeType::Back,
            mode: PolygonModeType::Fill,
            depth_write_enabled: true,
            src_color_blend_factor: BlendFactor::One,
            dst_color_blend_factor: BlendFactor::OneMinusSrcColor,
            src_alpha_blend_factor: BlendFactor::One,
            dst_alpha_blend_factor: BlendFactor::OneMinusSrcAlpha,
        }
    }
}

impl PipelineData {
    pub fn is_valid(&self) -> bool {
        !self.vertex_format.is_empty()
    }
    pub fn canonicalize_paths(mut self) -> Self {
        let data_path = Data::platform_data_folder();
        if !self.vertex_shader.to_str().unwrap().is_empty() {
            self.vertex_shader =
                convert_from_local_path(data_path.as_path(), self.vertex_shader.as_path());
        }
        if !self.fragment_shader.to_str().unwrap().is_empty() {
            self.fragment_shader =
                convert_from_local_path(data_path.as_path(), self.fragment_shader.as_path());
        }
        self
    }
    pub fn has_same_shaders(&self, other: &PipelineData) -> bool {
        self.vertex_shader == other.vertex_shader && self.fragment_shader == other.fragment_shader
    }
}

pub struct PipelineBindingData<'a> {
    pub width: u32,
    pub height: u32,
    pub view: &'a Matrix4,
    pub proj: &'a Matrix4,
    pub textures: &'a [TextureAtlas],
    pub used_textures: &'a [bool],
    pub light_data: &'a [LightData],
    pub texture_data: &'a [TextureData],
    pub material_data: &'a [ShaderMaterialData],
}
