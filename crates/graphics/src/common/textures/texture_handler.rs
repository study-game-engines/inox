use std::{num::NonZeroU32, path::Path};

use inox_profiler::debug_log;

use crate::{TextureData, TextureId, MAX_TEXTURE_ATLAS_COUNT};

use super::texture_atlas::TextureAtlas;

pub struct TextureHandler {
    texture_atlas: Vec<TextureAtlas>,
    encoder: Option<wgpu::CommandEncoder>,
    texture_data_bind_group_layout: wgpu::BindGroupLayout,
    default_sampler: wgpu::Sampler,
}

impl TextureHandler {
    pub fn create(device: &wgpu::Device) -> Self {
        let default_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let texture_atlas = vec![TextureAtlas::create_default(device)];
        let texture_data_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Textures bind group layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: NonZeroU32::new(MAX_TEXTURE_ATLAS_COUNT),
                    },
                ],
            });

        Self {
            encoder: None,
            texture_atlas,
            texture_data_bind_group_layout,
            default_sampler,
        }
    }
    fn create_encoder(&mut self, device: &wgpu::Device) -> &mut wgpu::CommandEncoder {
        if self.encoder.is_none() {
            let encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Texture Update Encoder"),
            });
            self.encoder = Some(encoder);
        }
        self.encoder.as_mut().unwrap()
    }
    pub fn send_to_gpu(&mut self, queue: &wgpu::Queue) {
        if let Some(encoder) = self.encoder.take() {
            queue.submit(std::iter::once(encoder.finish()));
        }
    }
    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.texture_data_bind_group_layout
    }
    pub fn default_sampler(&self) -> &wgpu::Sampler {
        &self.default_sampler
    }

    pub fn bind_group(
        &self,
        device: &wgpu::Device,
        render_target: Option<&TextureId>,
    ) -> wgpu::BindGroup {
        let mut textures = Vec::new();
        self.texture_atlas.iter().for_each(|texture_atlas| {
            if let Some(id) = render_target {
                if texture_atlas.texture_id() == id {
                    return;
                }
            }
            textures.push(texture_atlas.texture());
        });
        for _ in textures.len()..MAX_TEXTURE_ATLAS_COUNT as usize {
            textures.push(textures[0]);
        }

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Sampler(&self.default_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureViewArray(textures.as_slice()),
                },
            ],
            layout: &self.texture_data_bind_group_layout,
            label: Some("Textures bind group"),
        });

        bind_group
    }

    pub fn add_render_target(
        &mut self,
        device: &wgpu::Device,
        id: &TextureId,
        width: u32,
        height: u32,
    ) {
        self.texture_atlas
            .push(TextureAtlas::create_texture(device, id, width, height, 1));
    }

    pub fn get_textures_atlas(&self) -> &[TextureAtlas] {
        self.texture_atlas.as_slice()
    }

    pub fn get_texture_index(&self, id: &TextureId) -> Option<usize> {
        for (texture_index, texture_atlas) in self.texture_atlas.iter().enumerate() {
            if texture_atlas
                .get_texture_data(texture_index as _, id)
                .is_some()
            {
                return Some(texture_index);
            }
        }
        None
    }

    pub fn get_texture_atlas(&self, id: &TextureId) -> Option<&TextureAtlas> {
        if let Some(index) = self.texture_atlas.iter().position(|t| t.texture_id() == id) {
            return Some(&self.texture_atlas[index]);
        }
        None
    }

    pub fn is_empty(&self) -> bool {
        self.texture_atlas.is_empty()
    }

    pub fn copy(&self, device: &wgpu::Device, id: &TextureId, _image_data: &mut [u8]) {
        inox_profiler::scoped_profile!("texture::copy");

        self.texture_atlas.iter().for_each(|atlas| {
            if atlas.read_from_gpu(device, id) {
                todo!();
            }
        });
    }

    pub fn remove(&mut self, id: &TextureId) {
        let mut texture_to_remove = Vec::new();
        self.texture_atlas
            .iter_mut()
            .enumerate()
            .for_each(|(texture_index, atlas)| {
                if atlas.remove(texture_index as _, id) {
                    texture_to_remove.push(texture_index);
                }
            });
        texture_to_remove.iter().rev().for_each(|i| {
            let mut texture_atlas = self.texture_atlas.remove(*i);
            texture_atlas.destroy();
            debug_log!("Removing texture atlas {:?}", i);
        });
    }

    pub fn add_from_path(
        &mut self,
        device: &wgpu::Device,
        id: &TextureId,
        filepath: &Path,
    ) -> TextureData {
        let image = image::open(filepath).unwrap();
        self.add_image(
            device,
            id,
            (image.width(), image.height()),
            image.to_rgba8().as_raw().as_slice(),
        )
    }

    pub fn add_image(
        &mut self,
        device: &wgpu::Device,
        id: &TextureId,
        dimensions: (u32, u32),
        image_data: &[u8],
    ) -> TextureData {
        self.create_encoder(device);
        for (texture_index, texture_atlas) in self.texture_atlas.iter_mut().enumerate() {
            if let Some(texture_data) = texture_atlas.allocate(
                device,
                self.encoder.as_mut().unwrap(),
                id,
                texture_index as _,
                dimensions,
                image_data,
            ) {
                return texture_data;
            }
        }
        panic!("Unable to allocate texture")
    }

    pub fn get_texture_data(&self, id: &TextureId) -> Option<TextureData> {
        for (texture_index, texture_atlas) in self.texture_atlas.iter().enumerate() {
            if let Some(texture_data) = texture_atlas.get_texture_data(texture_index as _, id) {
                return Some(texture_data);
            }
        }
        None
    }
}