use inox_core::ContextRc;
use inox_graphics::{
    CullingEvent, DrawEvent, Light, Mesh, MeshFlags, MeshId, RendererRw,
    CONSTANT_DATA_FLAGS_DISPLAY_MESHLETS, CONSTANT_DATA_FLAGS_DISPLAY_MESHLETS_BOUNDING_BOX,
    CONSTANT_DATA_FLAGS_DISPLAY_MESHLETS_SPHERE,
};
use inox_math::{
    compute_frustum, Degrees, Frustum, Mat4Ops, MatBase, Matrix4, NewAngle, Quat, VecBase,
    VecBaseFloat, Vector3,
};
use inox_messenger::Listener;
use inox_resources::{DataTypeResourceEvent, HashBuffer, Resource, ResourceEvent};
use inox_scene::{Camera, Object, ObjectId, SceneId};
use inox_ui::{implement_widget_data, ComboBox, UIWidget, Window};
use inox_uid::INVALID_UID;

use crate::events::WidgetEvent;

use super::{Gfx, Hierarchy};

#[derive(Clone)]
struct MeshInfo {
    meshlets: Vec<MeshletInfo>,
    matrix: Matrix4,
    flags: MeshFlags,
}

impl Default for MeshInfo {
    fn default() -> Self {
        Self {
            meshlets: Vec::new(),
            matrix: Matrix4::default_identity(),
            flags: MeshFlags::None,
        }
    }
}

#[allow(dead_code)]
#[derive(Clone)]
struct MeshletInfo {
    min: Vector3,
    max: Vector3,
    center: Vector3,
    axis: Vector3,
}

impl Default for MeshletInfo {
    fn default() -> Self {
        Self {
            min: Vector3::default_zero(),
            max: Vector3::default_zero(),
            center: Vector3::default_zero(),
            axis: Vector3::default_zero(),
        }
    }
}

#[derive(Clone)]
pub struct InfoParams {
    pub is_active: bool,
    pub scene_id: SceneId,
    pub renderer: RendererRw,
}

#[derive(Default, Debug, PartialEq, Eq, Clone, Copy)]
enum MeshletDebug {
    #[default]
    None,
    Color,
    Sphere,
    BoundingBox,
    ConeAxis,
}

#[derive(Clone)]
struct Data {
    context: ContextRc,
    params: InfoParams,
    hierarchy: (bool, Option<Hierarchy>),
    graphics: (bool, Option<Gfx>),
    show_tlas: bool,
    show_blas: bool,
    show_frustum: bool,
    show_lights: bool,
    freeze_culling_camera: bool,
    meshlet_debug: MeshletDebug,
    fps: u32,
    dt: u128,
    cam_matrix: Matrix4,
    near: f32,
    far: f32,
    fov: Degrees,
    aspect_ratio: f32,
    selected_object_id: ObjectId,
}
implement_widget_data!(Data);

pub struct Info {
    ui_page: Resource<UIWidget>,
    listener: Listener,
    meshes: HashBuffer<MeshId, MeshInfo, 0>,
}

impl Info {
    pub fn new(context: &ContextRc, params: InfoParams) -> Self {
        let listener = Listener::new(context.message_hub());
        listener
            .register::<DataTypeResourceEvent<Mesh>>()
            .register::<ResourceEvent<Mesh>>()
            .register::<WidgetEvent>();
        let data = Data {
            context: context.clone(),
            params,
            hierarchy: (false, None),
            graphics: (false, None),
            show_tlas: false,
            show_blas: false,
            show_frustum: false,
            show_lights: false,
            freeze_culling_camera: false,
            meshlet_debug: MeshletDebug::None,
            fps: 0,
            dt: 0,
            cam_matrix: Matrix4::default_identity(),
            near: 0.,
            far: 0.,
            fov: Degrees::new(0.),
            aspect_ratio: 1.,
            selected_object_id: INVALID_UID,
        };
        Self {
            ui_page: Self::create(data),
            listener,
            meshes: HashBuffer::default(),
        }
    }

    pub fn is_active(&self) -> bool {
        if let Some(data) = self.ui_page.get().data::<Data>() {
            return data.params.is_active;
        }
        false
    }
    pub fn set_active(&self, is_active: bool) {
        if let Some(data) = self.ui_page.get_mut().data_mut::<Data>() {
            data.params.is_active = is_active;
        }
    }
    pub fn set_scene_id(&self, scene_id: &SceneId) {
        if let Some(data) = self.ui_page.get_mut().data_mut::<Data>() {
            data.params.scene_id = *scene_id;
        }
    }

    fn update_events(&mut self) {
        inox_profiler::scoped_profile!("Info::update_events");

        self.listener
            .process_messages(|e: &WidgetEvent| {
                let WidgetEvent::Selected(object_id) = e;
                if let Some(data) = self.ui_page.get_mut().data_mut::<Data>() {
                    data.selected_object_id = *object_id;
                }
            })
            .process_messages(|e: &DataTypeResourceEvent<Mesh>| {
                let DataTypeResourceEvent::Loaded(id, mesh_data) = e;
                let mut meshlets = Vec::new();
                mesh_data.meshlets.iter().for_each(|meshlet| {
                    meshlets.push(MeshletInfo {
                        min: meshlet.aabb_min,
                        max: meshlet.aabb_max,
                        center: meshlet.cone_center,
                        axis: meshlet.cone_axis,
                    });
                });
                self.meshes.insert(
                    id,
                    MeshInfo {
                        meshlets,
                        ..Default::default()
                    },
                );
            })
            .process_messages(|e: &ResourceEvent<Mesh>| match e {
                ResourceEvent::Changed(id) => {
                    if let Some(data) = self.ui_page.get().data::<Data>() {
                        if let Some(mesh) = data.context.shared_data().get_resource::<Mesh>(id) {
                            if let Some(m) = self.meshes.get_mut(id) {
                                m.matrix = mesh.get().matrix();
                                m.flags = *mesh.get().flags();
                            }
                        }
                    }
                }
                ResourceEvent::Destroyed(id) => {
                    self.meshes.remove(id);
                }
                _ => {}
            });
    }

    pub fn update(&mut self) {
        inox_profiler::scoped_profile!("Info::update");

        self.update_events();

        if let Some(data) = self.ui_page.get_mut().data_mut::<Data>() {
            data.fps = data.context.global_timer().fps();
            data.dt = data.context.global_timer().dt().as_millis();

            if data.hierarchy.0 && data.hierarchy.1.is_none() {
                data.hierarchy.1 = Hierarchy::new(
                    data.context.shared_data(),
                    data.context.message_hub(),
                    &data.params.scene_id,
                );
            } else if !data.hierarchy.0 && data.hierarchy.1.is_some() {
                data.hierarchy.1 = None;
            }

            if data.graphics.0 && data.graphics.1.is_none() {
                data.graphics.1 = Some(Gfx::new(&data.context, &data.params.renderer));
            } else if data.graphics.1.is_some() {
                if !data.graphics.0 {
                    data.graphics.1 = None;
                } else {
                    data.graphics.1.as_mut().unwrap().update();
                }
            }
            if data.show_lights {
                Self::show_lights(data);
            }
            if data.show_frustum {
                if !data.freeze_culling_camera {
                    if let Some(camera) = data
                        .context
                        .shared_data()
                        .match_resource(|c: &Camera| c.is_active())
                    {
                        let c = camera.get();
                        data.near = c.near_plane();
                        data.far = c.far_plane();
                        data.cam_matrix = c.transform();
                        data.fov = c.fov_in_degrees();
                        data.aspect_ratio = c.aspect_ratio();
                    }
                }
                let frustum = compute_frustum(
                    &data.cam_matrix,
                    data.near,
                    data.far,
                    data.fov,
                    data.aspect_ratio,
                );
                Self::show_frustum(data, &frustum);
            }
            if data.show_tlas {
                let renderer = data.params.renderer.read().unwrap();
                let render_context = renderer.render_context();
                let tlas = render_context.render_buffers.tlas.read().unwrap();
                tlas.for_each_data(|_i, _id, n| {
                    data.context
                        .message_hub()
                        .send_event(DrawEvent::BoundingBox(
                            n.min.into(),
                            n.max.into(),
                            [1.0, 1.0, 0.0, 1.0].into(),
                        ));
                });
            }
            if data.show_blas {
                let renderer = data.params.renderer.read().unwrap();
                let render_context = renderer.render_context();
                let bhv = render_context.render_buffers.bhv.read().unwrap();
                bhv.for_each_data(|_i_, _id, n| {
                    data.context
                        .message_hub()
                        .send_event(DrawEvent::BoundingBox(
                            n.min.into(),
                            n.max.into(),
                            [1.0, 1.0, 0.0, 1.0].into(),
                        ));
                });
            }
            if !data.selected_object_id.is_nil() {
                Self::show_meshes_of_object(data, &data.selected_object_id);
            }
            match data.meshlet_debug {
                MeshletDebug::Sphere => {
                    Self::show_meshlets_sphere(data, &self.meshes);
                }
                MeshletDebug::BoundingBox => Self::show_meshlets_bounding_box(data, &self.meshes),
                MeshletDebug::ConeAxis => Self::show_meshlets_cone_axis(data, &self.meshes),
                _ => {}
            }
        }
    }

    fn show_meshes_of_object(data: &Data, object_id: &ObjectId) {
        if let Some(object) = data.context.shared_data().get_resource::<Object>(object_id) {
            let object = object.get();
            let meshes = object.components_of_type::<Mesh>();
            if meshes.is_empty() {
                let children = object.children();
                children.iter().for_each(|o| {
                    Self::show_meshes_of_object(data, o.id());
                });
            } else {
                let renderer = data.params.renderer.read().unwrap();
                let render_context = renderer.render_context();
                let bhv = render_context.render_buffers.bhv.read().unwrap();
                meshes.iter().for_each(|mesh| {
                    if let Some(nodes) = bhv.items(mesh.id()) {
                        nodes.iter().for_each(|n| {
                            let matrix = mesh.get().matrix();
                            data.context
                                .message_hub()
                                .send_event(DrawEvent::BoundingBox(
                                    matrix.rotate_point(n.min.into()),
                                    matrix.rotate_point(n.max.into()),
                                    [1.0, 1.0, 0.0, 1.0].into(),
                                ));
                        });
                    }
                });
            }
        }
    }

    fn show_lights(data: &Data) {
        data.context
            .shared_data()
            .for_each_resource(|_, l: &Light| {
                if l.is_active() {
                    data.context.message_hub().send_event(DrawEvent::Sphere(
                        l.data().position.into(),
                        l.data().range,
                        [l.data().color[0], l.data().color[1], l.data().color[2], 1.].into(),
                        true,
                    ));
                }
            });
    }

    fn show_meshlets_sphere(data: &mut Data, meshes: &HashBuffer<MeshId, MeshInfo, 0>) {
        meshes.for_each_entry(|_id, mesh_info| {
            if mesh_info.flags.contains(MeshFlags::Visible) {
                mesh_info.meshlets.iter().for_each(|meshlet_info| {
                    let radius = ((meshlet_info.max - meshlet_info.min) * 0.5)
                        .mul(mesh_info.matrix.scale())
                        .length();
                    data.context.message_hub().send_event(DrawEvent::Circle(
                        mesh_info.matrix.rotate_point(meshlet_info.center),
                        radius,
                        [1.0, 1.0, 0.0, 1.0].into(),
                        true,
                    ));
                });
            }
        });
    }

    fn show_meshlets_bounding_box(data: &mut Data, meshes: &HashBuffer<MeshId, MeshInfo, 0>) {
        meshes.for_each_entry(|_id, mesh_info| {
            if mesh_info.flags.contains(MeshFlags::Visible) {
                mesh_info.meshlets.iter().for_each(|meshlet_info| {
                    data.context
                        .message_hub()
                        .send_event(DrawEvent::BoundingBox(
                            mesh_info.matrix.rotate_point(meshlet_info.min),
                            mesh_info.matrix.rotate_point(meshlet_info.max),
                            [1.0, 1.0, 0.0, 1.0].into(),
                        ));
                });
            }
        });
    }

    fn show_meshlets_cone_axis(data: &mut Data, meshes: &HashBuffer<MeshId, MeshInfo, 0>) {
        meshes.for_each_entry(|_id, mesh_info| {
            if mesh_info.flags.contains(MeshFlags::Visible) {
                mesh_info.meshlets.iter().for_each(|meshlet_info| {
                    let pos = mesh_info.matrix.rotate_point(meshlet_info.center);
                    data.context.message_hub().send_event(DrawEvent::Line(
                        pos,
                        pos + mesh_info
                            .matrix
                            .orientation()
                            .transform_vector(meshlet_info.axis),
                        [1.0, 1.0, 0.0, 1.0].into(),
                    ));
                });
            }
        });
    }

    fn show_frustum(data: &Data, frustum: &Frustum) {
        let color = [1., 1., 0., 1.];

        //NearPlane
        data.context.message_hub().send_event(DrawEvent::Line(
            frustum.ntr,
            frustum.ntl,
            color.into(),
        ));
        data.context.message_hub().send_event(DrawEvent::Line(
            frustum.ntr,
            frustum.nbr,
            color.into(),
        ));
        data.context.message_hub().send_event(DrawEvent::Line(
            frustum.ntl,
            frustum.nbl,
            color.into(),
        ));
        data.context.message_hub().send_event(DrawEvent::Line(
            frustum.nbr,
            frustum.nbl,
            color.into(),
        ));

        //FarPlane
        data.context.message_hub().send_event(DrawEvent::Line(
            frustum.ftr,
            frustum.ftl,
            color.into(),
        ));
        data.context.message_hub().send_event(DrawEvent::Line(
            frustum.ftr,
            frustum.fbr,
            color.into(),
        ));
        data.context.message_hub().send_event(DrawEvent::Line(
            frustum.ftl,
            frustum.fbl,
            color.into(),
        ));
        data.context.message_hub().send_event(DrawEvent::Line(
            frustum.fbr,
            frustum.fbl,
            color.into(),
        ));

        //LeftPlane
        data.context.message_hub().send_event(DrawEvent::Line(
            frustum.ftl,
            frustum.ntl,
            color.into(),
        ));
        data.context.message_hub().send_event(DrawEvent::Line(
            frustum.fbl,
            frustum.nbl,
            color.into(),
        ));

        //RightPlane
        data.context.message_hub().send_event(DrawEvent::Line(
            frustum.ftr,
            frustum.ntr,
            color.into(),
        ));
        data.context.message_hub().send_event(DrawEvent::Line(
            frustum.fbr,
            frustum.nbr,
            color.into(),
        ));
    }

    fn create(data: Data) -> Resource<UIWidget> {
        let shared_data = data.context.shared_data().clone();
        let message_hub = data.context.message_hub().clone();
        UIWidget::register(&shared_data, &message_hub, data, |ui_data, ui_context| {
            if let Some(data) = ui_data.as_any_mut().downcast_mut::<Data>() {
                if !data.params.is_active {
                    return false;
                }
                if let Some(response) = Window::new("Debug")
                    .vscroll(true)
                    .title_bar(true)
                    .resizable(true)
                    .show(ui_context, |ui| {
                        ui.label(format!("FPS: {} - ms: {:?}", data.fps, data.dt));
                        ui.checkbox(&mut data.hierarchy.0, "Hierarchy");
                        ui.checkbox(&mut data.graphics.0, "Graphics");
                        ui.checkbox(&mut data.show_lights, "Show Lights");
                        ui.checkbox(&mut data.show_tlas, "Show BHV TLAS");
                        ui.checkbox(&mut data.show_blas, "Show BHV BLAS");
                        ui.checkbox(&mut data.show_frustum, "Show Frustum");
                        let is_freezed = data.freeze_culling_camera;
                        ui.checkbox(&mut data.freeze_culling_camera, "Freeze Culling Camera");
                        if is_freezed != data.freeze_culling_camera {
                            if data.freeze_culling_camera {
                                data.context.message_hub().send_event(CullingEvent::FreezeCamera);
                            } else {
                                data.context.message_hub().send_event(CullingEvent::UnfreezeCamera);
                            }
                        }
                        ui.horizontal(|ui| {
                            ui.label("Show Meshlets");
                            let combo_box = ComboBox::from_id_source("Meshlet Debug")
                                .selected_text(format!("{:?}", data.meshlet_debug))
                                .show_ui(ui, |ui| {
                                    let mut is_changed = false;
                                    is_changed |= ui
                                        .selectable_value(
                                            &mut data.meshlet_debug,
                                            MeshletDebug::None,
                                            "None",
                                        )
                                        .changed();
                                    is_changed |= ui
                                        .selectable_value(
                                            &mut data.meshlet_debug,
                                            MeshletDebug::Color,
                                            "Color",
                                        )
                                        .changed();
                                    is_changed |= ui
                                        .selectable_value(
                                            &mut data.meshlet_debug,
                                            MeshletDebug::Sphere,
                                            "Sphere",
                                        )
                                        .changed();
                                    is_changed |= ui
                                        .selectable_value(
                                            &mut data.meshlet_debug,
                                            MeshletDebug::BoundingBox,
                                            "Bounding Box",
                                        )
                                        .changed();
                                        is_changed |= ui
                                            .selectable_value(
                                                &mut data.meshlet_debug,
                                                MeshletDebug::ConeAxis,
                                                "Cone Axis",
                                            )
                                            .changed();
                                    is_changed
                                });
                            if let Some(is_changed) = combo_box.inner {
                                if is_changed {

                                    let renderer = data.params.renderer.read().unwrap();
                                    let render_context = renderer.render_context();
                                    match &data.meshlet_debug {
                                        MeshletDebug::None => {
                                            render_context
                                                .constant_data
                                                .write()
                                                .unwrap()
                                                .remove_flag(CONSTANT_DATA_FLAGS_DISPLAY_MESHLETS)
                                                .remove_flag(
                                                    CONSTANT_DATA_FLAGS_DISPLAY_MESHLETS_SPHERE,
                                                )
                                                .remove_flag(
                                                    CONSTANT_DATA_FLAGS_DISPLAY_MESHLETS_BOUNDING_BOX,
                                                );
                                        }
                                        MeshletDebug::Color | MeshletDebug::ConeAxis => {
                                            render_context
                                                .constant_data
                                                .write()
                                                .unwrap()
                                                .remove_flag(
                                                    CONSTANT_DATA_FLAGS_DISPLAY_MESHLETS_SPHERE,
                                                )
                                                .remove_flag(
                                                    CONSTANT_DATA_FLAGS_DISPLAY_MESHLETS_BOUNDING_BOX,
                                                )
                                                .add_flag(CONSTANT_DATA_FLAGS_DISPLAY_MESHLETS);
                                        }
                                        MeshletDebug::Sphere => {
                                            render_context
                                                .constant_data
                                                .write()
                                                .unwrap()
                                                .remove_flag(
                                                    CONSTANT_DATA_FLAGS_DISPLAY_MESHLETS_BOUNDING_BOX,
                                                )
                                                .add_flag(CONSTANT_DATA_FLAGS_DISPLAY_MESHLETS)
                                                .add_flag(CONSTANT_DATA_FLAGS_DISPLAY_MESHLETS_SPHERE);
                                        }
                                        MeshletDebug::BoundingBox => {
                                            render_context
                                                .constant_data
                                                .write()
                                                .unwrap()
                                                .remove_flag(
                                                    CONSTANT_DATA_FLAGS_DISPLAY_MESHLETS_SPHERE,
                                                )
                                                .add_flag(CONSTANT_DATA_FLAGS_DISPLAY_MESHLETS)
                                                .add_flag(
                                                    CONSTANT_DATA_FLAGS_DISPLAY_MESHLETS_BOUNDING_BOX,
                                                );
                                        }
                                    }
                                }
                            }
                        });
                    })
                {
                    return response.response.is_pointer_button_down_on();
                }
            }
            false
        })
    }
}
