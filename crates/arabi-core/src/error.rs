use std::fmt;
use crate::span::Span;

#[derive(Debug, Clone)]
pub enum ArabiError {
    LexError {
        message: String,
        span: Span,
    },
    ParseError {
        message: String,
        span: Span,
    },
    CompileError {
        message: String,
        span: Span,
    },
    RuntimeError {
        message: String,
        span: Option<Span>,
    },
}

impl fmt::Display for ArabiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArabiError::LexError { message, span } => {
                write!(f, "خطا تحليل [{}]: {}", span, message)
            }
            ArabiError::ParseError { message, span } => {
                write!(f, "خطا تحليل [{}]: {}", span, message)
            }
            ArabiError::CompileError { message, span } => {
                write!(f, "خطا تجميع [{}]: {}", span, message)
            }
            ArabiError::RuntimeError { message, span } => {
                match span {
                    Some(s) => write!(f, "خطا تنفيذ [{}]: {}", s, message),
                    None => write!(f, "خطا تنفيذ: {}", message),
                }
            }
        }
    }
}

impl std::error::Error for ArabiError {}

pub type Result<T> = std::result::Result<T, ArabiError>;
