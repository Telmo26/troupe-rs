use std::{thread, time::Duration};

use crate::{interpreter::{Interpreter, environment::{EnvPtr, Environment}, runtime_error::RuntimeError, runtime_value::{RuntimeValue, Value}}, parser::{AST, Pattern}};

use uuid::Uuid;

pub fn install(env: &EnvPtr) {
    env.insert("self".into(), Value::Builtin(builtin_self).into());
    env.insert("spawn".into(), Value::Builtin(builtin_spawn).into());
    env.insert("send".into(), Value::Builtin(builtin_send).into());
    env.insert("receive".into(), Value::Builtin(builtin_receive).into());
    env.insert("print".into(), Value::Builtin(builtin_print).into());
    env.insert("sleep".into(), Value::Builtin(builtin_sleep).into());
    env.insert("mkuuid".into(), Value::Builtin(builtin_mkuuid).into());
    env.insert("exitAfterTimeout".into(), Value::Builtin(builtin_exit_after_timeout).into());
    env.insert("exit".into(), Value::Builtin(builtin_exit).into());
}

fn builtin_self(interpreter: &mut Interpreter, _runtime_value: RuntimeValue) -> Result<RuntimeValue, RuntimeError> {
    Ok(Value::PID(interpreter.uuid).into())
}

fn builtin_spawn(interpreter: &mut Interpreter, runtime_value: RuntimeValue) -> Result<RuntimeValue, RuntimeError> {
    let new_uuid = Uuid::new_v4();
    let mut new_interpreter = interpreter.fork(new_uuid.clone());

    let thread = match runtime_value.value {
        Value::Closure { parameter: None, body, env } => {
            interpreter.env = env;
            thread::spawn(move || new_interpreter.eval(body))
        }
        Value::Builtin(f) => thread::spawn(move || f(&mut new_interpreter, Value::Unit.into())),
        _ => return Err(RuntimeError::RuntimeError)
    };

    interpreter.threads.push((
        new_uuid.clone(),
        thread
    ));
    Ok(Value::PID(new_uuid).into())
}

fn builtin_send(interpreter: &mut Interpreter, runtime_value: RuntimeValue) -> Result<RuntimeValue, RuntimeError> {
    match runtime_value.value {
        Value::Tuple(mut v) if v.len() == 2 => {
            let value = v.pop().unwrap();
            let pid = v.pop().unwrap();

            if let Value::PID(pid) = pid.value {
                interpreter.network_layer.send(&pid, value).unwrap();
                Ok(Value::Unit.into())
            } else {
                Err(RuntimeError::RuntimeError)
            }
        }
        _ => Err(RuntimeError::RuntimeError)
    }
}

fn builtin_receive(interpreter: &mut Interpreter, runtime_value: RuntimeValue) -> Result<RuntimeValue, RuntimeError> {
    if let Ok(message) = interpreter.network_layer.receive(&interpreter.uuid) {
        match runtime_value.value {
            Value::List(handlers) => {
                for handler in handlers {
                    if let Some(res) = process_handler(interpreter, &message, handler) {
                        return res
                    }
                }
                Err(RuntimeError::RuntimeError)
            }
            _ => Err(RuntimeError::RuntimeError)
        }
    } else {
        Err(RuntimeError::RuntimeError)
    }
}

fn process_handler(interpreter: &mut Interpreter, message: &RuntimeValue, handler: RuntimeValue) -> Option<Result<RuntimeValue, RuntimeError>> {
    let rt_value = match handler.value {
        Value::Closure { parameter: Some(variable), body, env } if variable == "msg" => {
            let old_env = std::mem::replace(&mut interpreter.env, env);

            interpreter.env.insert(variable, message.clone());

            let result = match interpreter.eval(body) {
                Ok(rt) => rt,
                Err(e) => return Some(Err(e))
            };
            interpreter.env = old_env;

            result
        }
        _ => return Some(Err(RuntimeError::RuntimeError))
    };

    match rt_value.value {
        Value::Tuple(mut res) if res.len() == 2 => {
            let mut it = res.drain(0..2);
            let v1 = it.next().unwrap();
            let v2 = it.next().unwrap();

            match (v1.value, v2.value) {
                (Value::Boolean(true), Value::Closure { parameter: None, body, env }) => {
                    let old_env = std::mem::replace(&mut interpreter.env, env);

                    let result = match interpreter.eval(body) {
                        Ok(rt) => rt,
                        Err(e) => return Some(Err(e))
                    };
                    interpreter.env = old_env;

                    Some(Ok(result))
                }
                _ => None
            }
        }
        _ => Some(Err(RuntimeError::RuntimeError))
    }
}

fn builtin_print(_interpreter: &mut Interpreter, runtime_value: RuntimeValue) -> Result<RuntimeValue, RuntimeError> {
    println!("{}", runtime_value.value);
    Ok(Value::Unit.into())
}

fn builtin_sleep(_interpreter: &mut Interpreter, runtime_value: RuntimeValue) -> Result<RuntimeValue, RuntimeError> {
    match runtime_value.value {
        Value::Number(time) => thread::sleep(Duration::from_millis(time as u64)),
        _ => return Err(RuntimeError::RuntimeError)
    }
    Ok(Value::Unit.into())
}

fn builtin_mkuuid(_interpreter: &mut Interpreter, _runtime_value: RuntimeValue) -> Result<RuntimeValue, RuntimeError> {
    let uuid = uuid::Uuid::new_v4();
    Ok(Value::PID(uuid).into())
}

fn builtin_exit_after_timeout(interpreter: &mut Interpreter, runtime_value: RuntimeValue) -> Result<RuntimeValue, RuntimeError> {
    let body = AST::Lambda(
        Some("exit_code".to_string()),
        Box::new(AST::Lambda(
            Some("exit_message".to_string()), 
            Box::new(AST::Let { 
                name: Pattern::Empty, 
                value: Box::new(AST::FunctionCall { callee: Box::new(AST::Identifier("sleep".to_string())), argument: Box::new(AST::Identifier("timeout".to_string())) }), 
                body: Box::new(AST::Let { 
                    name: Pattern::Empty, 
                    value: Box::new(AST::FunctionCall { callee: Box::new(AST::Identifier("print".to_string())), argument: Box::new(AST::Identifier("exit_message".to_string())) }), 
                    body: Box::new(AST::FunctionCall { 
                        callee: Box::new(AST::Identifier("exit".to_string())), 
                        argument: Box::new(AST::Tuple(vec![AST::Identifier("authority".to_string()), AST::Identifier("exit_code".to_string())])) 
                    }), 
                    rec: false 
                }), 
                rec: false 
            })
        ))
    );

    let env = Environment::new_child(interpreter.env.clone());
    env.insert("authority".to_string(), runtime_value);

    Ok(Value::Closure { 
        parameter: Some("timeout".to_string()), 
        body, 
        env 
    }.into())
}

fn builtin_exit(_interpreter: &mut Interpreter, runtime_value: RuntimeValue) -> Result<RuntimeValue, RuntimeError> {
    match runtime_value.value {
        Value::Tuple(mut values) if values.len() == 2 => {
            let mut it = values.drain(0..2);
            let auth = it.next().unwrap();
            let err_code = it.next().unwrap();

            match (auth.value, err_code.value) {
                (Value::Authority, Value::Number(n)) => {
                    std::process::exit(n as i32)
                }
                _ => return Err(RuntimeError::RuntimeError)
            }
        }
        _ => return Err(RuntimeError::RuntimeError)
    }
}