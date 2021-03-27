use super::*;
use crate::screen::*;
use nrg_graphics::*;
use nrg_math::*;
use nrg_platform::*;
use nrg_serialize::*;

pub const DEFAULT_LAYER_OFFSET: f32 = 0.001;
pub const DEFAULT_WIDGET_SIZE: Vector2u = Vector2u { x: 10, y: 10 };

pub trait WidgetBase: Send + Sync {
    fn get_screen(&self) -> Screen;
    fn get_data(&self) -> &WidgetData;
    fn get_data_mut(&mut self) -> &mut WidgetData;
    fn update(
        &mut self,
        parent_data: Option<&WidgetState>,
        renderer: &mut Renderer,
        events: &mut EventsRw,
        input_handler: &InputHandler,
    );
    fn uninit(&mut self, renderer: &mut Renderer);
    fn id(&self) -> UID {
        self.get_data().node.get_id()
    }

    fn translate(&mut self, offset_in_px: Vector2i) {
        let data = self.get_data_mut();
        let new_pos: Vector2i = data.state.get_position().convert() + offset_in_px;
        data.state.set_position(new_pos.convert());

        data.node.propagate_on_children_mut(|w| {
            w.translate(offset_in_px);
        });
    }

    fn scale(&mut self, scale: Vector2f) {
        let data = self.get_data_mut();
        let scaled_size: Vector2u = [
            (data.state.get_size().x as f32 * scale.x) as _,
            (data.state.get_size().y as f32 * scale.y) as _,
        ]
        .into();
        data.state.set_size(scaled_size);

        data.node.propagate_on_children_mut(|w| {
            w.scale(scale);
        });
    }

    fn compute_offset_and_scale_from_alignment(&mut self) {
        let state = &self.get_data().state;
        let graphics = &self.get_data().graphics;
        let screen = &self.get_screen();

        let clip_rect = state.get_clip_area();
        let clip_min: Vector2u = [clip_rect.x, clip_rect.y].into();
        let clip_max: Vector2u = [clip_rect.z, clip_rect.w].into();

        let mut pos = state.get_position();
        let mut size = state.get_size();
        let stroke = screen.convert_size_into_pixels(graphics.get_stroke().into());

        match state.get_horizontal_alignment() {
            HorizontalAlignment::Left => {
                pos.x = clip_min.x + stroke.x;
            }
            HorizontalAlignment::Right => {
                pos.x = clip_max.x - (size.x + stroke.x);
            }
            HorizontalAlignment::Center => {
                pos.x = clip_min.x + (clip_max.x - clip_min.x) / 2 - size.x / 2;
            }
            HorizontalAlignment::Stretch => {
                pos.x = clip_min.x + stroke.x;
                size.x = (clip_max.x - clip_min.x) - stroke.x * 2;
            }
            _ => {}
        }

        match state.get_vertical_alignment() {
            VerticalAlignment::Top => {
                pos.y = clip_min.y + stroke.y;
            }
            VerticalAlignment::Bottom => {
                pos.y = clip_max.y - (size.y + stroke.y);
            }
            VerticalAlignment::Center => {
                pos.y = clip_min.y + (clip_max.y - clip_min.y) / 2 - size.y / 2;
            }
            VerticalAlignment::Stretch => {
                pos.y = clip_min.y + stroke.y;
                size.y = (clip_max.y - clip_min.y) - stroke.y * 2;
            }
            _ => {}
        }

        self.get_data_mut().state.set_position(pos);
        self.get_data_mut().state.set_size(size);
    }

    fn clip_in_area(&mut self) {
        let state = &self.get_data().state;
        let graphics = &self.get_data().graphics;
        let screen = &self.get_screen();

        let clip_rect = state.get_clip_area();
        let clip_min: Vector2u = [clip_rect.x, clip_rect.y].into();
        let clip_max: Vector2u = [clip_rect.z, clip_rect.w].into();

        let mut pos: Vector2i = state.get_position().convert();
        let size = state.get_size();
        let stroke = screen.convert_size_into_pixels(graphics.get_stroke().into());

        pos.x = pos
            .x
            .max(clip_min.x as i32 + stroke.x as i32)
            .min(clip_max.x as i32 - size.x as i32 - stroke.x as i32)
            .max(0);
        pos.y = pos
            .y
            .max(clip_min.y as i32 + stroke.y as i32)
            .min(clip_max.y as i32 - size.y as i32 - stroke.y as i32)
            .max(0);

        self.get_data_mut().state.set_position(pos.convert());
    }

    fn manage_style(&mut self) {
        let data = self.get_data_mut();

        if data.state.is_hover() {
            let (color, border_color) = data.graphics.get_colors(WidgetInteractiveState::Hover);
            data.graphics
                .set_color(color)
                .set_border_color(border_color);
        } else if data.state.is_pressed() {
            let (color, border_color) = data.graphics.get_colors(WidgetInteractiveState::Pressed);
            data.graphics
                .set_color(color)
                .set_border_color(border_color);
        } else if data.state.is_active() {
            let (color, border_color) = data.graphics.get_colors(WidgetInteractiveState::Active);
            data.graphics
                .set_color(color)
                .set_border_color(border_color);
        } else {
            let (color, border_color) = data.graphics.get_colors(WidgetInteractiveState::Inactive);
            data.graphics
                .set_color(color)
                .set_border_color(border_color);
        }
    }

    fn manage_events(&mut self, events: &mut EventsRw) {
        let id = self.id();
        let data = self.get_data_mut();
        let events = events.read().unwrap();
        if let Some(widget_events) = events.read_events::<WidgetEvent>() {
            for event in widget_events.iter() {
                match event {
                    WidgetEvent::Entering(widget_id) => {
                        if *widget_id == id && data.state.is_selectable() {
                            data.state.set_hover(true);
                        } else {
                            data.state.set_hover(false);
                        }
                    }
                    WidgetEvent::Exiting(widget_id) => {
                        if *widget_id == id && data.state.is_selectable() {
                            data.state.set_hover(false);
                            data.state.set_pressed(false);
                        }
                    }
                    WidgetEvent::Released(widget_id) => {
                        if *widget_id == id && data.state.is_selectable() {
                            data.state.set_pressed(false);
                        }
                    }
                    WidgetEvent::Pressed(widget_id) => {
                        if *widget_id == id && data.state.is_selectable() {
                            data.state.set_pressed(true);
                        } else {
                            data.state.set_pressed(false);
                        }
                    }
                    WidgetEvent::Dragging(widget_id, mouse_in_px) => {
                        if *widget_id == id && data.state.is_draggable() {
                            data.state
                                .set_horizontal_alignment(HorizontalAlignment::None);
                            data.state.set_vertical_alignment(VerticalAlignment::None);
                            let mut pos = data.state.get_position().convert() + *mouse_in_px;
                            pos = pos.max(Vector2i::default());
                            data.state.set_position(pos.convert());
                        }
                    }
                }
            }
        }
    }

    fn manage_input(&mut self, events: &mut EventsRw, input_handler: &InputHandler) {
        let id = self.id();
        let mut events = events.write().unwrap();
        let screen = self.get_screen();
        let data = self.get_data_mut();
        if !data.state.is_active() || !data.state.is_selectable() {
            return;
        }
        let mut is_on_child = false;
        data.node.propagate_on_children(|w| {
            is_on_child |= w.is_hover();
        });
        if is_on_child {
            return;
        }
        let mouse_in_px: Vector2u = screen
            .from_normalized_into_pixels(Vector2f {
                x: input_handler.get_mouse_data().get_x() as _,
                y: input_handler.get_mouse_data().get_y() as _,
            })
            .max(Vector2i::default())
            .convert();
        let is_inside =
            data.state.is_inside(mouse_in_px) && data.graphics.is_inside(mouse_in_px, &screen);
        if is_inside && !data.state.is_hover() {
            events.send_event(WidgetEvent::Entering(id));
            return;
        } else if !is_inside {
            if data.state.is_hover() {
                events.send_event(WidgetEvent::Exiting(id));
            }
            return;
        }
        let is_mouse_down = input_handler.get_mouse_data().is_pressed();
        if is_mouse_down && !data.state.is_pressed() {
            events.send_event(WidgetEvent::Pressed(id));
            return;
        } else if !is_mouse_down {
            if data.state.is_pressed() {
                events.send_event(WidgetEvent::Released(id));
            }
            return;
        }
        if data.state.is_pressed() && data.state.is_draggable() {
            let movement_in_pixels = screen.from_normalized_into_pixels(Vector2f {
                x: input_handler.get_mouse_data().movement_x() as _,
                y: input_handler.get_mouse_data().movement_y() as _,
            });
            events.send_event(WidgetEvent::Dragging(id, movement_in_pixels));
        }
    }
    fn move_to_layer(&mut self, layer: f32) {
        let data = self.get_data_mut();
        data.state.set_layer(layer);
        data.graphics.move_to_layer(layer);
    }

    fn compute_clip_area(&mut self, parent_data: Option<&WidgetState>) {
        let screen = self.get_screen();
        let current_area = self.get_data().state.get_clip_area();
        let clip_area: Vector4u = if let Some(parent_state) = parent_data {
            let parent_pos = parent_state.get_position();
            let parent_size = parent_state.get_size();
            [
                parent_pos.x,
                parent_pos.y,
                parent_pos.x + parent_size.x,
                parent_pos.y + parent_size.y,
            ]
            .into()
        } else if current_area == Vector4u::default() {
            let size = screen.get_size();
            [0, 0, size.x, size.y].into()
        } else {
            current_area
        };
        self.get_data_mut().state.set_clip_area(clip_area);
    }

    fn update_layout(&mut self, parent_data: Option<&WidgetState>) {
        self.compute_clip_area(parent_data);

        if !self.get_data().state.is_pressed() {
            self.compute_offset_and_scale_from_alignment();
        }
        self.clip_in_area();
        self.update_layers();

        let data = self.get_data_mut();
        let parent_data = Some(&data.state);
        data.node.propagate_on_children_mut(|w| {
            w.update_layout(parent_data);
        });
    }

    fn update_layers(&mut self) {
        let data = self.get_data_mut();
        let layer = data.state.get_layer();

        data.node.propagate_on_children_mut(|w| {
            w.move_to_layer(layer - DEFAULT_LAYER_OFFSET * 2.0);
            w.update_layers();
        });
    }
    fn set_stroke(&mut self, stroke: u32);
    fn set_color(&mut self, r: f32, g: f32, b: f32, a: f32);
    fn set_border_color(&mut self, r: f32, g: f32, b: f32, a: f32);
    fn set_position(&mut self, pos_in_px: Vector2u);
    fn set_size(&mut self, size_in_px: Vector2u);
    fn is_hover(&self) -> bool {
        self.get_data().state.is_hover()
    }
    fn set_draggable(&mut self, draggable: bool) {
        self.get_data_mut().state.set_draggable(draggable);
    }
    fn is_draggable(&self) -> bool {
        self.get_data().state.is_draggable()
    }
    fn set_selectable(&mut self, selectable: bool) {
        self.get_data_mut().state.set_selectable(selectable);
    }
    fn is_selectable(&self) -> bool {
        self.get_data().state.is_selectable()
    }
}