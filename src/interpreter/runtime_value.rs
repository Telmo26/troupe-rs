use std::{collections::{HashMap, HashSet, LinkedList}, fmt::Display};

use uuid::Uuid;

use crate::{interpreter::{Interpreter, runtime_error::RuntimeError}, parser::AST};

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeValue {
    pub value: Value,
    pub value_label: SecurityLabel,
    pub type_label: SecurityLabel,
}

impl RuntimeValue {
    pub fn with_value_label(self, label: SecurityLabel) -> RuntimeValue {
        RuntimeValue { 
            value: self.value, 
            value_label: label, 
            type_label: self.type_label 
        }
    }

    pub fn with_type_label(self, label: SecurityLabel) -> RuntimeValue {
        RuntimeValue { 
            value: self.value, 
            value_label: self.value_label, 
            type_label: label 
        }
    }
}

impl TryFrom<AST> for RuntimeValue {
    type Error = RuntimeError;
    fn try_from(value: AST) -> Result<Self, Self::Error> {
        let value = value.try_into()?;
        Ok(RuntimeValue {
            value,
            value_label: SecurityLabel::default(),
            type_label: SecurityLabel::default()
        })
    }
}

impl From<Value> for RuntimeValue {
    fn from(value: Value) -> Self {
        RuntimeValue {
            value,
            value_label: SecurityLabel::default(),
            type_label: SecurityLabel::default()
        }
    }
}

impl Display for RuntimeValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}@{}%{}", self.value, self.value_label, self.type_label)
    }
}

pub type BuiltinFn = fn(&mut Interpreter, RuntimeValue) -> Result<RuntimeValue, RuntimeError>;

#[derive(Debug, Clone)]
pub enum Value {
    Unit,
    Boolean(bool),
    Number(f64),
    String(String),
    Tuple(Vec<RuntimeValue>),
    List(LinkedList<RuntimeValue>),
    Closure {
        parameter: Option<String>,
        body: AST,
        env: HashMap<String, RuntimeValue>,
    },
    Builtin(BuiltinFn),
    PID(Uuid),
    Label(SecurityLabel),
    Authority, // ???

    Unreachable
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        use Value::*;

        match (self, other) {
            (Number(a), Number(b)) => a == b,
            (Boolean(a), Boolean(b)) => a == b,
            (String(a), String(b)) => a == b,
            (Unit, Unit) => true,

            (Tuple(a), Tuple(b)) => a == b,
            (List(a), List(b)) => a == b,
            (PID(pid1), PID(pid2)) => pid1 == pid2,

            // Functions are NOT comparable
            (Closure { .. }, _) | (_, Closure { .. }) => false,
            (Builtin(_), _) | (_, Builtin(_)) => false,

            _ => false,
        }
    }
}

impl TryFrom<AST> for Value {
    type Error = RuntimeError;
    fn try_from(value: AST) -> Result<Self, Self::Error> {
        match value {
            AST::Unit => Ok(Value::Unit),
            AST::Boolean(b) => Ok(Value::Boolean(b)),
            AST::Number(n) => Ok(Value::Number(n)),
            AST::StringLiteral(s) => Ok(Value::String(s)),
            AST::SecurityLevel(levels) => Ok(Value::Label(SecurityLabel {
                labels: levels.into_iter().collect()
            })),
            _ => Err(RuntimeError::RuntimeError)
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string = match self {
            Value::Unit => "()".to_string(),
            Value::Boolean(b) => b.to_string(),
            Value::Number(n) => n.to_string(),
            Value::String(s) => s.clone(),
            Value::Tuple(values) => {
                let joined = values.iter()
                    .cloned()
                    .map(|rt_value| rt_value.value.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("({joined})")
            }
            Value::List(values) => {
                let joined = values.iter()
                    .cloned()
                    .map(|rt_value| rt_value.value.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("[{joined}]")
            }
            Value::Closure { parameter, body, .. } => {
                let p = if let Some(p) = parameter { p.clone() } else { String::from("") };
                format!("({}) => {:?}", p, body)
            }
            Value::Builtin(f) => format!("<builtin function at {f:?}>"),
            Value::PID(pid) => pid.to_string(),
            Value::Label(label) => label.to_string(),
            Value::Authority => String::from("authority"),
            Value::Unreachable => String::from("UNREACHABLE")
        };
        write!(f, "{string}")
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SecurityLabel {
    labels: HashSet<String>
}

impl Display for SecurityLabel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let joined = self.labels.iter().cloned().collect::<Vec<_>>().join(", ");
        write!(f, "{{{joined}}}")
    }
}