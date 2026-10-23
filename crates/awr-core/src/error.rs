use std::{io, path::PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotFoundKind {
    Game,
    ModLoader,
    FlaAsi,
    FlaIni,
    Ola,
}

#[derive(Debug, thiserror::Error)]
pub enum AwrError {
    #[error("não encontrado: {0:?}")]
    NotFound(NotFoundKind),
    #[error("sem permissão de escrita: {path}")]
    Permission { path: PathBuf },
    #[error("erro de I/O em {path}: {src}")]
    Io { path: PathBuf, src: io::Error },
    #[error("encoding inválido em {path}")]
    Encoding { path: PathBuf },
    #[error("nome inválido: {0}")]
    InvalidName(String),
    #[error("nome curto já usado: {short}")]
    DuplicateShort { short: String },
    #[error("sem IDs livres entre {start} e {end} ({used} usados)")]
    IdExhausted { start: u32, end: u32, used: usize },
    #[error("já existe: {path}")]
    AlreadyExists { path: PathBuf },
    #[error("{path}:{line}: esperado {expected}")]
    Parse {
        path: PathBuf,
        line: usize,
        expected: &'static str,
    },
    #[error("o jogo está aberto; feche o GTA SA antes de alterar arquivos")]
    LockedByGame,
    #[error("instalação com problema: {0}")]
    Unhealthy(String),
    #[error("mod de terceiro, não é alterado: {path}")]
    ForeignMod { path: PathBuf },
    #[error("meta.json inválido em {path}: {msg}")]
    Meta { path: PathBuf, msg: String },
}

impl AwrError {
    /// Maps an io::Error to the right variant, keeping the path that failed.
    pub fn io(path: impl Into<PathBuf>, src: io::Error) -> Self {
        let path = path.into();
        match src.kind() {
            io::ErrorKind::PermissionDenied => AwrError::Permission { path },
            io::ErrorKind::AlreadyExists => AwrError::AlreadyExists { path },
            _ => AwrError::Io { path, src },
        }
    }
}

pub type Result<T> = std::result::Result<T, AwrError>;
