use eframe::egui;
use neon_hud_lib::native_api::NativeBackend;
use serde_json::Value;
use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver, SyncSender},
    thread,
};

pub enum Command {
    System(String, u64),
    Providers(String),
    Save(Value, u64),
    Connect(bool),
    Disconnect,
    Claude(bool),
    Dismiss(String),
    Stop,
}
pub enum Event {
    Loaded(Result<Value, String>),
    System(Result<Value, String>),
    Providers(Result<Value, String>),
    Saved(u64, Result<(), String>),
    Action(Result<String, String>),
    Stopped,
}
pub struct Worker {
    pub tx: SyncSender<Command>,
    pub rx: Receiver<Event>,
}
impl Worker {
    pub fn start(dir: PathBuf, ctx: egui::Context) -> Self {
        let (tx, commands) = mpsc::sync_channel(8);
        let (events, rx) = mpsc::channel();
        thread::spawn(move || {
            let send = |e| {
                let _ = events.send(e);
                ctx.request_repaint();
            };
            let mut core = match NativeBackend::new(dir) {
                Ok(c) => c,
                Err(e) => {
                    send(Event::Loaded(Err(e)));
                    return;
                }
            };
            send(Event::Loaded(core.load_profile()));
            while let Ok(cmd) = commands.recv() {
                match cmd {
                    Command::System(mode, ms) => send(Event::System(core.system(&mode, ms))),
                    Command::Providers(mode) => send(Event::Providers(core.providers(&mode))),
                    Command::Save(value, revision) => {
                        send(Event::Saved(revision, core.save_profile(value)))
                    }
                    Command::Connect(login) => send(Event::Action(core.connect_codex(login))),
                    Command::Disconnect => {
                        core.disconnect_codex();
                        send(Event::Action(Ok("Codex disconnected".into())));
                    }
                    Command::Claude(true) => send(Event::Action(core.install_claude_bridge())),
                    Command::Claude(false) => send(Event::Action(core.remove_claude_bridge())),
                    Command::Dismiss(id) => send(Event::Action(
                        core.dismiss_attention(&id)
                            .map(|_| "Question acknowledged".into()),
                    )),
                    Command::Stop => {
                        core.disconnect_codex();
                        send(Event::Stopped);
                        break;
                    }
                }
            }
        });
        Self { tx, rx }
    }
}
