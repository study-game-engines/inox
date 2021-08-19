use std::any::{type_name, Any};

use egui::{CtxRef, Ui};
use nrg_resources::{ResourceData, ResourceId, ResourceRef, SharedData, SharedDataRw};
use nrg_serialize::generate_random_uid;

use crate::{UIProperties, UIPropertiesRegistry};

pub type UIWidgetId = ResourceId;
pub type UIWidgetRc = ResourceRef<UIWidget>;

pub trait UIWidgetData: Send + Sync + Any {
    fn as_any(&mut self) -> &mut dyn Any;
}
#[macro_export]
macro_rules! implement_widget_data {
    ($Type:ident) => {
        unsafe impl Sync for $Type {}
        unsafe impl Send for $Type {}

        impl $crate::UIWidgetData for $Type {
            #[inline]
            fn as_any(&mut self) -> &mut dyn std::any::Any {
                self
            }
        }
    };
}

pub struct UIWidget {
    id: ResourceId,
    data: Box<dyn UIWidgetData>,
    func: Box<dyn FnMut(&mut dyn UIWidgetData, &CtxRef)>,
}

unsafe impl Send for UIWidget {}
unsafe impl Sync for UIWidget {}

impl ResourceData for UIWidget {
    fn id(&self) -> ResourceId {
        self.id
    }
}

impl UIProperties for UIWidget {
    fn show(&mut self, _ui_registry: &UIPropertiesRegistry, ui: &mut Ui) {
        ui.collapsing(self.id().to_simple().to_string(), |ui| {
            let widget_name = type_name::<Self>()
                .split(':')
                .collect::<Vec<&str>>()
                .last()
                .unwrap()
                .to_string();
            ui.label(widget_name);
        });
    }
}

impl UIWidget {
    pub fn register<D, F>(shared_data: &SharedDataRw, data: D, f: F) -> UIWidgetRc
    where
        D: UIWidgetData + Sized + 'static,
        F: FnMut(&mut dyn UIWidgetData, &CtxRef) + 'static,
    {
        let ui_page = Self {
            id: generate_random_uid(),
            data: Box::new(data),
            func: Box::new(f),
        };
        SharedData::add_resource::<UIWidget>(shared_data, ui_page)
    }

    pub fn data<D>(&mut self) -> Option<&D>
    where
        D: UIWidgetData + Sized + 'static,
    {
        self.data.as_any().downcast_ref::<D>()
    }

    pub fn data_mut<D>(&mut self) -> Option<&mut D>
    where
        D: UIWidgetData + Sized + 'static,
    {
        self.data.as_any().downcast_mut::<D>()
    }

    pub fn execute(&mut self, ui_context: &CtxRef) {
        (self.func)(self.data.as_mut(), ui_context);
    }
}