use std::{
    any::TypeId,
    collections::{hash_map::Entry, HashMap},
};

use egui::{
    ClippedMesh, CtxRef, Event, Modifiers, Output, PointerButton, RawInput, Rect,
    TextureId as eguiTextureId,
};
use image::{DynamicImage, Pixel};
use nrg_core::{JobHandlerRw, System, SystemId};
use nrg_graphics::{
    Material, Mesh, MeshCategoryId, MeshData, RenderPass, Texture, TextureId, VertexData,
};

use nrg_math::Vector4;
use nrg_messenger::{read_messages, MessageChannel, MessengerRw};
use nrg_platform::{
    InputState, KeyEvent, KeyTextEvent, MouseButton, MouseEvent, MouseState, WindowEvent,
    DEFAULT_DPI,
};
use nrg_resources::{DataTypeResource, Handle, Resource, ResourceData, SharedData, SharedDataRw};

use crate::UIWidget;

const UI_MESH_CATEGORY_IDENTIFIER: &str = "ui_mesh";

pub struct UISystem {
    id: SystemId,
    shared_data: SharedDataRw,
    job_handler: JobHandlerRw,
    global_messenger: MessengerRw,
    message_channel: MessageChannel,
    ui_context: CtxRef,
    ui_texture_version: u64,
    ui_texture: Handle<Texture>,
    ui_input: RawInput,
    ui_input_modifiers: Modifiers,
    ui_clipboard: Option<String>,
    ui_materials: HashMap<TextureId, Resource<Material>>,
    ui_meshes: Vec<Resource<Mesh>>,
    ui_scale: f32,
}

impl UISystem {
    pub fn new(
        shared_data: SharedDataRw,
        global_messenger: MessengerRw,
        job_handler: JobHandlerRw,
    ) -> Self {
        let message_channel = MessageChannel::default();

        crate::register_resource_types(&shared_data);

        Self {
            id: SystemId::new(),
            shared_data,
            job_handler,
            global_messenger,
            message_channel,
            ui_context: CtxRef::default(),
            ui_texture_version: 0,
            ui_texture: None,
            ui_input: RawInput::default(),
            ui_input_modifiers: Modifiers::default(),
            ui_clipboard: None,
            ui_materials: HashMap::new(),
            ui_meshes: Vec::new(),
            ui_scale: 2.,
        }
    }

    fn get_ui_material(&mut self, texture: Resource<Texture>) -> Resource<Material> {
        nrg_profiler::scoped_profile!("ui_system::get_ui_material");
        match self.ui_materials.entry(texture.id()) {
            Entry::Occupied(e) => e.get().clone(),
            Entry::Vacant(e) => {
                if let Some(render_pass) =
                    SharedData::match_resource(&self.shared_data, |r: &RenderPass| {
                        r.data().name == "UIPass"
                    })
                {
                    render_pass
                        .get_mut()
                        .add_category_to_draw(MeshCategoryId::new(UI_MESH_CATEGORY_IDENTIFIER));
                    if let Some(pipeline) = render_pass.get().pipeline() {
                        let material = Material::create_from_pipeline(&self.shared_data, pipeline);
                        material.get_mut().add_texture(texture);
                        e.insert(material.clone());
                        return material;
                    }
                    panic!("No pipeline inside UIPass has been loaded");
                }
                panic!("No UIPass has been loaded");
            }
        }
    }

    fn update_egui_texture(&mut self) -> &mut Self {
        nrg_profiler::scoped_profile!("ui_system::update_egui_texture");
        if self.ui_texture_version != self.ui_context.texture().version {
            let image = DynamicImage::new_rgba8(
                self.ui_context.texture().width as _,
                self.ui_context.texture().height as _,
            );
            let mut image_data = image.to_rgba8();
            let (width, height) = image_data.dimensions();
            for x in 0..width {
                for y in 0..height {
                    let r = self.ui_context.texture().pixels[(x + y * width) as usize];
                    image_data.put_pixel(x, y, Pixel::from_channels(r, r, r, r));
                }
            }
            if let Some(texture) = &self.ui_texture {
                if let Some(material) = self.ui_materials.remove(&texture.id()) {
                    material.get_mut().remove_texture(texture.id());
                }
            }
            let texture = Texture::create_from_data(&self.shared_data, image_data);
            self.ui_texture = Some(texture);
            self.ui_texture_version = self.ui_context.texture().version;
        }
        self
    }

    fn compute_mesh_data(&mut self, clipped_meshes: Vec<ClippedMesh>) {
        nrg_profiler::scoped_profile!("ui_system::compute_mesh_data");
        let shared_data = self.shared_data.clone();
        self.ui_meshes.resize_with(clipped_meshes.len(), || {
            Mesh::create_from_data(&shared_data, MeshData::new(UI_MESH_CATEGORY_IDENTIFIER))
        });

        for (i, clipped_mesh) in clipped_meshes.into_iter().enumerate() {
            let ClippedMesh(clip_rect, mesh) = clipped_mesh;
            let draw_index = i as u32;
            self.ui_meshes[i].get_mut().set_draw_index(draw_index);
            if mesh.vertices.is_empty() || mesh.indices.is_empty() {
                continue;
            }
            let texture = match mesh.texture_id {
                eguiTextureId::Egui => self.ui_texture.as_ref().unwrap().clone(),
                eguiTextureId::User(texture_index) => {
                    SharedData::get_resource_from_index::<Texture>(
                        &self.shared_data,
                        texture_index as usize,
                    )
                }
            };
            let material = self.get_ui_material(texture);
            let mesh_instance = self.ui_meshes[i].clone();
            let ui_scale = self.ui_scale;
            let job_name = format!("ui_system::compute_mesh_data[{}]", i);
            self.job_handler
                .write()
                .unwrap()
                .add_job(job_name.as_str(), move || {
                    let mut mesh_data = MeshData::new(UI_MESH_CATEGORY_IDENTIFIER);
                    let mut vertices: Vec<VertexData> = Vec::new();
                    vertices.resize(mesh.vertices.len(), VertexData::default());
                    for (i, v) in mesh.vertices.iter().enumerate() {
                        vertices[i].pos =
                            [v.pos.x * ui_scale, v.pos.y * ui_scale, draw_index as _].into();
                        vertices[i].tex_coord = [v.uv.x, v.uv.y].into();
                        vertices[i].color = [
                            v.color.r() as f32 / 255.,
                            v.color.g() as f32 / 255.,
                            v.color.b() as f32 / 255.,
                            v.color.a() as f32 / 255.,
                        ]
                        .into();
                    }
                    mesh_data.append_mesh(vertices.as_slice(), mesh.indices.as_slice());
                    mesh_instance
                        .get_mut()
                        .set_material(material)
                        .set_mesh_data(mesh_data)
                        .set_draw_area(Vector4::new(
                            clip_rect.min.x * ui_scale,
                            clip_rect.min.y * ui_scale,
                            clip_rect.max.x * ui_scale,
                            clip_rect.max.y * ui_scale,
                        ));
                });
        }
    }

    fn update_events(&mut self) -> &mut Self {
        self.ui_input.events.clear();
        read_messages(self.message_channel.get_listener(), |msg| {
            if msg.type_id() == TypeId::of::<MouseEvent>() {
                let event = msg.as_any().downcast_ref::<MouseEvent>().unwrap();
                if event.state == MouseState::Move {
                    self.ui_input.events.push(Event::PointerMoved(
                        [
                            event.x as f32 / self.ui_scale,
                            event.y as f32 / self.ui_scale,
                        ]
                        .into(),
                    ));
                } else if event.state == MouseState::Down || event.state == MouseState::Up {
                    self.ui_input.events.push(Event::PointerButton {
                        pos: [
                            event.x as f32 / self.ui_scale,
                            event.y as f32 / self.ui_scale,
                        ]
                        .into(),
                        button: match event.button {
                            MouseButton::Right => PointerButton::Secondary,
                            MouseButton::Middle => PointerButton::Middle,
                            _ => PointerButton::Primary,
                        },
                        pressed: event.state == MouseState::Down,
                        modifiers: self.ui_input_modifiers,
                    });
                }
            } else if msg.type_id() == TypeId::of::<WindowEvent>() {
                let event = msg.as_any().downcast_ref::<WindowEvent>().unwrap();
                match *event {
                    WindowEvent::SizeChanged(width, height) => {
                        self.ui_input.screen_rect = Some(Rect::from_min_size(
                            Default::default(),
                            [width as f32 / self.ui_scale, height as f32 / self.ui_scale].into(),
                        ));
                    }
                    WindowEvent::DpiChanged(x, _y) => {
                        self.ui_input.pixels_per_point = Some(x / DEFAULT_DPI);
                    }
                    _ => {}
                }
            } else if msg.type_id() == TypeId::of::<KeyEvent>() {
                let event = msg.as_any().downcast_ref::<KeyEvent>().unwrap();
                let just_pressed = event.state == InputState::JustPressed;
                let pressed = just_pressed || event.state == InputState::Pressed;

                if let Some(key) = convert_key(event.code) {
                    self.ui_input.events.push(Event::Key {
                        key,
                        pressed,
                        modifiers: self.ui_input_modifiers,
                    });
                }

                if event.code == nrg_platform::Key::Shift {
                    self.ui_input_modifiers.shift = pressed;
                } else if event.code == nrg_platform::Key::Control {
                    self.ui_input_modifiers.ctrl = pressed;
                    self.ui_input_modifiers.command = pressed;
                } else if event.code == nrg_platform::Key::Alt {
                    self.ui_input_modifiers.alt = pressed;
                } else if event.code == nrg_platform::Key::Meta {
                    self.ui_input_modifiers.command = pressed;
                    self.ui_input_modifiers.mac_cmd = pressed;
                }

                if just_pressed
                    && self.ui_input_modifiers.ctrl
                    && event.code == nrg_platform::input::Key::C
                {
                    self.ui_input.events.push(Event::Copy);
                } else if just_pressed
                    && self.ui_input_modifiers.ctrl
                    && event.code == nrg_platform::input::Key::X
                {
                    self.ui_input.events.push(Event::Cut);
                } else if just_pressed
                    && self.ui_input_modifiers.ctrl
                    && event.code == nrg_platform::input::Key::V
                {
                    if let Some(content) = &self.ui_clipboard {
                        self.ui_input.events.push(Event::Text(content.clone()));
                    }
                }
            } else if msg.type_id() == TypeId::of::<KeyTextEvent>() {
                let event = msg.as_any().downcast_ref::<KeyTextEvent>().unwrap();
                if event.char.is_ascii_control() {
                    return;
                }
                self.ui_input
                    .events
                    .push(Event::Text(event.char.to_string()));
            }
        });
        self
    }

    fn show_ui(&mut self, use_multithreading: bool) {
        nrg_profiler::scoped_profile!("ui_system::show_ui");
        SharedData::for_each_resource(&self.shared_data, |widget: &Resource<UIWidget>| {
            if use_multithreading {
                let context = self.ui_context.clone();
                let widget = widget.clone();
                let job_name = format!("ui_system::show_ui[{:?}]", widget.id());
                self.job_handler
                    .write()
                    .unwrap()
                    .add_job(job_name.as_str(), move || {
                        widget.get_mut().execute(&context);
                    });
            } else {
                widget.get_mut().execute(&self.ui_context);
            }
        });
    }

    fn handle_output(&mut self, output: Output) -> &mut Self {
        if let Some(open) = output.open_url {
            println!("Trying to open url: {:?}", open.url);
        }

        if !output.copied_text.is_empty() {
            self.ui_clipboard = Some(output.copied_text);
        }

        self
    }
}

impl Drop for UISystem {
    fn drop(&mut self) {
        crate::unregister_resource_types(&self.shared_data);
    }
}

impl System for UISystem {
    fn id(&self) -> nrg_core::SystemId {
        self.id
    }

    fn should_run_when_not_focused(&self) -> bool {
        false
    }
    fn init(&mut self) {
        self.global_messenger
            .write()
            .unwrap()
            .register_messagebox::<WindowEvent>(self.message_channel.get_messagebox())
            .register_messagebox::<KeyEvent>(self.message_channel.get_messagebox())
            .register_messagebox::<KeyTextEvent>(self.message_channel.get_messagebox())
            .register_messagebox::<MouseEvent>(self.message_channel.get_messagebox());
    }

    fn run(&mut self) -> bool {
        self.update_events();

        {
            nrg_profiler::scoped_profile!("ui_context::begin_frame");
            self.ui_context.begin_frame(self.ui_input.take());
        }

        self.show_ui(false);

        let (output, shapes) = {
            nrg_profiler::scoped_profile!("ui_context::end_frame");
            self.ui_context.end_frame()
        };
        let clipped_meshes = {
            nrg_profiler::scoped_profile!("ui_context::tessellate");
            self.ui_context.tessellate(shapes)
        };
        self.handle_output(output)
            .update_egui_texture()
            .compute_mesh_data(clipped_meshes);

        true
    }

    fn uninit(&mut self) {
        self.global_messenger
            .write()
            .unwrap()
            .unregister_messagebox::<MouseEvent>(self.message_channel.get_messagebox())
            .unregister_messagebox::<KeyTextEvent>(self.message_channel.get_messagebox())
            .unregister_messagebox::<KeyEvent>(self.message_channel.get_messagebox())
            .unregister_messagebox::<WindowEvent>(self.message_channel.get_messagebox());
    }
}

fn convert_key(key: nrg_platform::input::Key) -> Option<egui::Key> {
    match key {
        nrg_platform::Key::ArrowDown => Some(egui::Key::ArrowDown),
        nrg_platform::Key::ArrowLeft => Some(egui::Key::ArrowLeft),
        nrg_platform::Key::ArrowRight => Some(egui::Key::ArrowRight),
        nrg_platform::Key::ArrowUp => Some(egui::Key::ArrowUp),
        nrg_platform::Key::Escape => Some(egui::Key::Escape),
        nrg_platform::Key::Tab => Some(egui::Key::Tab),
        nrg_platform::Key::Backspace => Some(egui::Key::Backspace),
        nrg_platform::Key::Enter => Some(egui::Key::Enter),
        nrg_platform::Key::Space => Some(egui::Key::Space),
        nrg_platform::Key::Insert => Some(egui::Key::Insert),
        nrg_platform::Key::Delete => Some(egui::Key::Delete),
        nrg_platform::Key::Home => Some(egui::Key::Home),
        nrg_platform::Key::End => Some(egui::Key::End),
        nrg_platform::Key::PageUp => Some(egui::Key::PageUp),
        nrg_platform::Key::PageDown => Some(egui::Key::PageDown),
        nrg_platform::Key::Numpad0 | nrg_platform::Key::Key0 => Some(egui::Key::Num0),
        nrg_platform::Key::Numpad1 | nrg_platform::Key::Key1 => Some(egui::Key::Num1),
        nrg_platform::Key::Numpad2 | nrg_platform::Key::Key2 => Some(egui::Key::Num2),
        nrg_platform::Key::Numpad3 | nrg_platform::Key::Key3 => Some(egui::Key::Num3),
        nrg_platform::Key::Numpad4 | nrg_platform::Key::Key4 => Some(egui::Key::Num4),
        nrg_platform::Key::Numpad5 | nrg_platform::Key::Key5 => Some(egui::Key::Num5),
        nrg_platform::Key::Numpad6 | nrg_platform::Key::Key6 => Some(egui::Key::Num6),
        nrg_platform::Key::Numpad7 | nrg_platform::Key::Key7 => Some(egui::Key::Num7),
        nrg_platform::Key::Numpad8 | nrg_platform::Key::Key8 => Some(egui::Key::Num8),
        nrg_platform::Key::Numpad9 | nrg_platform::Key::Key9 => Some(egui::Key::Num9),
        nrg_platform::Key::A => Some(egui::Key::A),
        nrg_platform::Key::B => Some(egui::Key::B),
        nrg_platform::Key::C => Some(egui::Key::C),
        nrg_platform::Key::D => Some(egui::Key::D),
        nrg_platform::Key::E => Some(egui::Key::E),
        nrg_platform::Key::F => Some(egui::Key::F),
        nrg_platform::Key::G => Some(egui::Key::G),
        nrg_platform::Key::H => Some(egui::Key::H),
        nrg_platform::Key::I => Some(egui::Key::I),
        nrg_platform::Key::J => Some(egui::Key::J),
        nrg_platform::Key::K => Some(egui::Key::K),
        nrg_platform::Key::L => Some(egui::Key::L),
        nrg_platform::Key::M => Some(egui::Key::M),
        nrg_platform::Key::N => Some(egui::Key::N),
        nrg_platform::Key::O => Some(egui::Key::O),
        nrg_platform::Key::P => Some(egui::Key::P),
        nrg_platform::Key::Q => Some(egui::Key::Q),
        nrg_platform::Key::R => Some(egui::Key::R),
        nrg_platform::Key::S => Some(egui::Key::S),
        nrg_platform::Key::T => Some(egui::Key::T),
        nrg_platform::Key::U => Some(egui::Key::U),
        nrg_platform::Key::V => Some(egui::Key::V),
        nrg_platform::Key::W => Some(egui::Key::W),
        nrg_platform::Key::X => Some(egui::Key::X),
        nrg_platform::Key::Y => Some(egui::Key::Y),
        nrg_platform::Key::Z => Some(egui::Key::Z),
        _ => None,
    }
}
