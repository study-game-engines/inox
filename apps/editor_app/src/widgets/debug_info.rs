use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

use nrg_graphics::{
    FontInstance, MaterialInstance, MeshInstance, PipelineInstance, TextureInstance, ViewInstance,
};
use nrg_resources::{ResourceData, SharedData, SharedDataRw};
use nrg_scene::{Hitbox, Object, Scene, Transform};
use nrg_ui::{
    implement_widget_data, UIProperties, UIPropertiesRegistry, UIWidget, UIWidgetRc, Ui, Window,
};

struct DebugData {
    frame_seconds: VecDeque<Instant>,
    shared_data: SharedDataRw,
    ui_registry: UIPropertiesRegistry,
}
implement_widget_data!(DebugData);

pub struct DebugInfo {
    ui_page: UIWidgetRc,
}

impl DebugInfo {
    pub fn new(shared_data: &SharedDataRw) -> Self {
        let data = DebugData {
            frame_seconds: VecDeque::default(),
            shared_data: shared_data.clone(),
            ui_registry: Self::create_registry(),
        };
        Self {
            ui_page: Self::create(shared_data, data),
        }
    }

    fn create_registry() -> UIPropertiesRegistry {
        let mut ui_registry = UIPropertiesRegistry::default();

        ui_registry.register::<PipelineInstance>();
        ui_registry.register::<FontInstance>();
        ui_registry.register::<MaterialInstance>();
        ui_registry.register::<MeshInstance>();
        ui_registry.register::<TextureInstance>();
        ui_registry.register::<ViewInstance>();

        ui_registry.register::<UIWidget>();

        ui_registry.register::<Scene>();
        ui_registry.register::<Object>();
        ui_registry.register::<Transform>();
        ui_registry.register::<Hitbox>();
        ui_registry
    }

    fn create(shared_data: &SharedDataRw, data: DebugData) -> UIWidgetRc {
        UIWidget::register(shared_data, data, |ui_data, ui_context| {
            if let Some(data) = ui_data.as_any().downcast_mut::<DebugData>() {
                let now = Instant::now();
                let one_sec_before = now - Duration::from_secs(1);
                data.frame_seconds.push_back(now);
                data.frame_seconds.retain(|t| *t >= one_sec_before);

                Window::new("Stats")
                    .scroll(true)
                    .title_bar(true)
                    .resizable(true)
                    .show(ui_context, |ui| {
                        ui.label(format!("FPS: {}", data.frame_seconds.len()));
                        ui.separator();
                        Self::resource_ui_properties::<PipelineInstance>(
                            &data.shared_data,
                            &data.ui_registry,
                            ui,
                            "Pipeline",
                        );
                        Self::resource_ui_properties::<FontInstance>(
                            &data.shared_data,
                            &data.ui_registry,
                            ui,
                            "Font",
                        );
                        Self::resource_ui_properties::<MaterialInstance>(
                            &data.shared_data,
                            &data.ui_registry,
                            ui,
                            "Material",
                        );
                        Self::resource_ui_properties::<TextureInstance>(
                            &data.shared_data,
                            &data.ui_registry,
                            ui,
                            "Texture",
                        );
                        Self::resource_ui_properties::<MeshInstance>(
                            &data.shared_data,
                            &data.ui_registry,
                            ui,
                            "Mesh",
                        );
                        ui.separator();
                        Self::resource_ui_properties::<ViewInstance>(
                            &data.shared_data,
                            &data.ui_registry,
                            ui,
                            "View",
                        );
                        ui.separator();
                        Self::resource_ui_properties::<Scene>(
                            &data.shared_data,
                            &data.ui_registry,
                            ui,
                            "Scene",
                        );
                        Self::resource_ui_properties::<Object>(
                            &data.shared_data,
                            &data.ui_registry,
                            ui,
                            "Object",
                        );
                        Self::resource_ui_properties::<Transform>(
                            &data.shared_data,
                            &data.ui_registry,
                            ui,
                            "Transform",
                        );
                        Self::resource_ui_properties::<Hitbox>(
                            &data.shared_data,
                            &data.ui_registry,
                            ui,
                            "Hitbox",
                        );
                    });
            }
        })
    }

    fn resource_ui_properties<R>(
        shared_data: &SharedDataRw,
        ui_registry: &UIPropertiesRegistry,
        ui: &mut Ui,
        title: &str,
    ) where
        R: ResourceData + UIProperties,
    {
        ui.collapsing(
            format!(
                "{}: {}",
                title,
                SharedData::get_num_resources_of_type::<R>(shared_data)
            ),
            |ui| {
                let resources = SharedData::get_resources_of_type::<R>(shared_data);
                for r in resources.iter() {
                    r.resource().get_mut().show(ui_registry, ui);
                }
            },
        );
    }
}