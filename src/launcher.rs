//! Ventana principal: lista de conexiones para abrir rápido.

use tao::dpi::LogicalSize;
use tao::event_loop::{EventLoopProxy, EventLoopWindowTarget};
use tao::window::{Window, WindowBuilder, WindowId};
use wry::{WebView, WebViewBuilder};

use crate::UserEvent;
use crate::config::Host;

const HTML: &str = include_str!("launcher.html");

pub struct Launcher {
    window: Window,
    webview: WebView,
}

fn hosts_json(hosts: &[Host]) -> serde_json::Value {
    serde_json::Value::Array(
        hosts
            .iter()
            .map(|h| serde_json::json!({ "name": h.name, "address": h.address, "user": h.user }))
            .collect(),
    )
}

impl Launcher {
    pub fn open(
        target: &EventLoopWindowTarget<UserEvent>,
        proxy: EventLoopProxy<UserEvent>,
        hosts: &[Host],
    ) -> Option<Self> {
        let window = WindowBuilder::new()
            .with_title("rdOO")
            .with_inner_size(LogicalSize::new(380.0, 520.0))
            .with_min_inner_size(LogicalSize::new(300.0, 300.0))
            .build(target)
            .ok()?;
        let webview = WebViewBuilder::new()
            .with_html(HTML)
            .with_initialization_script(format!("window.RDOO_HOSTS = {};", hosts_json(hosts)))
            .with_ipc_handler(move |req| {
                let _ = proxy.send_event(UserEvent::Ipc(req.body().clone()));
            })
            .build(&window)
            .ok()?;
        window.set_focus();
        Some(Self { window, webview })
    }

    pub fn id(&self) -> WindowId {
        self.window.id()
    }

    pub fn focus(&self) {
        self.window.set_visible(true);
        self.window.set_minimized(false);
        self.window.set_focus();
    }

    pub fn set_hosts(&self, hosts: &[Host]) {
        let _ = self.webview.evaluate_script(&format!("rdooSetHosts({})", hosts_json(hosts)));
    }
}
