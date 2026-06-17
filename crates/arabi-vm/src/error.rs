use std::fmt;

#[derive(Debug, Clone)]
pub struct CallFrame {
    pub function_name: String,
    pub line: Option<usize>,
}

#[derive(Debug)]
pub struct RuntimeError {
    class_name: String,
    message: String,
    line: Option<usize>,
    call_stack: Vec<CallFrame>,
}

impl RuntimeError {
    pub fn new(message: impl Into<String>) -> Self {
        RuntimeError { class_name: "خطا".to_string(), message: message.into(), line: None, call_stack: Vec::new() }
    }

    pub fn new_typed(class_name: impl Into<String>, message: impl Into<String>) -> Self {
        RuntimeError { class_name: class_name.into(), message: message.into(), line: None, call_stack: Vec::new() }
    }

    pub fn with_line(mut self, line: usize) -> Self {
        self.line = Some(line);
        self
    }

    pub fn with_call_stack(mut self, stack: Vec<CallFrame>) -> Self {
        self.call_stack = stack;
        self
    }

    pub fn class_name(&self) -> &str {
        &self.class_name
    }

    pub fn line(&self) -> Option<usize> {
        self.line
    }

    pub fn call_stack(&self) -> &[CallFrame] {
        &self.call_stack
    }

    pub fn into_value(self) -> crate::frame::Value {
        crate::frame::Value::Exception(Box::new(crate::frame::ExceptionData {
            class_name: self.class_name,
            message: self.message,
            line: self.line,
            call_stack: self.call_stack.into_iter().map(|f| (f.function_name, f.line)).collect(),
        }))
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)?;
        if let Some(line) = self.line {
            write!(f, " (سطر {})", line)?;
        }
        if !self.call_stack.is_empty() {
            write!(f, "\nتتبع الاستدعاء:")?;
            for frame in &self.call_stack {
                write!(f, "\n  في {}", frame.function_name)?;
                if let Some(line) = frame.line {
                    write!(f, " (سطر {})", line)?;
                }
            }
        }
        Ok(())
    }
}

impl std::error::Error for RuntimeError {}

impl From<String> for RuntimeError {
    fn from(message: String) -> Self {
        RuntimeError { class_name: "خطا".to_string(), message, line: None, call_stack: Vec::new() }
    }
}

impl From<&str> for RuntimeError {
    fn from(message: &str) -> Self {
        RuntimeError { class_name: "خطا".to_string(), message: message.to_string(), line: None, call_stack: Vec::new() }
    }
}
