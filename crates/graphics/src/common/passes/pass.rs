use inox_core::ContextRc;

use crate::RenderContext;
use downcast_rs::{impl_downcast, Downcast};

pub trait Pass: Downcast + Send + Sync + 'static {
    fn create(context: &ContextRc) -> Self
    where
        Self: Sized;
    fn init(&mut self, render_context: &mut RenderContext);
    fn update(&mut self, render_context: &RenderContext);
}
impl_downcast!(Pass);
