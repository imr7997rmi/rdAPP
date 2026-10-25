//! Diálogos nativos mínimos vía osascript (sin dependencias de GUI).

use std::path::Path;
use std::process::Command;

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Pide la contraseña con un diálogo oculto. None si se cancela.
pub fn ask_password(host_name: &str, account: &str) -> Option<String> {
    let script = format!(
        "text returned of (display dialog \"Contraseña para {}\\n{}\" default answer \"\" with hidden answer with title \"rdOO\" buttons {{\"Cancelar\", \"Conectar\"}} default button \"Conectar\")",
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
    let script = format!("display notification \"{}\" with title \"rdOO\"", esc(msg));
    let _ = Command::new("osascript").arg("-e").arg(script).spawn();
}

pub fn open_text(path: &Path) {
    let _ = Command::new("open").arg("-t").arg(path).spawn();
}

pub fn open_path(path: &Path) {
    let _ = std::fs::create_dir_all(path);
    let _ = Command::new("open").arg(path).spawn();
}

