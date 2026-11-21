use serde::Serialize;

/// Ошибка команд IPC. Никогда не содержит секретов: только код и безопасное сообщение.
#[derive(Debug)]
pub struct AppError {
    pub code: &'static str,
    pub message: String,
}

impl AppError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self { code, message: message.into() }
    }
    pub fn internal(message: impl Into<String>) -> Self {
        Self::new("internal", message)
    }
    /// Секреты никогда не включаем в текст: только провайдер и код.
    pub fn provider(code: &'static str, provider: &str, detail: impl std::fmt::Display) -> Self {
        Self::new(code, format!("{provider}: {detail}"))
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for AppError {}

impl Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("AppError", 2)?;
        s.serialize_field("code", self.code)?;
        s.serialize_field("message", &self.message)?;
        s.end()
    }
}

pub type CmdResult<T> = Result<T, AppError>;
