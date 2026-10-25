use security_framework::passwords::{delete_generic_password, get_generic_password, set_generic_password};

const SERVICE: &str = "rdOO";
/// Nombre anterior de la app: sus contraseñas se migran al leerlas.
const OLD_SERVICE: &str = "rdAPP";

pub fn get(account: &str) -> Option<String> {
    if let Some(p) = get_generic_password(SERVICE, account).ok().and_then(|b| String::from_utf8(b).ok()) {
        return Some(p);
    }
    let old = get_generic_password(OLD_SERVICE, account).ok().and_then(|b| String::from_utf8(b).ok())?;
    set(account, &old);
    Some(old)
}

pub fn set(account: &str, password: &str) {
    let _ = set_generic_password(SERVICE, account, password.as_bytes());
}

pub fn delete(account: &str) {
    let _ = delete_generic_password(SERVICE, account);
    let _ = delete_generic_password(OLD_SERVICE, account);
}

pub fn has(account: &str) -> bool {
    get_generic_password(SERVICE, account).is_ok() || get_generic_password(OLD_SERVICE, account).is_ok()
}
