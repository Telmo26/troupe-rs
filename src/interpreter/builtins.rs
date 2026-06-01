use std::{collections::HashMap, time::Duration};

use crate::{interpreter::{Process, environment::{EnvPtr, Environment}, runtime_error::RuntimeError, runtime_value::{RuntimeValue, Value}}, parser::{AST, Pattern}};

#[derive(Debug, Clone)]
pub enum Builtin {
    BuiltinSelf,
    Spawn,
    Send,
    Receive,
    Print,
    PrintWithLabels,
    Sleep,
    Mkuuid,
    ExitAfterTimeout,
    Exit
}

impl Builtin {
    pub fn install(env: &EnvPtr) {
        env.insert("self".into(), Value::Builtin(Builtin::BuiltinSelf).into());
        env.insert("spawn".into(), Value::Builtin(Builtin::Spawn).into());
        env.insert("send".into(), Value::Builtin(Builtin::Send).into());
        env.insert("receive".into(), Value::Builtin(Builtin::Receive).into());
        env.insert("print".into(), Value::Builtin(Builtin::Print).into());
        env.insert("printWithLabels".into(), Value::Builtin(Builtin::PrintWithLabels).into());
        env.insert("sleep".into(), Value::Builtin(Builtin::Sleep).into());
        env.insert("mkuuid".into(), Value::Builtin(Builtin::Mkuuid).into());
        env.insert("exitAfterTimeout".into(), Value::Builtin(Builtin::ExitAfterTimeout).into());
        env.insert("exit".into(), Value::Builtin(Builtin::Exit).into());
    }

    pub async fn run(&self, process: &mut Process, runtime_value: RuntimeValue) -> Result<RuntimeValue, RuntimeError> {
        use Builtin::*;
        match self {
            BuiltinSelf => builtin_self(process, runtime_value).await,
            Spawn => builtin_spawn(process, runtime_value).await,
            Send => builtin_send(process, runtime_value).await,
            Receive => builtin_receive(process, runtime_value).await,
            Print => builtin_print(process, runtime_value).await,
            PrintWithLabels => builtin_print_with_labels(process, runtime_value).await,
            Sleep => builtin_sleep(process, runtime_value).await,
            Mkuuid => builtin_mkuuid(process, runtime_value).await,
            ExitAfterTimeout => builtin_exit_after_timeout(process, runtime_value).await,
            Exit => builtin_exit(process, runtime_value).await
        }
    }
}



async fn builtin_self(process: &mut Process, _runtime_value: RuntimeValue) -> Result<RuntimeValue, RuntimeError> {
    Ok(Value::PID(process.uuid).into())
}

async fn builtin_spawn(process: &mut Process, runtime_value: RuntimeValue) -> Result<RuntimeValue, RuntimeError> {
    let Value::Closure { parameter: None, body, env } = runtime_value.value
        else { return Err(RuntimeError::RuntimeError); };
    
    let new_env = Environment::new_child(&process.env);
    new_env.extend(env);

    let new_uuid = process.get_runtime()?
        .spawn_process(new_env, body)
        .await;

    Ok(Value::PID(new_uuid).into())
}

async fn builtin_send(process: &mut Process, runtime_value: RuntimeValue) -> Result<RuntimeValue, RuntimeError> {
    match runtime_value.value {
        Value::Tuple(mut v) if v.len() == 2 => {
            let value = v.pop().unwrap();
            let pid = v.pop().unwrap();

            if let Value::PID(pid) = pid.value {
                process.get_runtime()?
                    .network_layer.send(&pid, value).await
                    .unwrap();
                Ok(Value::Unit.into())
            } else {
                Err(RuntimeError::RuntimeError)
            }
        }
        _ => Err(RuntimeError::RuntimeError)
    }
}

async fn builtin_receive(process: &mut Process, runtime_value: RuntimeValue) -> Result<RuntimeValue, RuntimeError> {
    if let Ok(message) = process.receive().await {
        match runtime_value.value {
            Value::List(handlers) => {
                for handler in handlers {
                    if let Some(res) = process_handler(process, &message, handler).await? {
                        return Ok(res)
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

async fn process_handler(process: &mut Process, message: &RuntimeValue, handler: RuntimeValue) -> Result<Option<RuntimeValue>, RuntimeError> {
    let rt_value = match handler.value {
        Value::Closure { parameter: Some(variable), body, env } if variable == "msg" => {
            process.eval_closure(Some(variable), Some(message.clone()), body, env).await?
        }
        _ => return Err(RuntimeError::RuntimeError)
    };

    match rt_value.value {
        Value::Tuple(mut res) if res.len() == 2 => {
            let mut it = res.drain(0..2);
            let v1 = it.next().unwrap();
            let v2 = it.next().unwrap();

            match (v1.value, v2.value) {
                (Value::Boolean(true), Value::Closure { parameter: None, body, env }) => {
                    let result = Some(process.eval_closure(None, None, body, env).await);
                    result.transpose()
                }
                _ => Ok(None)
            }
        }
        _ => Err(RuntimeError::RuntimeError)
    }
}

async fn builtin_print(_process: &mut Process, runtime_value: RuntimeValue) -> Result<RuntimeValue, RuntimeError> {
    println!("{}", runtime_value.value);
    Ok(Value::Unit.into())
}

async fn builtin_print_with_labels(_process: &mut Process, runtime_value: RuntimeValue) -> Result<RuntimeValue, RuntimeError> {
    println!("{}", runtime_value);
    Ok(Value::Unit.into())
}

async fn builtin_sleep(_process: &mut Process, runtime_value: RuntimeValue) -> Result<RuntimeValue, RuntimeError> {
    match runtime_value.value {
        Value::Number(time) => tokio::time::sleep(Duration::from_millis(time as u64)).await,
        _ => return Err(RuntimeError::RuntimeError)
    }
    Ok(Value::Unit.into())
}

async fn builtin_mkuuid(_process: &mut Process, _runtime_value: RuntimeValue) -> Result<RuntimeValue, RuntimeError> {
    let uuid = uuid::Uuid::new_v4();
    Ok(Value::PID(uuid).into())
}

async fn builtin_exit_after_timeout(_process: &mut Process, runtime_value: RuntimeValue) -> Result<RuntimeValue, RuntimeError> {
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

    let mut env = HashMap::new();
    env.insert("authority".to_string(), runtime_value);

    Ok(Value::Closure { 
        parameter: Some("timeout".to_string()), 
        body, 
        env 
    }.into())
}

async fn builtin_exit(_process: &mut Process, runtime_value: RuntimeValue) -> Result<RuntimeValue, RuntimeError> {
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