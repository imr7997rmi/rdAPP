//! rdAPP — cliente RDP ultraligero para macOS.
//! Lista de equipos en iCloud Drive, contraseñas en el Llavero,
//! conexión mediante ironrdp-viewer (IronRDP).

mod config;
#[cfg(target_os = "macos")]
mod dock;
mod editor;
mod icon;
mod keychain;
mod launch;
mod ui;

use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use muda::{AboutMetadata, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use notify::{RecursiveMode, Watcher};
use tao::event::{Event, StartCause, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tray_icon::{TrayIcon, TrayIconBuilder};

use config::Host;
use editor::Editor;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug)]
pub enum UserEvent {
    Menu(MenuEvent),
    Reload,
    Ipc(String),
    OpenEditor,
}

fn version_item() -> MenuItem {
    MenuItem::new(format!("rdAPP {VERSION}"), false, None)
}

fn build_menu(hosts: &[Host], error: Option<&str>) -> Menu {
    let menu = Menu::new();
    let _ = menu.append(&version_item());
    let _ = menu.append(&PredefinedMenuItem::separator());

    if let Some(err) = error {
        let _ = menu.append(&MenuItem::new(format!("⚠︎ {err}"), false, None));
        let _ = menu.append(&PredefinedMenuItem::separator());
    }

    if hosts.is_empty() {
        let _ = menu.append(&MenuItem::new("(sin conexiones)", false, None));
    }
    for (i, h) in hosts.iter().enumerate() {
        let _ = menu.append(&MenuItem::with_id(format!("connect:{i}"), &h.name, true, None));
    }

    let _ = menu.append(&PredefinedMenuItem::separator());
    let _ = menu.append(&MenuItem::with_id("editor", "Conexiones…", true, None));

    let forget = Submenu::new("Olvidar contraseña", !hosts.is_empty());
    for (i, h) in hosts.iter().enumerate() {
        let _ = forget.append(&MenuItem::with_id(format!("forget:{i}"), &h.name, true, None));
    }
    let _ = menu.append(&forget);
    let _ = menu.append(&MenuItem::with_id("edit", "Abrir hosts.toml", true, None));
    let _ = menu.append(&MenuItem::with_id("logs", "Ver logs", true, None));
    let _ = menu.append(&PredefinedMenuItem::separator());
    let _ = menu.append(&MenuItem::with_id("quit", "Salir", true, None));
    menu
}

/// Menú del Dock: conexiones + editor.
fn build_dock_menu(hosts: &[Host]) -> Menu {
    let menu = Menu::new();
    for (i, h) in hosts.iter().enumerate() {
        let _ = menu.append(&MenuItem::with_id(format!("connect:{i}"), &h.name, true, None));
    }
    let _ = menu.append(&PredefinedMenuItem::separator());
    let _ = menu.append(&MenuItem::with_id("editor", "Conexiones…", true, None));
    menu
}

/// Barra de menú de la app (necesaria también para copiar/pegar en el editor).
fn build_app_menu() -> Menu {
    let about = PredefinedMenuItem::about(
        Some("Acerca de rdAPP"),
        Some(AboutMetadata {
            name: Some("rdAPP".into()),
            version: Some(VERSION.into()),
            comments: Some("Cliente RDP ligero sobre IronRDP".into()),
            ..Default::default()
        }),
    );
    let app = Submenu::new("rdAPP", true);
    let _ = app.append_items(&[
        &about,
        &PredefinedMenuItem::separator(),
        &MenuItem::with_id("editor", "Conexiones…", true, None),
        &PredefinedMenuItem::separator(),
        &PredefinedMenuItem::hide(None),
        &PredefinedMenuItem::quit(Some("Salir de rdAPP")),
    ]);
    let edit = Submenu::new("Edición", true);
    let _ = edit.append_items(&[
        &PredefinedMenuItem::undo(None),
        &PredefinedMenuItem::redo(None),
        &PredefinedMenuItem::separator(),
        &PredefinedMenuItem::cut(None),
        &PredefinedMenuItem::copy(None),
        &PredefinedMenuItem::paste(None),
        &PredefinedMenuItem::select_all(None),
    ]);
    let window = Submenu::new("Ventana", true);
    let _ = window.append_items(&[
        &PredefinedMenuItem::minimize(None),
        &PredefinedMenuItem::close_window(Some("Cerrar")),
    ]);
    let menu = Menu::new();
    let _ = menu.append_items(&[&app, &edit, &window]);
    menu
}

struct State {
    /// Archivo tal cual (para el editor)
    file: config::File,
    /// Conexiones con usuario resuelto (para menús y conexión)
    hosts: Vec<Host>,
    tray: Option<TrayIcon>,
    dock_menu: Option<Menu>,
    app_menu: Option<Menu>,
    editor: Option<Editor>,
}

impl State {
    fn reload(&mut self, path: &Path) {
        let (file, mut err) = match config::load(path) {
            Ok(f) => (f, None),
            Err(e) => (config::File::default(), Some(e)),
        };
        self.hosts = Vec::new();
        for h in &file.hosts {
            match h.resolve(&file.logins) {
                Ok(r) => self.hosts.push(r),
                Err(e) => err = err.or(Some(e)),
            }
        }
        self.file = file;
        if let Some(t) = &self.tray {
            t.set_menu(Some(Box::new(build_menu(&self.hosts, err.as_deref()))));
        }
        let dock = build_dock_menu(&self.hosts);
        #[cfg(target_os = "macos")]
        {
            use muda::ContextMenu;
            dock::set_menu(dock.ns_menu());
        }
        self.dock_menu = Some(dock);
    }
}

fn main() {
    let cfg_path = config::config_path();
    config::ensure_exists(&cfg_path);

    #[allow(unused_mut)]
    let mut event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();

    #[cfg(target_os = "macos")]
    {
        use tao::platform::macos::{ActivationPolicy, EventLoopExtMacOS};
        event_loop.set_activation_policy(ActivationPolicy::Regular);
    }

    let proxy = event_loop.create_proxy();
    MenuEvent::set_event_handler(Some(move |e| {
        let _ = proxy.send_event(UserEvent::Menu(e));
    }));

    // Vigila la carpeta (iCloud reemplaza el archivo al sincronizar).
    let proxy_w = Mutex::new(event_loop.create_proxy());
    let last = Arc::new(Mutex::new(Instant::now() - Duration::from_secs(10)));
    let watch_name = cfg_path.file_name().map(|s| s.to_owned());
    let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        if let Ok(ev) = res {
            if ev.paths.iter().any(|p| p.file_name().map(|s| s.to_owned()) == watch_name) {
                let mut l = last.lock().unwrap();
                if l.elapsed() > Duration::from_millis(500) {
                    *l = Instant::now();
                    let _ = proxy_w.lock().unwrap().send_event(UserEvent::Reload);
                }
            }
        }
    })
    .ok();
    if let (Some(w), Some(dir)) = (watcher.as_mut(), cfg_path.parent()) {
        let _ = w.watch(dir, RecursiveMode::NonRecursive);
    }

    let ipc_proxy = event_loop.create_proxy();
    let mut st = State { file: config::File::default(), hosts: Vec::new(), tray: None, dock_menu: None, app_menu: None, editor: None };

    event_loop.run(move |event, target, control_flow| {
        *control_flow = ControlFlow::Wait;
        let _keep = &watcher;

        let open_editor = |st: &mut State| match &st.editor {
            Some(e) => e.focus(),
            None => st.editor = Editor::open(target, ipc_proxy.clone(), &st.file),
        };

        match event {
            Event::NewEvents(StartCause::Init) => {
                let app_menu = build_app_menu();
                #[cfg(target_os = "macos")]
                app_menu.init_for_nsapp();
                st.app_menu = Some(app_menu);

                let t = TrayIconBuilder::new()
                    .with_icon_templated(icon::tray_icon())
                    .with_tooltip(format!("rdAPP {VERSION}"))
                    .build()
                    .expect("no se pudo crear el icono de la barra de menú");
                st.tray = Some(t);
                #[cfg(target_os = "macos")]
                dock::install();
                st.reload(&cfg_path);
                if std::env::var_os("RDAPP_OPEN_EDITOR").is_some() {
                    open_editor(&mut st);
                } else {
                    ui::picker(st.hosts.clone(), ipc_proxy.clone());
                }
            }
            // Clic en el icono del Dock / volver a abrir la app.
            Event::Reopen { .. } => {
                if st.editor.is_none() {
                    ui::picker(st.hosts.clone(), ipc_proxy.clone());
                }
            }
            Event::WindowEvent { window_id, event: WindowEvent::CloseRequested, .. } => {
                if st.editor.as_ref().is_some_and(|e| e.id() == window_id) {
                    st.editor = None;
                }
            }
            Event::UserEvent(UserEvent::Reload) => st.reload(&cfg_path),
            Event::UserEvent(UserEvent::OpenEditor) => open_editor(&mut st),
            Event::UserEvent(UserEvent::Ipc(msg)) => {
                let Ok(v) = serde_json::from_str::<serde_json::Value>(&msg) else { return };
                let logins: Vec<config::Login> = serde_json::from_value(v["logins"].clone()).unwrap_or_default();
                let host = || serde_json::from_value::<Host>(v["host"].clone()).ok();
                match v["cmd"].as_str() {
                    Some("save") => {
                        let res = serde_json::from_value::<Vec<Host>>(v["hosts"].clone())
                            .map_err(|e| e.to_string())
                            .and_then(|hosts| config::save(&cfg_path, &config::File { logins: logins.clone(), hosts }));
                        if res.is_ok() {
                            st.reload(&cfg_path);
                        }
                        if let Some(e) = &st.editor {
                            e.saved(res);
                        }
                    }
                    Some("connect") => {
                        if let Some(h) = host() {
                            match h.resolve(&logins) {
                                Ok(h) => launch::connect(h),
                                Err(msg) => {
                                    if let Some(e) = &st.editor {
                                        e.notice(&msg);
                                    }
                                }
                            }
                        }
                    }
                    Some("forget") => {
                        if let Some(h) = host() {
                            keychain::delete(&h.keychain_account());
                            if let Some(e) = &st.editor {
                                e.notice(&format!("Contraseña de «{}» eliminada del Llavero.", h.name));
                            }
                        }
                    }
                    Some("set_password") => {
                        if let (Some(acc), Some(pw)) = (v["account"].as_str(), v["password"].as_str()) {
                            if !acc.is_empty() && !pw.is_empty() {
                                keychain::set(acc, pw);
                                if let Some(e) = &st.editor {
                                    e.password_saved(acc);
                                }
                            }
                        }
                    }
                    Some("forget_login") => {
                        if let Some(name) = v["name"].as_str() {
                            keychain::delete(&format!("login:{name}"));
                            if let Some(e) = &st.editor {
                                e.notice(&format!("Contraseña del usuario «{name}» eliminada del Llavero."));
                            }
                        }
                    }
                    Some("close") => st.editor = None,
                    _ => {}
                }
            }
            Event::UserEvent(UserEvent::Menu(e)) => {
                let id = e.id.as_ref();
                if let Some(i) = id.strip_prefix("connect:").and_then(|s| s.parse::<usize>().ok()) {
                    if let Some(h) = st.hosts.get(i).cloned() {
                        launch::connect(h);
                    }
                } else if let Some(i) = id.strip_prefix("forget:").and_then(|s| s.parse::<usize>().ok()) {
                    if let Some(h) = st.hosts.get(i) {
                        keychain::delete(&h.keychain_account());
                        ui::notify(&format!("Contraseña de «{}» eliminada del Llavero.", h.name));
                    }
                } else {
                    match id {
                        "editor" => open_editor(&mut st),
                        "edit" => ui::open_text(&cfg_path),
                        "logs" => ui::open_path(&launch::log_dir()),
                        "quit" => *control_flow = ControlFlow::Exit,
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    });
}
