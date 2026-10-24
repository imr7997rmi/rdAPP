//! Editor visual de equipos: ventana tao + WebView del sistema (wry).

use tao::dpi::LogicalSize;
use tao::event_loop::{EventLoopProxy, EventLoopWindowTarget};
use tao::window::{Window, WindowBuilder, WindowId};
use wry::{WebView, WebViewBuilder};

use crate::config::Host;
use crate::{UserEvent, VERSION};

const HTML: &str = include_str!("editor.html");

pub struct Editor {
    window: Window,
    webview: WebView,
}

impl Editor {
    pub fn open(
        target: &EventLoopWindowTarget<UserEvent>,
        proxy: EventLoopProxy<UserEvent>,
        hosts: &[Host],
    ) -> Option<Self> {
        let window = WindowBuilder::new()
            .with_title(format!("Equipos — rdAPP {VERSION}"))
            .with_inner_size(LogicalSize::new(820.0, 580.0))
            .with_min_inner_size(LogicalSize::new(640.0, 440.0))
            .build(target)
            .ok()?;

        let init = serde_json::json!({ "version": VERSION, "hosts": hosts });
        let webview = WebViewBuilder::new()
            .with_html(HTML)
            .with_initialization_script(format!("window.RDAPP_INIT = {init};"))
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
        self.window.set_focus();
    }

    pub fn saved(&self, result: Result<(), String>) {
        let js = match result {
            Ok(()) => "rdappSaved(true)".to_string(),
            Err(e) => format!("rdappSaved(false, {})", serde_json::Value::String(e)),
        };
        let _ = self.webview.evaluate_script(&js);
    }

    pub fn notice(&self, msg: &str) {
        let _ = self
            .webview
            .evaluate_script(&format!("rdappNotice({})", serde_json::Value::String(msg.to_string())));
    }
}
