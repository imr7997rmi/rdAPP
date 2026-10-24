//! rdAPP — lanzador RDP ultraligero para macOS (barra de menú).
//! Lista de equipos en iCloud Drive, contraseñas en el Llavero,
//! conexión mediante ironrdp-viewer (IronRDP).

mod config;
#[cfg(target_os = "macos")]
mod dock;
mod icon;
mod keychain;
mod launch;
mod ui;

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use muda::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use notify::{RecursiveMode, Watcher};
use tao::event::{Event, StartCause};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tray_icon::{TrayIcon, TrayIconBuilder};

use config::Host;

#[derive(Debug)]
enum UserEvent {
    Menu(MenuEvent),
    Reload,
}

fn build_menu(hosts: &[Host], error: Option<&str>) -> Menu {
    let menu = Menu::new();

    if let Some(err) = error {
        let _ = menu.append(&MenuItem::new(format!("⚠︎ {err}"), false, None));
        let _ = menu.append(&PredefinedMenuItem::separator());
    }

    if hosts.is_empty() {
        let _ = menu.append(&MenuItem::new("(sin equipos — Editar lista…)", false, None));
    }
    for (i, h) in hosts.iter().enumerate() {
        let _ = menu.append(&MenuItem::with_id(format!("connect:{i}"), &h.name, true, None));
    }

    let _ = menu.append(&PredefinedMenuItem::separator());

    let forget = Submenu::new("Olvidar contraseña", !hosts.is_empty());
    for (i, h) in hosts.iter().enumerate() {
        let _ = forget.append(&MenuItem::with_id(format!("forget:{i}"), &h.name, true, None));
    }
    let _ = menu.append(&forget);
    let _ = menu.append(&MenuItem::with_id("edit", "Editar lista…", true, None));
    let _ = menu.append(&MenuItem::with_id("reload", "Recargar", true, None));
    let _ = menu.append(&MenuItem::with_id("logs", "Ver logs", true, None));
    let _ = menu.append(&PredefinedMenuItem::separator());
    let _ = menu.append(&MenuItem::with_id("quit", "Salir", true, None));
    menu
}

/// Menú del Dock: solo equipos + editar (mismos ids que la barra de menú).
fn build_dock_menu(hosts: &[Host]) -> Menu {
    let menu = Menu::new();
    for (i, h) in hosts.iter().enumerate() {
        let _ = menu.append(&MenuItem::with_id(format!("connect:{i}"), &h.name, true, None));
    }
    let _ = menu.append(&PredefinedMenuItem::separator());
    let _ = menu.append(&MenuItem::with_id("edit", "Editar lista…", true, None));
    menu
}

fn refresh_dock(hosts: &[Host], keep: &mut Option<Menu>) {
    let menu = build_dock_menu(hosts);
    #[cfg(target_os = "macos")]
    {
        use muda::ContextMenu;
        dock::set_menu(menu.ns_menu());
    }
    *keep = Some(menu);
}

fn load_menu(path: &std::path::Path, hosts: &mut Vec<Host>) -> Menu {
    let (list, err) = match config::load(path) {
        Ok(l) => (l, None),
        Err(e) => (Vec::new(), Some(e)),
    };
    *hosts = list;
    build_menu(hosts, err.as_deref())
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

    let mut hosts: Vec<Host> = Vec::new();
    let mut tray: Option<TrayIcon> = None;
    let mut dock_menu: Option<Menu> = None;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        let _keep = &watcher;

        match event {
            Event::NewEvents(StartCause::Init) => {
                let menu = load_menu(&cfg_path, &mut hosts);
                let t = TrayIconBuilder::new()
                    .with_menu(Box::new(menu))
                    .with_icon_templated(icon::tray_icon())
                    .with_tooltip("rdAPP")
                    .build()
                    .expect("no se pudo crear el icono de la barra de menú");
                tray = Some(t);
                #[cfg(target_os = "macos")]
                dock::install();
                refresh_dock(&hosts, &mut dock_menu);
                ui::picker(hosts.clone(), cfg_path.clone());
            }
            // Clic en el icono del Dock / volver a abrir la app.
            Event::Reopen { .. } => ui::picker(hosts.clone(), cfg_path.clone()),
            Event::UserEvent(UserEvent::Reload) => {
                let menu = load_menu(&cfg_path, &mut hosts);
                if let Some(t) = &tray {
                    t.set_menu(Some(Box::new(menu)));
                }
                refresh_dock(&hosts, &mut dock_menu);
            }
            Event::UserEvent(UserEvent::Menu(e)) => {
                let id = e.id.as_ref();
                if let Some(i) = id.strip_prefix("connect:").and_then(|s| s.parse::<usize>().ok()) {
                    if let Some(h) = hosts.get(i).cloned() {
                        launch::connect(h);
                    }
                } else if let Some(i) = id.strip_prefix("forget:").and_then(|s| s.parse::<usize>().ok()) {
                    if let Some(h) = hosts.get(i) {
                        keychain::delete(&h.keychain_account());
                        ui::notify(&format!("Contraseña de «{}» eliminada del Llavero.", h.name));
                    }
                } else {
                    match id {
                        "edit" => ui::open_text(&cfg_path),
                        "reload" => {
                            let menu = load_menu(&cfg_path, &mut hosts);
                            if let Some(t) = &tray {
                                t.set_menu(Some(Box::new(menu)));
                            }
                            refresh_dock(&hosts, &mut dock_menu);
                        }
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
