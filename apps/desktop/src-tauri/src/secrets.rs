//! Хранение API-ключей: системное кольцо Secret Service при наличии, иначе —
//! только память процесса (сессионный fallback, ключ теряется при выходе).
//! Ключи НИКОГДА не попадают в настройки, экспорт, логи или сообщения об ошибках.
use std::collections::HashMap;
use std::sync::Mutex;

pub const SERVICE: &str = "meshpilot";

pub struct SecretsState {
    /// Сессионное хранилище (в памяти, не на диске).
    session: Mutex<HashMap<String, String>>,
}

impl Default for SecretsState {
    fn default() -> Self {
        Self::new()
    }
}

impl SecretsState {
    pub fn new() -> Self {
        Self { session: Mutex::new(HashMap::new()) }
    }

    fn session_get(&self, account: &str) -> Option<String> {
        self.session.lock().unwrap().get(account).cloned()
    }

    fn session_set(&self, account: &str, value: &str) {
        self.session.lock().unwrap().insert(account.to_string(), value.to_string());
    }

    fn session_delete(&self, account: &str) {
        self.session.lock().unwrap().remove(account);
    }
}

/// Где хранится ключ провайдера.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KeyLocation {
    None,
    Keyring,
    Session,
}

impl KeyLocation {
    pub fn as_str(self) -> &'static str {
        match self {
            KeyLocation::None => "none",
            KeyLocation::Keyring => "keyring",
            KeyLocation::Session => "session",
        }
    }
}

fn keyring_entry(account: &str) -> Result<keyring::Entry, keyring::Error> {
    keyring::Entry::new(SERVICE, account)
}

/// Доступно ли системное кольцо: трактуем NoEntry как «кольцо работает, ключа нет».
fn keyring_readable() -> bool {
    match keyring_entry("__meshpilot_probe__").and_then(|e| e.get_password()) {
        Ok(_) | Err(keyring::Error::NoEntry) => true,
        Err(_) => false,
    }
}

/// Возвращает (ключ, местоположение). Порядок: кольцо → сессия.
pub fn get_secret(state: &SecretsState, account: &str) -> (Option<String>, KeyLocation) {
    if let Ok(entry) = keyring_entry(account) {
        match entry.get_password() {
            Ok(secret) => return (Some(secret), KeyLocation::Keyring),
            Err(_) => {} // NoEntry или кольцо недоступно — пробуем сессию
        }
    }
    if let Some(secret) = state.session_get(account) {
        return (Some(secret), KeyLocation::Session);
    }
    (None, KeyLocation::None)
}

/// Сохранить ключ. Возвращает фактическое местоположение.
pub fn store_secret(state: &SecretsState, account: &str, secret: &str) -> KeyLocation {
    if let Ok(entry) = keyring_entry(account) {
        if entry.set_password(secret).is_ok() {
            state.session_delete(account); // не дублируем в памяти, если лёг в кольцо
            return KeyLocation::Keyring;
        }
    }
    state.session_set(account, secret);
    KeyLocation::Session
}

/// Удалить ключ из обоих хранилищ. Идемпотентно.
pub fn delete_secret(state: &SecretsState, account: &str) -> KeyLocation {
    let mut removed_from = KeyLocation::None;
    if let Ok(entry) = keyring_entry(account) {
        match entry.delete_credential() {
            Ok(()) => removed_from = KeyLocation::Keyring,
            Err(_) => {}
        }
    }
    if state.session_get(account).is_some() {
        state.session_delete(account);
        if removed_from == KeyLocation::None {
            removed_from = KeyLocation::Session;
        }
    }
    removed_from
}

pub fn keyring_available() -> bool {
    keyring_readable()
}
