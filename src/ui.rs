//! Diálogos nativos mínimos vía osascript (sin dependencias de GUI).

use std::path::Path;
use std::process::Command;

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Pide la contraseña con un diálogo oculto. None si se cancela.
pub fn ask_password(host_name: &str, account: &str) -> Option<String> {
    let script = format!(
        "text returned of (display dialog \"Contraseña para {}\\n{}\" default answer \"\" with hidden answer with title \"rdAPP\" buttons {{\"Cancelar\", \"Conectar\"}} default button \"Conectar\")",
        esc(host_name),
        esc(account)
    );
    let out = Command::new("osascript").arg("-e").arg(script).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim_end_matches('\n').to_string();
    if s.is_empty() { None } else { Some(s) }
}

/// Alerta con botones; devuelve el botón pulsado.
pub fn alert(title: &str, msg: &str, buttons: &[&str]) -> Option<String> {
    let btns = buttons.iter().map(|b| format!("\"{}\"", esc(b))).collect::<Vec<_>>().join(", ");
    let script = format!(
        "button returned of (display alert \"{}\" message \"{}\" buttons {{{}}})",
        esc(title),
        esc(msg),
        btns
    );
    let out = Command::new("osascript").arg("-e").arg(script).output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

pub fn notify(msg: &str) {
    let script = format!("display notification \"{}\" with title \"rdAPP\"", esc(msg));
    let _ = Command::new("osascript").arg("-e").arg(script).spawn();
}

pub fn open_text(path: &Path) {
    let _ = Command::new("open").arg("-t").arg(path).spawn();
}

pub fn open_path(path: &Path) {
    let _ = std::fs::create_dir_all(path);
    let _ = Command::new("open").arg(path).spawn();
}

const EDIT_ITEM: &str = "✎ Editar lista…";

/// Lista de equipos al abrir la app o pulsar el Dock. No bloquea el bucle de eventos.
pub fn picker(hosts: Vec<crate::config::Host>, cfg: std::path::PathBuf) {
    std::thread::spawn(move || {
        let mut items: Vec<String> = hosts.iter().map(|h| h.name.clone()).collect();
        items.push(EDIT_ITEM.to_string());
        let list = items.iter().map(|s| format!("\"{}\"", esc(s))).collect::<Vec<_>>().join(", ");
        let default = items.first().map(|s| format!("\"{}\"", esc(s))).unwrap_or_default();
        let script = format!(
            "activate\n\
             set r to choose from list {{{list}}} with title \"rdAPP\" with prompt \"Conectar a:\" default items {{{default}}} OK button name \"Conectar\" cancel button name \"Cerrar\"\n\
             if r is false then return \"\"\n\
             return item 1 of r"
        );
        let Ok(out) = Command::new("osascript").arg("-e").arg(script).output() else { return };
        let choice = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if choice.is_empty() {
            return;
        }
        if choice == EDIT_ITEM {
            open_text(&cfg);
        } else if let Some(h) = hosts.into_iter().find(|h| h.name == choice) {
            crate::launch::connect(h);
        }
    });
}
