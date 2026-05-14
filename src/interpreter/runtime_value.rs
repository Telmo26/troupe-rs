use std::{collections::{HashSet, LinkedList}, fmt::Display};

use uuid::Uuid;

use crate::{interpreter::{Interpreter, environment::EnvPtr, runtime_error::RuntimeError}, parser::AST};

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeValue {
    pub value: Value,
    pub value_label: SecurityLabel,
    pub type_label: SecurityLabel,
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
        env: EnvPtr,
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
                let mut string = String::from("(");
                for rt_value in values {
                    string.push_str(&rt_value.value.to_string());
                    string.push(',');
                }
                string.pop();
                string.push(')');
                string
            }
            Value::List(values) => {
                let mut string = String::from("[");
                for rt_value in values {
                    string.push_str(&rt_value.value.to_string());
                    string.push(',');
                }
                string.push(']');
                string
            }
            Value::Closure { parameter, body, .. } => {
                let p = if let Some(p) = parameter { p.clone() } else { String::from("()") };
                format!("({}) => {:?}", p, body)
            }
            Value::Builtin(f) => format!("<builtin function at {f:?}"),
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
        let mut output = String::from("{");
        for label in self.labels.iter() {
            output.push_str(label);
            output.push(',');
            output.push(' ');
        }
        output.push('}');
        
        write!(f, "{output}")
    }
}