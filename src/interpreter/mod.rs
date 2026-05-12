use std::{collections::{HashMap, HashSet}, sync::Arc, thread::JoinHandle};

use crate::{interpreter::network_layer::NetworkLayer, parser::{AST, MatchClause, Pattern}};

use uuid::Uuid;

mod runtime_error;
mod builtins;
mod network_layer;

use self::{
    runtime_error::RuntimeError
};

#[derive(Debug)]
pub struct Interpreter {
    uuid: Uuid,
    network_layer: Arc<NetworkLayer>,
    threads: Vec<(Uuid, JoinHandle<Result<AST, RuntimeError>>)>,
    env: HashMap<String, AST>,
}

impl Default for Interpreter {
    fn default() -> Self {
        let uuid = Uuid::new_v4();
        let network_layer = NetworkLayer::default();
        network_layer.new_local_id(&uuid);
        Interpreter { 
            uuid, 
            network_layer: Arc::new(network_layer), 
            threads: vec![], 
            env: HashMap::new() 
        }
    }
}

impl Interpreter {
    fn fork(&self, uuid: Uuid) -> Interpreter {
        let network_layer= self.network_layer.clone();
        network_layer.new_local_id(&uuid);
        Interpreter { 
            uuid, 
            network_layer, 
            threads: vec![], 
            env: self.env.clone() 
        }
    }

    pub fn run(&mut self, tree: AST) {
        match self.eval(tree) {
            Ok(value) => println!("Main thread {} terminated with value {value:?}", self.uuid.to_string()),
            Err(e) => eprintln!("Main thread {} terminated early due to an error: {e:?}", self.uuid.to_string())
        };
        // We wait for all threads to be over
        self.threads.drain(..).for_each(|(uuid, t)| match t.join().unwrap() {
            Ok(value) => println!("Thread {uuid} terminated with value {value:?}"),
            Err(e) => eprintln!("Thread {uuid} terminated early due to an error: {e:?}")
        });
    }

    pub fn eval(&mut self, tree: AST) -> Result<AST, RuntimeError> {
        // #[cfg(debug_assertions)]
        // dbg!(&tree);

        match tree {
            AST::Unit | AST::Number(_) | AST::StringLiteral(_) | AST::Boolean(_) | AST::Lambda(_, _)=> return Ok(tree),
            AST::Tuple(values) => {
                let values = values.into_iter()
                    .map(|v| self.eval(v))
                    .collect::<Result<Vec<_>, _>>()?;
                return Ok(AST::Tuple(values))
            }
            AST::List(values) => {
                let values = values.into_iter()
                    .map(|v| self.eval(v))
                    .collect::<Result<Vec<_>, _>>()?;
                return Ok(AST::List(values))
            }
            AST::Identifier(id) => match self.env.get(&id) {
                Some(value) => Ok(value.clone()),
                None => Err(RuntimeError::RuntimeError)
            }
            AST::Let { name, value, body, .. } => {
                self.run_declaration(name, *value)?;
                self.eval(*body)
            }
            AST::FunctionCall { callee, argument } => {
                let argument = self.eval(*argument)?;
                match *callee {
                    AST::Identifier(id) => {
                        match self.env.get(&id) {
                            Some(AST::Lambda(parameter, body)) => {
                                if parameter.is_some() {
                                    let parameter = parameter.as_ref().unwrap();
                                    let substituted_body = substitute(parameter, argument, *body.clone());
                                    self.eval(substituted_body)
                                } else {
                                    self.eval(*body.clone())
                                }
                            }
                            None => self.use_builtin(&id, argument),
                            _ => Err(RuntimeError::RuntimeError)
                        }
                    }
                    AST::Lambda(param, body) => {
                        match param {
                            Some(parameter) => {
                                let new_body = substitute(&parameter, argument, *body);
                                self.eval(new_body)
                            }
                            None => self.eval(*body)
                        }
                    }
                    AST::FunctionCall { .. } => {
                        let callee = self.eval(*callee)?;
                        self.eval(AST::FunctionCall { callee: Box::new(callee), argument: Box::new(argument) })
                    }
                    _ => Err(RuntimeError::RuntimeError)
                }
            }
            AST::Conditional(condition, branch1, branch2) => {
                let cond = self.eval(*condition)?;
                match cond {
                    AST::Boolean(bool) => {
                        if bool {
                            self.eval(*branch1)
                        } else if let Some(b2) = branch2 {
                            self.eval(*b2)
                        } else {
                            Ok(AST::Unit)
                        }
                    } 
                    _ => Err(RuntimeError::RuntimeError)
                }
            }
            AST::Operation(op, values) => self.run_operation(op, values),
            AST::Case(identifier, clauses) => {
                match *identifier {
                    // This case is only hit when handlers are in play, and the receive built-in function is responsible 
                    // for handling the execution of the handlers, not the main function
                    AST::Identifier(id) if id == "_handlerInput" => Ok(AST::Case(Box::new(AST::Identifier(id)), clauses)),
                    _ => todo!()
                }
            }
            _ => todo!()
        }
    }

    fn run_declaration(&mut self, pattern: Pattern, value: AST) -> Result<(), RuntimeError> {
        let value = self.eval(value)?;
        match pattern {
            Pattern::Empty => (),
            Pattern::Variable(name) => { self.env.insert(name, value); },
            Pattern::Tuple(pattern_tuple) =>  match value {
                AST::Tuple(value_tuple) => {
                    if pattern_tuple.len() == value_tuple.len() {
                        for (pattern, value) in pattern_tuple.into_iter().zip(value_tuple) {
                            self.run_declaration(pattern, value)?;
                        }
                    } else {
                        return Err(RuntimeError::RuntimeError)
                    };
                }
                _ => return Err(RuntimeError::RuntimeError)
            }
            Pattern::Value(_) => return Err(RuntimeError::RuntimeError)
        };
        Ok(())
    }

    fn run_operation(&mut self, op: String, values: Vec<AST>) -> Result<AST, RuntimeError> {
        // We first reduce the parameters
        let mut values = values.into_iter()
            .map(|v| self.eval(v));
        let v1 = values.next().unwrap()?; // Every operation has a first member
        let v2 = values.next().transpose()?; // Some have a second member

        match (op.as_str(), v1, v2) {
            ("<", AST::Number(n1), Some(AST::Number(n2))) => Ok(AST::Boolean(n1 < n2)),
            ("<=", AST::Number(n1), Some(AST::Number(n2))) => Ok(AST::Boolean(n1 <= n2)),
            (">", AST::Number(n1), Some(AST::Number(n2))) => Ok(AST::Boolean(n1 > n2)),
            (">=", AST::Number(n1), Some(AST::Number(n2))) => Ok(AST::Boolean(n1 >= n2)),
            ("+", AST::Number(n1), Some(AST::Number(n2))) => Ok(AST::Number(n1 + n2)),
            ("-", AST::Number(n1), None) => Ok(AST::Number(-n1)),
            ("-", AST::Number(n1), Some(AST::Number(n2))) => Ok(AST::Number(n1 - n2)),
            ("/", AST::Number(n1), Some(AST::Number(n2))) => Ok(AST::Number(n1 / n2)),
            ("*", AST::Number(n1), Some(AST::Number(n2))) => Ok(AST::Number(n1 * n2)),
            ("=", ast1, Some(ast2)) => Ok(AST::Boolean(ast1 == ast2)),
            ("andalso", AST::Boolean(b1), Some(AST::Boolean(b2))) => Ok(AST::Boolean(b1 && b2)),
            ("orelse", AST::Boolean(b1), Some(AST::Boolean(b2))) => Ok(AST::Boolean(b1 || b2)),
            _ => todo!()
        }
    }
}

fn substitute(parameter: &str, argument_value: AST, body: AST) -> AST {
    match body {
        AST::Identifier(name) if name == parameter => argument_value,
        AST::Unit | AST::Wildcard | AST::Number(_) | AST::StringLiteral(_) | 
        AST::Boolean(_) | AST::SecurityLevel(_) | AST::Identifier(_) => body,
        AST::Let { name, value, body, rec } => {
            match name {
                Pattern::Variable(n) if n == parameter => {
                    AST::Let { 
                        name: Pattern::Variable(n), 
                        value: Box::new(substitute(parameter, argument_value, *value)), 
                        body, 
                        rec 
                    }
                }
                _ => AST::Let { 
                    name, 
                    value: Box::new(substitute(parameter, argument_value.clone(), *value)), 
                    body: Box::new(substitute(parameter, argument_value, *body)), 
                    rec 
                }
            }
        },
        AST::FunctionCall { callee, argument } => {
            AST::FunctionCall { 
                callee: Box::new(substitute(parameter, argument_value.clone(), *callee)), 
                argument: Box::new(substitute(parameter, argument_value, *argument))
            }
        }
        AST::Operation(name, members) => {
            AST::Operation(
                name, 
                members.into_iter()
                    .map(|m| substitute(parameter, argument_value.clone(), m))
                    .collect()
            )
        }
        AST::Case(matched_value, clauses) => {
            let new_clauses = clauses.into_iter().map(|clause| MatchClause {
                guard: clause.guard.map(|v| substitute(parameter, argument_value.clone(), v)),
                body: substitute(parameter, argument_value.clone(), clause.body),
                ..clause
            })
            .collect();

            AST::Case(
                Box::new(substitute(parameter, argument_value, *matched_value)), 
                new_clauses
            )
        }
        AST::Conditional(condition, branch1, branch2) => {
            AST::Conditional(
                Box::new(substitute(parameter, argument_value.clone(), *condition)), 
                Box::new(substitute(parameter, argument_value.clone(), *branch1)), 
                branch2.map(|b| Box::new(substitute(parameter, argument_value, *b)))
            )
        }
        AST::Tuple(values) => {
            AST::Tuple(values.into_iter()
                .map(|value| substitute(parameter, argument_value.clone(), value))
                .collect()
            )
        }
        AST::List(values) => {
            AST::List(values.into_iter()
                .map(|value| substitute(parameter, argument_value.clone(), value))
                .collect()
            )
        }
        AST::Lambda(None, body) => AST::Lambda(None, Box::new(substitute(parameter, argument_value, *body))),
        AST::Lambda(Some(var), body) if var != parameter => {
            let fv = free_variables(&argument_value);

            if fv.contains(&var) {
                let mut new_var = var.clone() + "@";
                while fv.contains(&new_var) {
                    new_var += "@";
                };
                let new_body = substitute(&var, AST::Identifier(new_var.clone()), *body);
                AST::Lambda(Some(new_var), Box::new(substitute(parameter, argument_value, new_body)))
            } else {
                AST::Lambda(Some(var), Box::new(substitute(parameter, argument_value, *body)))
            }
        }
        AST::Lambda(Some(_), _) => body,
        
    }
}

pub fn free_variables(ast: &AST) -> HashSet<String> {
    let mut set = HashSet::new();
    compute_fv(ast, &mut set);
    set
}

pub fn compute_fv(ast: &AST, acc: &mut HashSet<String>) {
    match ast {
        AST::Identifier(id) => {
            acc.insert(id.clone());
        }

        AST::FunctionCall { callee, argument } => {
            compute_fv(callee, acc);
            compute_fv(argument, acc);
        }

        AST::Operation(_, args) => {
            for e in args {
                compute_fv(e, acc);
            }
        }

        AST::Conditional(cond, then_e, else_e) => {
            compute_fv(cond, acc);
            compute_fv(then_e, acc);
            if let Some(e) = else_e {
                compute_fv(e, acc);
            }
        }

        AST::Tuple(elements) | AST::List(elements) => {
            for e in elements {
                compute_fv(e, acc);
            }
        }

        AST::Lambda(param, body) => {
            compute_fv(body, acc);

            if let Some(p) = param {
                acc.remove(p);
            }
        }

        AST::Let { name, value, body, .. } => {
            compute_fv(value, acc);

            let mut body_fv = HashSet::new();
            compute_fv(body, &mut body_fv);

            // Variables bound by the let pattern
            let mut bound = HashSet::new();
            bound_vars_pattern(name, &mut bound);

            // We remove the variables bound by the let from the body's free variables
            body_fv.retain(|v| !bound.contains(v));

            acc.extend(body_fv);
        }

        AST::Case(expr, clauses) => {
            compute_fv(expr, acc);

            for clause in clauses {
                let mut clause_fv = HashSet::new();

                // guard contributes free vars
                if let Some(g) = &clause.guard {
                    compute_fv(g, &mut clause_fv);
                }

                compute_fv(&clause.body, &mut clause_fv);

                // subtract pattern-bound variables
                let mut bound = HashSet::new();
                bound_vars_pattern(&clause.pattern, &mut bound);
                clause_fv.retain(|v| !bound.contains(v));

                acc.extend(clause_fv);
            }
        }

        AST::Unit
        | AST::Wildcard
        | AST::Number(_)
        | AST::StringLiteral(_)
        | AST::Boolean(_)
        | AST::SecurityLevel(_) => ()
    }
}

fn bound_vars_pattern(pattern: &Pattern, acc: &mut HashSet<String>) {
    match pattern {
        Pattern::Empty | Pattern::Value(_) => (),
        Pattern::Variable(id) => { acc.remove(id); },
        Pattern::Tuple(values) => {
            for p in values {
                bound_vars_pattern(p, acc);
            }
        }
    }
}