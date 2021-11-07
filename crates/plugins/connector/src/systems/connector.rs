use std::{
    io::Read,
    net::{SocketAddr, TcpListener, TcpStream},
    str::from_utf8,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
};

use nrg_core::System;
use nrg_messenger::{GlobalMessenger, MessengerRw};
use nrg_profiler::debug_log;
use nrg_resources::ConfigBase;
use nrg_serialize::SerializeFile;

use crate::config::Config;

const SERVER_THREAD_NAME: &str = "Server Thread";

#[derive(Default)]
struct ConnectorData {
    can_continue: Arc<AtomicBool>,
    global_messenger: MessengerRw,
    client_threads: Vec<JoinHandle<()>>,
}

pub struct Connector {
    global_messenger: MessengerRw,
    can_continue: Arc<AtomicBool>,
    host_address_and_port: String,
    server_thread: Option<JoinHandle<()>>,
}

impl Connector {
    pub fn new(global_messenger: &MessengerRw) -> Self {
        Self {
            global_messenger: global_messenger.clone(),
            can_continue: Arc::new(AtomicBool::new(false)),
            host_address_and_port: String::new(),
            server_thread: None,
        }
    }
}

impl System for Connector {
    fn read_config(&mut self, plugin_name: &str) {
        let mut config = Config::default();
        config.load_from_file(config.get_filepath(plugin_name).as_path());

        self.host_address_and_port = config.host_address + ":" + config.port.to_string().as_str();
    }
    fn should_run_when_not_focused(&self) -> bool {
        false
    }

    fn init(&mut self) {
        if self.server_thread.is_none() {
            if let Ok(tcp_listener) = TcpListener::bind(self.host_address_and_port.as_str()) {
                self.can_continue.store(true, Ordering::SeqCst);
                let mut connector_data = ConnectorData {
                    can_continue: self.can_continue.clone(),
                    global_messenger: self.global_messenger.clone(),
                    ..Default::default()
                };
                let builder = thread::Builder::new().name(SERVER_THREAD_NAME.to_string());
                let server_thread = builder
                    .spawn(move || {
                        while connector_data.can_continue.load(Ordering::SeqCst) {
                            match tcp_listener.accept() {
                                Ok((client_stream, addr)) => {
                                    let is_running = connector_data.can_continue.clone();
                                    let global_messenger = connector_data.global_messenger.clone();
                                    let thread = thread::Builder::new()
                                        .name("Reader".to_string())
                                        .spawn(move || {
                                            client_thread_execution(
                                                client_stream,
                                                addr,
                                                &global_messenger,
                                                is_running,
                                            )
                                        })
                                        .unwrap();
                                    connector_data.client_threads.push(thread);
                                }
                                Err(e) => {
                                    println!("Connection failed: {}", e);
                                }
                            }
                        }
                    })
                    .unwrap();
                self.server_thread = Some(server_thread);
            } else {
                debug_log(
                    format!(
                        "Unable to bind to requested address {:?}",
                        self.host_address_and_port,
                    )
                    .as_str(),
                );
            }
        }
    }

    fn run(&mut self) -> bool {
        true
    }
    fn uninit(&mut self) {
        self.can_continue.store(false, Ordering::SeqCst);
    }
}

fn client_thread_execution(
    mut client_stream: TcpStream,
    addr: SocketAddr,
    global_messenger: &MessengerRw,
    is_running: Arc<AtomicBool>,
) {
    println!("New Thread for client at {:?}", addr);
    let mut buffer = [0u8; 1024];
    while is_running.load(Ordering::SeqCst) {
        match client_stream.read(&mut buffer) {
            Ok(_) => {
                let last = buffer
                    .iter()
                    .rposition(|&b| b != 0u8)
                    .unwrap_or(buffer.len());
                let s = String::from(from_utf8(&buffer).unwrap_or_default());
                let s = s.split_at(last + 1).0.to_string();

                println!("[ServerThread] Received: {}", s);

                global_messenger.send_event_from_string(s);
            }
            Err(e) => {
                println!("[ServerThread] Failed to receive msg: {}", e);
            }
        }
    }
    println!("Thread for client at {:?} terminated", addr);
}