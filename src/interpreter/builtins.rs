use std::{str::FromStr, thread, time::Duration};

use crate::{interpreter::{Interpreter, runtime_error::RuntimeError}, parser::AST};

use uuid::{self, Uuid};

impl Interpreter {
    pub fn use_builtin(&mut self, identifier: &str, value: AST) -> Result<AST, RuntimeError> {
        match identifier {
            "print" => Ok(builtin_print(value)),
            "sleep" => builtin_sleep(value),
            "self" => Ok(self.builtin_self()),
            "mkuuid" => Ok(builtin_mkuuid()),
            "spawn" => Ok(self.builtin_spawn(value)),
            "send" => self.builtin_send(value),
            "receive" => Ok(self.builtin_receive(value)),            
            _ => Err(RuntimeError::RuntimeError)
        }
    }

    fn builtin_self(&self) -> AST {
        AST::StringLiteral(self.uuid.to_string())
    }

    fn builtin_spawn(&mut self, value: AST) -> AST {
        let new_uuid = Uuid::new_v4();
        let mut interpreter = self.fork(new_uuid.clone());
        
        let function = AST::FunctionCall { callee: Box::new(value), argument: Box::new(AST::Unit) };
        // println!("Spawning a new thread for {function:#?}\nThread environment: {:#?}", interpreter.env);

        self.threads.push((
            new_uuid.clone(),
            thread::spawn(move || interpreter.eval(function))
        ));
        AST::StringLiteral(new_uuid.to_string())
    }

    fn builtin_send(&mut self, parameters: AST) -> Result<AST, RuntimeError> {
        match parameters {
            AST::Tuple(mut v) if v.len() == 2 => {
                let value = v.pop().unwrap();
                let pid = v.pop().unwrap();

                let pid = self.eval(pid)?;

                if let AST::StringLiteral(pid) = pid && 
                    let Ok(uuid) = Uuid::from_str(&pid) 
                {
                    self.network_layer.send(&uuid, value).unwrap();
                    Ok(AST::Unit)
                } else {
                    Err(RuntimeError::RuntimeError)
                }
            }
            _ => Err(RuntimeError::RuntimeError)
        }
    }

    fn builtin_receive(&self, handlers: AST) -> AST {
        if let Ok(message) = self.network_layer.receive(&self.uuid) {
            println!("Received {message:?}")
        } 
        AST::Unit
    }
}

fn builtin_print(value: AST) -> AST {
    dbg!(value);
    AST::Unit
}

fn builtin_sleep(value: AST) -> Result<AST, RuntimeError> {
    match value {
        AST::Number(time) => thread::sleep(Duration::from_millis(time as u64)),
        _ => return Err(RuntimeError::RuntimeError)
    }
    Ok(AST::Unit)
}

fn builtin_mkuuid() -> AST {
    let uuid = uuid::Uuid::new_v4().to_string();
    AST::StringLiteral(uuid)
}