use security_framework::passwords::{delete_generic_password, get_generic_password, set_generic_password};

const SERVICE: &str = "rdAPP";

pub fn get(account: &str) -> Option<String> {
    get_generic_password(SERVICE, account)
        .ok()
        .and_then(|b| String::from_utf8(b).ok())
}

pub fn set(account: &str, password: &str) {
    let _ = set_generic_password(SERVICE, account, password.as_bytes());
}

pub fn delete(account: &str) {
    let _ = delete_generic_password(SERVICE, account);
}
