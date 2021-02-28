use super::screen::*;
use crate::colors::*;
use nrg_graphics::*;
use nrg_math::*;
use nrg_platform::*;
use nrg_serialize::*;

const LAYER_OFFSET: f32 = 0.001;

pub struct WidgetMargins {
    pub top: f32,
    pub left: f32,
    pub bottom: f32,
    pub right: f32,
}
impl Default for WidgetMargins {
    fn default() -> Self {
        Self {
            top: 0.0,
            left: 0.0,
            right: 0.0,
            bottom: 0.0,
        }
    }
}

impl WidgetMargins {
    pub fn top_left(&self) -> Vector2f {
        Vector2f {
            x: self.left,
            y: self.top,
        }
    }
}

pub struct WidgetState {
    pub pos: Vector2f,
    pub size: Vector2f,
    pub is_active: bool,
    pub is_draggable: bool,
    pub is_hover: bool,
    pub margins: WidgetMargins,
    pub layer: f32,
}

impl Default for WidgetState {
    fn default() -> Self {
        Self {
            pos: Vector2f::default(),
            size: Vector2f::default(),
            is_active: true,
            is_draggable: false,
            is_hover: false,
            margins: WidgetMargins::default(),
            layer: 0.0,
        }
    }
}

impl WidgetState {
    pub fn get_position(&self) -> Vector2f {
        self.pos
    }

    pub fn set_position(&mut self, pos: Vector2f) -> &mut Self {
        self.pos = pos;
        self
    }
    pub fn get_size(&self) -> Vector2f {
        self.size
    }
    pub fn set_size(&mut self, size: Vector2f) -> &mut Self {
        self.size = size;
        self
    }

    pub fn set_margins(&mut self, top: f32, left: f32, right: f32, bottom: f32) -> &mut Self {
        self.margins.top = top;
        self.margins.left = left;
        self.margins.right = right;
        self.margins.bottom = bottom;
        self
    }

    pub fn is_inside(&self, pos: Vector2f) -> bool {
        if pos.x >= self.pos.x
            && pos.x <= self.pos.x + self.size.x
            && pos.y >= self.pos.y
            && pos.y <= self.pos.y + self.size.y
        {
            return true;
        }
        false
    }
}

pub struct WidgetStyle {
    inactive_color: Vector3f,
    active_color: Vector3f,
    hover_color: Vector3f,
    dragging_color: Vector3f,
}

impl Default for WidgetStyle {
    fn default() -> Self {
        Self {
            inactive_color: COLOR_LIGHT_GRAY,
            active_color: COLOR_DARK_GRAY,
            hover_color: COLOR_LIGHT_CYAN,
            dragging_color: COLOR_LIGHT_BLUE,
        }
    }
}

pub struct WidgetGraphics {
    material_id: MaterialId,
    mesh_id: MeshId,
    mesh_data: MeshData,
    color: Vector3f,
    style: WidgetStyle,
}

impl Default for WidgetGraphics {
    fn default() -> Self {
        Self {
            material_id: INVALID_ID,
            mesh_id: INVALID_ID,
            mesh_data: MeshData::default(),
            color: Vector3f::default(),
            style: WidgetStyle::default(),
        }
    }
}

impl WidgetGraphics {
    pub fn init(&mut self, renderer: &mut Renderer, pipeline: &str) -> &mut Self {
        let pipeline_id = renderer.get_pipeline_id(pipeline);
        self.material_id = renderer.add_material(pipeline_id);
        self
    }
    pub fn set_mesh_data(&mut self, renderer: &mut Renderer, mesh_data: MeshData) -> &mut Self {
        self.mesh_data = mesh_data;
        if self.mesh_id == INVALID_ID {
            self.mesh_id = renderer.add_mesh(self.material_id, &self.mesh_data);
        }
        self
    }
    pub fn get_color(&self) -> Vector3f {
        self.color
    }
    pub fn set_color(&mut self, rgb: Vector3f) -> &mut Self {
        self.color = rgb;
        self
    }
    pub fn move_to_layer(&mut self, layer: f32) -> &mut Self {
        self.mesh_data.translate([0.0, 0.0, layer].into());
        self
    }
    pub fn is_inside(&self, pos: Vector2f) -> bool {
        let mut i = 0;
        let count = self.mesh_data.indices.len();
        while i < count {
            let v1 = self.mesh_data.vertices[self.mesh_data.indices[i] as usize].pos;
            let v2 = self.mesh_data.vertices[self.mesh_data.indices[i + 1] as usize].pos;
            let v3 = self.mesh_data.vertices[self.mesh_data.indices[i + 2] as usize].pos;
            if is_point_in_triangle(v1.into(), v2.into(), v3.into(), pos.x, pos.y) {
                return true;
            }
            i += 3;
        }
        false
    }

    pub fn update(&mut self, renderer: &mut Renderer) -> &mut Self {
        renderer.update_mesh(self.material_id, self.mesh_id, &self.mesh_data);
        self
    }

    pub fn uninit(&mut self, renderer: &mut Renderer) -> &mut Self {
        renderer.remove_mesh(self.material_id, self.mesh_id);
        renderer.remove_material(self.material_id);
        self.material_id = INVALID_ID;
        self.mesh_id = INVALID_ID;
        self.mesh_data.clear();
        self
    }
}

pub struct WidgetNode {
    id: UID,
    children: Vec<Box<dyn WidgetBase>>,
}

impl Default for WidgetNode {
    fn default() -> Self {
        Self {
            id: generate_random_uid(),
            children: Vec::new(),
        }
    }
}

impl WidgetNode {
    pub fn add_child<W: 'static + WidgetTrait>(&mut self, widget: Widget<W>) -> &mut Self {
        self.children.push(Box::new(widget));
        self
    }
    pub fn propagate_on_children<F>(&mut self, mut f: F) -> &mut Self
    where
        F: FnMut(&mut dyn WidgetBase),
    {
        self.children.iter_mut().for_each(|w| f(w.as_mut()));
        self
    }
}

pub struct WidgetData {
    pub node: WidgetNode,
    pub graphics: WidgetGraphics,
    pub state: WidgetState,
}

impl Default for WidgetData {
    fn default() -> Self {
        Self {
            node: WidgetNode::default(),
            graphics: WidgetGraphics::default(),
            state: WidgetState::default(),
        }
    }
}

pub trait WidgetBase: Send + Sync {
    fn get_screen(&self) -> Screen;
    fn get_data(&self) -> &WidgetData;
    fn get_data_mut(&mut self) -> &mut WidgetData;
    fn update(&mut self, renderer: &mut Renderer, input_handler: &InputHandler) -> bool;
    fn uninit(&mut self, renderer: &mut Renderer);
    fn id(&self) -> UID {
        self.get_data().node.id
    }
    fn translate(&mut self, offset: Vector2f) {
        let data = self.get_data_mut();
        data.state.set_position(data.state.get_position() + offset);
    }

    fn scale(&mut self, scale: Vector2f) {
        let data = self.get_data_mut();
        data.state.set_size(data.state.get_size() * scale);
    }

    fn manage_input(&mut self, input_handler: &InputHandler) -> bool {
        let screen = self.get_screen();
        let data = self.get_data_mut();
        if !data.state.is_active {
            data.graphics.set_color(data.graphics.style.inactive_color);
            return false;
        }
        let mut is_on_children = false;
        data.node.propagate_on_children(|child| {
            is_on_children |= child.is_hover();
        });
        if is_on_children {
            data.state.is_hover = false;
            data.graphics.set_color(data.graphics.style.active_color);
            return true;
        }
        let mouse = screen.convert_into_pixels(Vector2f {
            x: input_handler.get_mouse_data().get_x() as _,
            y: input_handler.get_mouse_data().get_y() as _,
        });
        data.state.is_hover = data.state.is_inside(mouse);
        if !data.state.is_hover {
            data.graphics.set_color(data.graphics.style.active_color);
            return false;
        }
        let mouse_in_screen_space = screen.convert_from_pixels_into_screen_space(mouse);
        if !data.graphics.is_inside(mouse_in_screen_space) {
            data.state.is_hover = false;
            return false;
        } else {
            data.graphics.set_color(data.graphics.style.hover_color);
        }
        if !data.state.is_draggable {
            return false;
        }
        if !input_handler.get_mouse_data().is_dragging() {
            return false;
        } else {
            data.graphics.set_color(data.graphics.style.dragging_color);
        }
        let movement = Vector2f {
            x: input_handler.get_mouse_data().movement_x() as _,
            y: input_handler.get_mouse_data().movement_y() as _,
        };
        let movement_in_pixels = screen.convert_into_pixels(movement);
        let pos = data.state.get_position() + movement_in_pixels;
        self.set_position(pos);
        true
    }
    fn move_to_layer(&mut self, layer: f32) {
        let data = self.get_data_mut();
        data.state.layer = layer;
        data.graphics.move_to_layer(layer);
    }

    fn update_layout(&mut self) {
        let data = self.get_data_mut();
        let pos = data.state.get_position();
        let layer = data.state.layer;
        data.node.propagate_on_children(|w| {
            let widget_pos = pos + w.get_data().state.margins.top_left();
            w.set_position(widget_pos);
            w.move_to_layer(layer + LAYER_OFFSET);
            w.update_layout();
        });
    }
    fn set_color(&mut self, r: f32, g: f32, b: f32);
    fn set_position(&mut self, pos: Vector2f);
    fn set_size(&mut self, size: Vector2f);
    fn set_margins(&mut self, top: f32, left: f32, right: f32, bottom: f32);
    fn set_draggable(&mut self, draggable: bool) {
        self.get_data_mut().state.is_draggable = draggable;
    }
    fn is_hover(&self) -> bool {
        self.get_data().state.is_hover
    }
    fn is_draggable(&self) -> bool {
        self.get_data().state.is_draggable
    }
}

pub struct Widget<T> {
    data: WidgetData,
    screen: Screen,
    _inner: T,
}

impl<T> WidgetBase for Widget<T>
where
    T: WidgetTrait,
{
    fn get_screen(&self) -> Screen {
        self.screen.clone()
    }

    fn get_data(&self) -> &WidgetData {
        &self.data
    }
    fn get_data_mut(&mut self) -> &mut WidgetData {
        &mut self.data
    }
    fn update(&mut self, renderer: &mut Renderer, input_handler: &InputHandler) -> bool {
        let mut input_managed = false;
        input_managed |= self.manage_input(input_handler);
        T::update::<T>(self, renderer, input_handler);
        self.data.node.propagate_on_children(|w| {
            input_managed |= w.update(renderer, input_handler);
        });
        input_managed
    }

    fn uninit(&mut self, renderer: &mut Renderer) {
        self.data.node.propagate_on_children(|w| w.uninit(renderer));
        T::uninit::<T>(self, renderer);
        self.data.graphics.uninit(renderer);
    }
    fn set_color(&mut self, r: f32, g: f32, b: f32) {
        self.color(r, g, b);
    }
    fn set_position(&mut self, pos: Vector2f) {
        self.position(pos);
    }
    fn set_size(&mut self, size: Vector2f) {
        self.size(size);
    }
    fn set_margins(&mut self, top: f32, left: f32, right: f32, bottom: f32) {
        self.margins(top, left, right, bottom);
    }
}

pub trait WidgetTrait: Send + Sync {
    fn get_type(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
    fn init<T: WidgetTrait>(widget: &mut Widget<T>, renderer: &mut Renderer);
    fn update<T: WidgetTrait>(
        widget: &mut Widget<T>,
        renderer: &mut Renderer,
        input_handler: &InputHandler,
    );
    fn uninit<T: WidgetTrait>(widget: &mut Widget<T>, renderer: &mut Renderer);
}

impl<T> Widget<T>
where
    T: WidgetTrait,
{
    pub fn new(inner: T, screen: Screen) -> Self {
        Self {
            data: WidgetData::default(),
            _inner: inner,
            screen,
        }
    }

    pub fn init(&mut self, renderer: &mut Renderer) -> &mut Self {
        T::init::<T>(self, renderer);
        self
    }

    pub fn add_child<W: 'static + WidgetTrait>(&mut self, mut widget: Widget<W>) -> UID {
        let id = widget.data.node.id;
        widget.data.state.margins.left = widget.get_position().x;
        widget.data.state.margins.top = widget.get_position().y;
        widget.set_position([0.0, 0.0].into());
        self.data.node.add_child(widget);
        self.update_layout();
        id
    }

    pub fn propagate_on_child<F>(&mut self, uid: UID, mut f: F)
    where
        F: FnMut(&mut dyn WidgetBase),
    {
        if let Some(index) = self
            .data
            .node
            .children
            .iter()
            .position(|child| child.id() == uid)
        {
            let w = &mut self.data.node.children[index as usize];
            return f(w.as_mut());
        }
    }

    pub fn get_position(&self) -> Vector2f {
        self.data.state.get_position()
    }
    pub fn margins(&mut self, top: f32, left: f32, right: f32, bottom: f32) -> &mut Self {
        self.data.state.set_margins(top, left, right, bottom);
        self.update_layout();
        self
    }

    pub fn position(&mut self, pos: Vector2f) -> &mut Self {
        let offset = pos - self.data.state.get_position();
        self.translate(offset);
        self.update_layout();
        self
    }

    pub fn size(&mut self, size: Vector2f) -> &mut Self {
        let old_screen_scale = self.screen.convert_from_pixels(self.data.state.get_size());
        let screen_size = self.screen.convert_from_pixels(size);
        let scale = screen_size / old_screen_scale;
        self.scale(scale);
        self.update_layout();
        self
    }

    pub fn color(&mut self, r: f32, g: f32, b: f32) -> &mut Self {
        self.data.graphics.set_color([r, g, b].into());
        self
    }
}
