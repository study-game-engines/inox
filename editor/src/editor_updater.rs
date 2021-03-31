use std::time::Instant;

use super::config::*;

use nrg_commands::*;
use nrg_core::*;
use nrg_graphics::*;
use nrg_gui::*;
use nrg_platform::*;
use nrg_serialize::*;

pub struct EditorUpdater {
    id: SystemId,
    shared_data: SharedDataRw,
    config: Config,
    is_ctrl_pressed: bool,
    history: CommandsHistory,
    input_handler: InputHandler,
    fps_text_widget_id: UID,
    history_text_widget_id: UID,
    history_redo_button: UID,
    history_undo_button: UID,
    history_clear_button: UID,
    time_per_fps: f64,
    widget: Panel,
    node: GraphNode,
}

impl EditorUpdater {
    pub fn new(shared_data: &SharedDataRw, config: &Config) -> Self {
        let read_data = shared_data.read().unwrap();
        let events_rw = &mut *read_data.get_unique_resource_mut::<EventsRw>();
        Self {
            id: SystemId::new(),
            shared_data: shared_data.clone(),
            config: config.clone(),
            is_ctrl_pressed: false,
            history: CommandsHistory::new(&events_rw),
            input_handler: InputHandler::default(),
            node: GraphNode::default(),
            widget: Panel::default(),
            fps_text_widget_id: INVALID_ID,
            history_text_widget_id: INVALID_ID,
            history_redo_button: INVALID_ID,
            history_undo_button: INVALID_ID,
            history_clear_button: INVALID_ID,
            time_per_fps: 0.,
        }
    }
}

impl System for EditorUpdater {
    fn id(&self) -> SystemId {
        self.id
    }

    fn init(&mut self) {
        self.load_pipelines();

        let read_data = self.shared_data.read().unwrap();
        let renderer = &mut *read_data.get_unique_resource_mut::<Renderer>();
        let window = &*read_data.get_unique_resource::<Window>();
        let events_rw = &mut *read_data.get_unique_resource_mut::<EventsRw>();

        self.input_handler
            .init(window.get_width() as _, window.get_heigth() as _);

        Screen::create(
            window.get_width(),
            window.get_heigth(),
            window.get_scale_factor(),
            events_rw.clone(),
        );

        self.widget
            .init(renderer)
            .position([300, 300].into())
            .size([500, 800].into())
            .selectable(false)
            .vertical_alignment(VerticalAlignment::Top)
            .horizontal_alignment(HorizontalAlignment::Left)
            .fill_type(ContainerFillType::Vertical)
            .fit_to_content(true)
            .space_between_elements(20);

        let mut fps_text = Text::default();
        fps_text
            .init(renderer)
            .size([500, 20].into())
            .vertical_alignment(VerticalAlignment::Top)
            .horizontal_alignment(HorizontalAlignment::Left)
            .set_text("FPS: ");
        self.fps_text_widget_id = self.widget.add_child(Box::new(fps_text));

        let (
            history_panel,
            history_text_id,
            history_undo_button_id,
            history_redo_button_id,
            history_clear_button_id,
        ) = self.create_history_widget(renderer);
        self.widget.add_child(Box::new(history_panel));
        self.history_text_widget_id = history_text_id;
        self.history_undo_button = history_undo_button_id;
        self.history_redo_button = history_redo_button_id;
        self.history_clear_button = history_clear_button_id;

        let mut checkbox = Checkbox::default();
        checkbox
            .init(renderer)
            .horizontal_alignment(HorizontalAlignment::Left);
        self.widget.add_child(Box::new(checkbox));

        let mut editable_text = EditableText::default();
        editable_text.init(renderer);
        self.widget.add_child(Box::new(editable_text));

        self.node.init(renderer);
        /*
        let filepath = PathBuf::from(format!(
            "./data/widgets/{}.widget",
            self.node.id().to_simple().to_string()
        ));
        serialize_to_file(&self.node, filepath);
        */
        /*
        let filepath = PathBuf::from("./data/widgets/2cbbe60c59194b0983026b24dad5b69b.widget");
        deserialize_from_file(&mut self.node, filepath);
        */
    }

    fn run(&mut self) -> bool {
        let time = std::time::Instant::now();

        Screen::update();

        self.update_mouse_pos()
            .update_keyboard_input()
            .update_widgets()
            .manage_history_interactions();

        self.history.update();

        self.update_fps_counter(&time);
        true
    }
    fn uninit(&mut self) {
        let read_data = self.shared_data.read().unwrap();
        let renderer = &mut *read_data.get_unique_resource_mut::<Renderer>();

        self.node.uninit(renderer);
        self.widget.uninit(renderer);
    }
}

impl EditorUpdater {
    fn create_history_widget(&self, renderer: &mut Renderer) -> (Panel, UID, UID, UID, UID) {
        let mut history_panel = Panel::default();
        history_panel
            .init(renderer)
            .size([400, 100].into())
            .horizontal_alignment(HorizontalAlignment::Stretch)
            .selectable(false)
            .draggable(false)
            .fill_type(ContainerFillType::Vertical)
            .space_between_elements(5);

        let mut label = Text::default();
        label
            .init(renderer)
            .size([0, 16].into())
            .vertical_alignment(VerticalAlignment::Top)
            .horizontal_alignment(HorizontalAlignment::Left)
            .set_text("Command History:");
        history_panel.add_child(Box::new(label));

        let mut button_box = Panel::default();
        button_box
            .init(renderer)
            .horizontal_alignment(HorizontalAlignment::Stretch)
            .selectable(false)
            .draggable(false)
            .fit_to_content(true)
            .fill_type(ContainerFillType::Horizontal)
            .space_between_elements(10);

        let mut history_undo = Button::default();
        history_undo
            .init(renderer)
            .size([150, 100].into())
            .stroke(10);
        let mut text = Text::default();
        text.init(renderer)
            .size([0, 20].into())
            .vertical_alignment(VerticalAlignment::Center)
            .horizontal_alignment(HorizontalAlignment::Center)
            .set_text("Undo");
        history_undo.add_child(Box::new(text));

        let mut history_redo = Button::default();
        history_redo
            .init(renderer)
            .size([150, 100].into())
            .stroke(10);
        let mut text = Text::default();
        text.init(renderer)
            .size([0, 20].into())
            .vertical_alignment(VerticalAlignment::Center)
            .horizontal_alignment(HorizontalAlignment::Center)
            .set_text("Redo");
        history_redo.add_child(Box::new(text));

        let mut history_clear = Button::default();
        history_clear
            .init(renderer)
            .size([150, 100].into())
            .stroke(10);
        let mut text = Text::default();
        text.init(renderer)
            .size([0, 20].into())
            .vertical_alignment(VerticalAlignment::Center)
            .horizontal_alignment(HorizontalAlignment::Center)
            .set_text("Clear");
        history_clear.add_child(Box::new(text));

        let history_undo_button_id = button_box.add_child(Box::new(history_undo));
        let history_redo_button_id = button_box.add_child(Box::new(history_redo));
        let history_clear_button_id = button_box.add_child(Box::new(history_clear));

        history_panel.add_child(Box::new(button_box));

        let mut separator = Separator::default();
        separator.init(renderer);
        history_panel.add_child(Box::new(separator));

        let mut history_commands_box = Panel::default();
        history_commands_box
            .init(renderer)
            .size([300, 20].into())
            .horizontal_alignment(HorizontalAlignment::Stretch)
            .selectable(false)
            .draggable(false)
            .fit_to_content(true)
            .fill_type(ContainerFillType::Vertical)
            .space_between_elements(10);

        let history_text_id = history_panel.add_child(Box::new(history_commands_box));

        let mut separator = Separator::default();
        separator.init(renderer);
        history_panel.add_child(Box::new(separator));

        (
            history_panel,
            history_text_id,
            history_undo_button_id,
            history_redo_button_id,
            history_clear_button_id,
        )
    }

    fn update_history_widget(&mut self) -> &mut Self {
        if let Some(history_commands_box) = self
            .widget
            .get_data_mut()
            .node
            .get_child::<Panel>(self.history_text_widget_id)
        {
            let read_data = self.shared_data.read().unwrap();
            let renderer = &mut *read_data.get_unique_resource_mut::<Renderer>();
            history_commands_box.remove_children(renderer);
            if let Some(history_debug_commands) =
                self.history.get_undoable_commands_history_as_string()
            {
                for (index, str) in history_debug_commands.iter().enumerate() {
                    let mut text = Text::default();
                    text.init(renderer)
                        .position(
                            [
                                0,
                                20 * history_commands_box.get_data_mut().node.get_num_children()
                                    as u32,
                            ]
                            .into(),
                        )
                        .size([300, 20].into())
                        .set_text(str);
                    if index >= history_debug_commands.len() - 1 {
                        text.get_data_mut()
                            .graphics
                            .set_style(WidgetStyle::full_highlight())
                            .set_border_style(WidgetStyle::full_highlight());
                        let mut string = String::from("-> ");
                        string.push_str(str);
                        text.set_text(string.as_str());
                    }
                    history_commands_box.add_child(Box::new(text));
                }
            }
            if let Some(history_debug_commands) =
                self.history.get_redoable_commands_history_as_string()
            {
                for str in history_debug_commands.iter().rev() {
                    let mut text = Text::default();
                    text.init(renderer)
                        .position(
                            [
                                0,
                                20 * history_commands_box.get_data_mut().node.get_num_children()
                                    as u32,
                            ]
                            .into(),
                        )
                        .size([300, 20].into())
                        .set_text(str);
                    history_commands_box.add_child(Box::new(text));
                }
            }
        }
        self
    }

    fn update_fps_counter(&mut self, time: &Instant) -> &mut Self {
        if let Some(widget) = self
            .widget
            .get_data_mut()
            .node
            .get_child::<Text>(self.fps_text_widget_id)
        {
            let str = format!("FPS: {:.3}", (60. * self.time_per_fps / 0.001) as u32);
            widget.set_text(str.as_str());
        }
        self.time_per_fps = time.elapsed().as_secs_f64();
        self
    }
    fn update_widgets(&mut self) -> &mut Self {
        self.update_history_widget();

        {
            let read_data = self.shared_data.read().unwrap();
            let events = &mut *read_data.get_unique_resource_mut::<EventsRw>();
            let renderer = &mut *read_data.get_unique_resource_mut::<Renderer>();

            self.widget.update(
                Screen::get_draw_area(),
                renderer,
                events,
                &self.input_handler,
            );

            self.node.update(
                Screen::get_draw_area(),
                renderer,
                events,
                &self.input_handler,
            );
        }

        self
    }

    fn manage_history_interactions(&mut self) -> &mut Self {
        {
            let read_data = self.shared_data.read().unwrap();
            let events_rw = &mut *read_data.get_unique_resource_mut::<EventsRw>();
            let events = events_rw.read().unwrap();

            if let Some(button_events) = events.read_events::<WidgetEvent>() {
                for event in button_events.iter() {
                    if let WidgetEvent::Pressed(widget_id) = event {
                        if *widget_id == self.history_redo_button {
                            self.history.redo_last_command();
                        } else if *widget_id == self.history_undo_button {
                            self.history.undo_last_command();
                        } else if *widget_id == self.history_clear_button {
                            self.history.clear();
                        }
                    }
                }
            }
        }
        self
    }

    fn load_pipelines(&mut self) {
        let read_data = self.shared_data.read().unwrap();
        let renderer = &mut *read_data.get_unique_resource_mut::<Renderer>();

        for pipeline_data in self.config.pipelines.iter() {
            renderer.add_pipeline(pipeline_data);
        }

        let pipeline_id = renderer.get_pipeline_id("Font");
        renderer.add_font(pipeline_id, self.config.fonts.first().unwrap());
    }

    fn update_mouse_pos(&mut self) -> &mut Self {
        {
            let read_data = self.shared_data.read().unwrap();
            let window = &*read_data.get_unique_resource::<Window>();

            let window_events = window.get_events();
            self.input_handler.update(&window_events);
        }
        self
    }

    fn update_keyboard_input(&mut self) -> &mut Self {
        {
            let read_data = self.shared_data.read().unwrap();
            let events_rw = &mut *read_data.get_unique_resource_mut::<EventsRw>();
            let events = events_rw.read().unwrap();

            if let Some(key_events) = events.read_events::<KeyEvent>() {
                for event in key_events.iter() {
                    if event.code == Key::Control {
                        if event.state == InputState::Pressed
                            || event.state == InputState::JustPressed
                        {
                            self.is_ctrl_pressed = true;
                        } else if event.state == InputState::Released
                            || event.state == InputState::JustReleased
                        {
                            self.is_ctrl_pressed = false;
                        }
                    } else if self.is_ctrl_pressed
                        && event.code == Key::Z
                        && event.state == InputState::JustPressed
                    {
                        self.history.undo_last_command();
                    } else if self.is_ctrl_pressed
                        && event.code == Key::Y
                        && event.state == InputState::JustPressed
                    {
                        self.history.redo_last_command();
                    } else if event.state == InputState::JustPressed && event.code == Key::F5 {
                        println!("Launch game");
                        let result = std::process::Command::new("nrg_game_app").spawn().is_ok();
                        if !result {
                            println!("Failed to execute process");
                        }
                    }
                }
            }
        }
        self
    }
}
