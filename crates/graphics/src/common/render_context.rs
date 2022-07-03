use std::{
    collections::HashMap,
    sync::{Arc, RwLock, RwLockReadGuard},
};

use inox_math::{Matrix4, Vector2};
use inox_platform::Handle;

use crate::{
    generate_buffer_id,
    platform::{platform_limits, required_gpu_features},
    AsBinding, BufferId, ConstantData, GpuBuffer, MeshFlags, RenderBuffers, RenderPass, RendererRw,
    TextureHandler, TextureId, TextureInfo, CONSTANT_DATA_FLAGS_SUPPORT_SRGB, DEFAULT_HEIGHT,
    DEFAULT_WIDTH,
};

#[derive(Default)]
pub struct BindingDataBuffer {
    pub buffers: RwLock<HashMap<BufferId, GpuBuffer>>,
}

impl BindingDataBuffer {
    pub fn has_buffer(&self, uid: &BufferId) -> bool {
        self.buffers.read().unwrap().contains_key(uid)
    }
    pub fn bind_buffer<T>(
        &self,
        id: BufferId,
        data: &mut T,
        usage: wgpu::BufferUsages,
        render_core_context: &RenderCoreContext,
    ) -> (bool, BufferId)
    where
        T: AsBinding,
    {
        let mut bind_data_buffer = self.buffers.write().unwrap();
        let buffer = bind_data_buffer
            .entry(id)
            .or_insert_with(GpuBuffer::default);
        let mut is_changed = false;
        if data.is_dirty() {
            let typename = std::any::type_name::<T>();
            let label = format!("{}[{}]", typename, id);
            is_changed |= buffer.init(render_core_context, data.size(), usage, label.as_str());
            data.fill_buffer(render_core_context, buffer);
            data.set_dirty(false);
        }
        let buffer_id = if let Some(gpu_buffer) = buffer.gpu_buffer() {
            generate_buffer_id(gpu_buffer)
        } else {
            id
        };
        (is_changed, buffer_id)
    }
}

pub struct CommandBuffer {
    pub encoder: wgpu::CommandEncoder,
}

pub struct RenderCoreContext {
    pub instance: wgpu::Instance,
    pub surface: wgpu::Surface,
    pub config: wgpu::SurfaceConfiguration,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

impl RenderCoreContext {
    pub fn new_command_buffer(&self) -> CommandBuffer {
        CommandBuffer {
            encoder: self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Command Encoder"),
                }),
        }
    }
    pub fn submit(&self, command_buffer: CommandBuffer) {
        let command_buffer = command_buffer.encoder.finish();
        self.queue.submit(std::iter::once(command_buffer));
    }
    pub fn set_surface_size(&mut self, width: u32, height: u32) {
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
        inox_log::debug_log!("Surface size: {}x{}", width, height);
    }
}

pub struct RenderContext {
    pub core: RenderCoreContext,
    pub frame_commands: Option<wgpu::CommandEncoder>,
    pub surface_texture: Option<wgpu::SurfaceTexture>,
    pub surface_view: Option<wgpu::TextureView>,
    pub constant_data: ConstantData,
    pub texture_handler: TextureHandler,
    pub binding_data_buffer: BindingDataBuffer,
    pub render_buffers: RenderBuffers,
}

pub type RenderContextRw = Arc<RwLock<RenderContext>>;

impl RenderContext {
    pub async fn create_render_context(handle: Handle, renderer: RendererRw) {
        inox_profiler::scoped_profile!("render_context::create_render_context");

        let backend = wgpu::Backends::all();
        let instance = wgpu::Instance::new(backend);
        let surface = unsafe { instance.create_surface(&handle) };

        let adapter =
            wgpu::util::initialize_adapter_from_env_or_default(&instance, backend, Some(&surface))
                .await
                .expect("No suitable GPU adapters found on the system!");
        let required_features = required_gpu_features();
        let limits = platform_limits();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: required_features,
                    limits,
                },
                // Some(&std::path::Path::new("trace")), // Trace path
                None,
            )
            .await
            .expect("Failed to create device");

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: *surface.get_supported_formats(&adapter).first().unwrap(),
            width: DEFAULT_WIDTH,
            height: DEFAULT_HEIGHT,
            present_mode: wgpu::PresentMode::AutoVsync,
        };

        //debug_log!("Surface format: {:?}", config.format);
        surface.configure(&device, &config);

        let render_core_context = RenderCoreContext {
            instance,
            surface,
            config,
            adapter,
            device,
            queue,
        };

        renderer
            .write()
            .unwrap()
            .set_render_context(Arc::new(RwLock::new(RenderContext {
                texture_handler: TextureHandler::create(&render_core_context.device),
                core: render_core_context,
                frame_commands: None,
                surface_texture: None,
                surface_view: None,
                constant_data: ConstantData::default(),
                binding_data_buffer: BindingDataBuffer::default(),
                render_buffers: RenderBuffers::default(),
            })));
    }

    pub fn update_constant_data(&mut self, view: Matrix4, proj: Matrix4, screen_size: Vector2) {
        inox_profiler::scoped_profile!("render_context::update_constant_data");
        self.constant_data.update(view, proj, screen_size);
        if self.core.config.format.describe().srgb {
            self.constant_data
                .add_flag(CONSTANT_DATA_FLAGS_SUPPORT_SRGB);
        } else {
            self.constant_data
                .remove_flag(CONSTANT_DATA_FLAGS_SUPPORT_SRGB);
        }
    }

    pub fn has_instances(&self, flags: MeshFlags) -> bool {
        if let Some(instances) = self.render_buffers.instances.get(&flags) {
            return !instances.is_empty();
        }
        false
    }

    pub fn resolution(&self) -> (u32, u32) {
        (self.core.config.width, self.core.config.height)
    }

    pub fn buffers(&self) -> RwLockReadGuard<HashMap<BufferId, GpuBuffer>> {
        self.binding_data_buffer.buffers.read().unwrap()
    }

    pub fn render_targets<'a>(&'a self, render_pass: &'a RenderPass) -> Vec<&'a wgpu::TextureView> {
        let mut render_targets = Vec::new();
        let render_textures = render_pass.render_textures_id();
        if render_textures.is_empty() {
            debug_assert!(self.surface_view.is_some());
            render_targets.push(self.surface_view.as_ref().unwrap());
        } else {
            render_textures.iter().for_each(|&id| {
                if let Some(atlas) = self.texture_handler.get_texture_atlas(id) {
                    render_targets.push(atlas.texture());
                }
            });
        }
        render_targets
    }

    pub fn depth_target<'a>(
        &'a self,
        render_pass: &'a RenderPass,
    ) -> Option<&'a wgpu::TextureView> {
        if let Some(texture) = render_pass.depth_texture() {
            if let Some(atlas) = self.texture_handler.get_texture_atlas(texture.id()) {
                return Some(atlas.texture());
            }
        }
        None
    }

    pub fn render_formats(&self, render_pass: &RenderPass) -> Vec<&wgpu::TextureFormat> {
        let mut render_formats = Vec::new();
        let render_textures = render_pass.render_textures_id();
        if render_textures.is_empty() {
            render_formats.push(&self.core.config.format);
        } else {
            render_textures.iter().for_each(|&id| {
                if let Some(atlas) = self.texture_handler.get_texture_atlas(id) {
                    render_formats.push(atlas.texture_format());
                }
            });
        }
        render_formats
    }

    pub fn depth_format(&self, render_pass: &RenderPass) -> Option<&wgpu::TextureFormat> {
        if let Some(texture) = render_pass.depth_texture() {
            self.texture_handler
                .get_texture_atlas(texture.id())
                .map(|atlas| atlas.texture_format())
        } else {
            None
        }
    }

    pub fn add_image(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        texture_id: &TextureId,
        dimensions: (u32, u32),
        image_data: &[u8],
    ) -> TextureInfo {
        self.texture_handler.add_image(
            &self.core.device,
            encoder,
            texture_id,
            dimensions,
            image_data,
        )
    }
}
