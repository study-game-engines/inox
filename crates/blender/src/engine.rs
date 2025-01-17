use crate::exporter::Exporter;
use inox_resources::Singleton;
use inox_serialize::inox_serializable::SerializableRegistryRc;
use pyo3::{pyclass, pymethods, PyResult, Python};

use inox_binarizer::Binarizer;
use inox_core::App;
use inox_filesystem::EXE_PATH;
use inox_nodes::{LogicNodeRegistry, NodeType};
use inox_platform::PLATFORM_TYPE_PC;
use inox_resources::{DATA_FOLDER, DATA_RAW_FOLDER};

use std::{
    env,
    io::Write,
    net::{Shutdown, TcpStream},
    path::PathBuf,
    process::Command,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, RwLock,
    },
    thread::{self, JoinHandle},
};

#[derive(Default)]
struct ThreadData {
    can_continue: Arc<AtomicBool>,
    files_to_load: Vec<PathBuf>,
}

unsafe impl Send for ThreadData {}
unsafe impl Sync for ThreadData {}

#[pyclass]
pub struct INOXEngine {
    is_running: Arc<AtomicBool>,
    exporter: Exporter,
    binarizer: Binarizer<PLATFORM_TYPE_PC>,
    app: App,
    app_dir: PathBuf,
    working_dir: PathBuf,
    plugins: Vec<String>,
    thread_data: Arc<RwLock<ThreadData>>,
    process: Option<std::process::Child>,
    client_thread: Option<JoinHandle<()>>,
}

#[pymethods]
impl INOXEngine {
    #[new]
    fn new(executable_path: &str, plugins_to_load: Vec<String>) -> Self {
        let app_dir = PathBuf::from(executable_path);

        let mut working_dir = app_dir.clone();
        if working_dir.ends_with("release") || working_dir.ends_with("debug") {
            working_dir.pop();
            working_dir.pop();
            working_dir.pop();
        }
        env::set_var(EXE_PATH, app_dir.clone());
        env::set_current_dir(&working_dir).ok();

        let mut app = App::default();

        let mut binarizer = Binarizer::new(
            app.context(),
            working_dir.join(DATA_RAW_FOLDER),
            working_dir.join(DATA_FOLDER),
        );
        binarizer.stop();

        let mut plugins = Vec::new();
        plugins_to_load.iter().for_each(|plugin| {
            plugins.push(
                PathBuf::from(plugin)
                    .file_stem()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string(),
            );
            let mut plugin_path = app_dir.clone();
            plugin_path = plugin_path.join(plugin);
            app.add_dynamic_plugin(plugin_path.as_path());
        });

        Self {
            app_dir,
            working_dir,
            is_running: Arc::new(AtomicBool::new(false)),
            thread_data: Arc::new(RwLock::new(ThreadData::default())),
            process: None,
            client_thread: None,
            exporter: Exporter::default(),
            binarizer,
            app,
            plugins,
        }
    }

    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }
    pub fn start(&mut self) -> PyResult<bool> {
        println!("[Blender] INOXEngine started");

        let path = self.app_dir.join("inox_launcher.exe");

        let mut command = Command::new(path.as_path());
        self.plugins.iter().for_each(|plugin| {
            let string = "-plugin ".to_string() + plugin;
            command.arg(string);
        });
        command.arg("-plugin inox_connector");
        command.arg("-plugin inox_viewer");
        command.current_dir(self.working_dir.as_path());

        if let Ok(process) = command.spawn() {
            self.process = Some(process);
            self.is_running.store(true, Ordering::SeqCst);
        }

        if self.process.is_some() {
            let thread_data = self.thread_data.clone();
            thread_data.write().unwrap().can_continue = self.is_running.clone();

            let builder = thread::Builder::new().name("Blender client thread".to_string());
            let client_thread = builder
                .spawn(move || client_thread_execution(thread_data))
                .unwrap();
            self.client_thread = Some(client_thread);

            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn stop(&mut self) {
        println!("[Blender] INOXEngine stopped");
        self.is_running.store(false, Ordering::SeqCst);
    }

    pub fn export(
        &mut self,
        py: Python,
        file_to_export: &str,
        load_immediately: bool,
    ) -> PyResult<bool> {
        let current_dir = self.working_dir.clone();
        let scenes = self.exporter.process(
            py,
            current_dir.as_path(),
            PathBuf::from(file_to_export).as_path(),
        )?;

        self.binarizer.start();
        while !self.binarizer.is_running() {
            thread::yield_now();
        }
        self.binarizer.stop();

        if load_immediately {
            for scene_file in scenes {
                self.thread_data
                    .write()
                    .unwrap()
                    .files_to_load
                    .insert(0, scene_file);
            }
        }
        Ok(true)
    }

    pub fn register_nodes(&self, py: Python) -> PyResult<bool> {
        let data = self.app.context().shared_data();

        let registry = LogicNodeRegistry::get(data);

        registry.for_each_node(|node, serializable_registry| {
            add_node_in_blender(node, serializable_registry, py)
        });
        Ok(true)
    }
}

fn add_node_in_blender(
    node: &dyn NodeType,
    serializable_registry: &SerializableRegistryRc,
    py: Python,
) {
    let node_name = node.name();
    let category = node.category();
    let base_class = "LogicNodeBase";
    let description = node.description();
    let serialized_class = node.serialize_node(serializable_registry);

    py.import("INOX")
        .unwrap()
        .getattr("node_tree")
        .unwrap()
        .call_method1(
            "create_node_from_data",
            (
                node_name,
                base_class,
                category,
                description,
                serialized_class,
            ),
        )
        .ok();
}

fn client_thread_execution(thread_data: Arc<RwLock<ThreadData>>) {
    match TcpStream::connect("127.0.0.1:1983") {
        Ok(mut stream) => {
            println!("[Blender] Successfully connected to server in port 1983");
            let is_running = thread_data.read().unwrap().can_continue.clone();
            while is_running.load(Ordering::SeqCst) {
                let file = { thread_data.write().unwrap().files_to_load.pop() };
                if let Some(file) = file {
                    let file = file.to_str().unwrap_or_default().to_string();

                    println!("[Blender] INOXEngine sending to load {file:?}");

                    let message = format!("-load_file {file}");
                    let msg = message.as_bytes();

                    stream.write_all(msg).ok();
                }
            }
            stream
                .shutdown(Shutdown::Both)
                .expect("[Blender] Client thread shutdown call failed");
        }
        Err(e) => {
            println!("[Blender] Failed to connect: {e}");
        }
    }
}
