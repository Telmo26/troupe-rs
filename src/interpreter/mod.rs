use std::{collections::{HashMap, HashSet, LinkedList}, sync::Arc, thread::JoinHandle};

use crate::{interpreter::environment::Environment, parser::{AST, MatchClause, Pattern}};

use uuid::Uuid;

mod runtime_error;
mod builtins;
mod network_layer;
mod runtime_value;
mod environment;

use self::{
    runtime_error::RuntimeError,
    runtime_value::{RuntimeValue, Value},
    environment::EnvPtr,
    network_layer::NetworkLayer,
};

#[derive(Debug)]
pub struct Interpreter {
    uuid: Uuid,
    network_layer: Arc<NetworkLayer>,
    threads: Vec<(Uuid, JoinHandle<Result<RuntimeValue, RuntimeError>>)>,
    env: EnvPtr,
}

impl Default for Interpreter {
    fn default() -> Self {
        let uuid = Uuid::new_v4();
        let network_layer = NetworkLayer::default();
        network_layer.new_local_id(&uuid);

        let mut env = EnvPtr::default();
        builtins::install(&mut env);
        env.insert("authority".into(), Value::Authority.into());

        Interpreter { 
            uuid, 
            network_layer: Arc::new(network_layer), 
            threads: vec![], 
            env 
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

    pub fn eval(&mut self, tree: AST) -> Result<RuntimeValue, RuntimeError> {
        // #[cfg(debug_assertions)]
        // dbg!(&tree);

        match tree {
            AST::Unreachable => Ok(Value::Unreachable.into()),
            AST::Unit | AST::Number(_) | AST::StringLiteral(_) | AST::Boolean(_) | AST::SecurityLevel(_) => return tree.try_into(),
            AST::Lambda(parameter, body) => {
                let mut free_variables = free_variables(&body);

                if let Some(p) = parameter.as_ref() {
                    free_variables.remove(p);
                }

                let mut closure_env = HashMap::new();
                for free_variable in free_variables {
                    if let Some(rt_value) = self.env.get(&free_variable) && !matches!(rt_value.value, Value::Builtin(_)) {
                        closure_env.insert(free_variable, rt_value);
                    }
                }

                return Ok(Value::Closure { 
                    parameter, 
                    body: *body, 
                    env: closure_env 
                }.into())
            }
            AST::Tuple(values) => {
                let values = values.into_iter()
                    .map(|v| self.eval(v))
                    .collect::<Result<Vec<_>, _>>()?;
                return Ok(Value::Tuple(values).into())
            }
            AST::List(values) => {
                let values = values.into_iter()
                    .map(|v| self.eval(v))
                    .collect::<Result<LinkedList<_>, _>>()?;
                return Ok(Value::List(values).into())
            }
            AST::Identifier(id) => match self.env.get(&id) {
                Some(value) => Ok(value.clone()),
                None => Err(RuntimeError::RuntimeError)
            }
            AST::Let { name, value, body, rec } => {
                let let_env = Environment::new_child(self.env.clone());
                let old_env = self.env.clone();
                
                if rec {
                    self.env = let_env.clone();
                    let v = self.eval(*value)?;
                    self.run_declaration(name, v)?;
                } else {
                    let v = self.eval(*value)?;
                    self.env = let_env.clone();
                    self.run_declaration(name, v)?;
                }

                let result = self.eval(*body);
                self.env = old_env;
                result
            }
            AST::FunctionCall { callee, argument } => {
                let callee_val = self.eval(*callee)?;
                let arg_val = self.eval(*argument)?;

                match callee_val.value {
                    Value::Closure { parameter, body, env } => {
                        self.eval_closure(parameter, Some(arg_val), body, env)
                    }

                    Value::Builtin(f) => f(self, arg_val),

                    _ => Err(RuntimeError::RuntimeError),
                }
            }
            AST::Conditional(condition, branch1, branch2) => {
                let cond = self.eval(*condition)?;
                match cond.value {
                    Value::Boolean(bool) => {
                        if bool {
                            self.eval(*branch1)
                        } else if let Some(b2) = branch2 {
                            self.eval(*b2)
                        } else {
                            Ok(Value::Unit.into())
                        }
                    } 
                    _ => Err(RuntimeError::RuntimeError)
                }
            }
            AST::Operation(op, values) => self.run_operation(op, values),
            AST::Case(expr, clauses) => {
                let value = self.eval(*expr)?;
                self.run_case(value, clauses)
            }
        }
    }

    fn run_declaration(&mut self, pattern: Pattern, runtime_value: RuntimeValue) -> Result<(), RuntimeError> {
        match pattern {
            Pattern::Empty => (),
            Pattern::Variable(name) => { self.env.insert(name, runtime_value); },
            Pattern::Tuple(pattern_tuple) => match runtime_value.value {
                Value::Tuple(value_tuple) => {
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

    fn run_operation(&mut self, op: String, values: Vec<AST>) -> Result<RuntimeValue, RuntimeError> {
        // We first reduce the parameters
        let mut values = values.into_iter()
            .map(|v| self.eval(v));
        let v1 = values.next().unwrap()?.value; // Every operation has a first member
        let v2 = values.next().transpose()?.map(|v| v.value); // Some have a second member

        match (op.as_str(), v1, v2) {
            ("<", Value::Number(n1), Some(Value::Number(n2))) => Ok(Value::Boolean(n1 < n2).into()),
            ("<=", Value::Number(n1), Some(Value::Number(n2))) => Ok(Value::Boolean(n1 <= n2).into()),
            (">", Value::Number(n1), Some(Value::Number(n2))) => Ok(Value::Boolean(n1 > n2).into()),
            (">=", Value::Number(n1), Some(Value::Number(n2))) => Ok(Value::Boolean(n1 >= n2).into()),
            ("+", Value::Number(n1), Some(Value::Number(n2))) => Ok(Value::Number(n1 + n2).into()),
            ("-", Value::Number(n1), None) => Ok(Value::Number(-n1).into()),
            ("-", Value::Number(n1), Some(Value::Number(n2))) => Ok(Value::Number(n1 - n2).into()),
            ("/", Value::Number(n1), Some(Value::Number(n2))) => Ok(Value::Number(n1 / n2).into()),
            ("*", Value::Number(n1), Some(Value::Number(n2))) => Ok(Value::Number(n1 * n2).into()),
            ("=", ast1, Some(ast2)) => Ok(Value::Boolean(ast1 == ast2).into()),
            ("andalso", Value::Boolean(b1), Some(Value::Boolean(b2))) => Ok(Value::Boolean(b1 && b2).into()),
            ("orelse", Value::Boolean(b1), Some(Value::Boolean(b2))) => Ok(Value::Boolean(b1 || b2).into()),
            ("::", value, Some(Value::List(mut l))) => {
                l.push_front(value.into());
                Ok(Value::List(l).into())
            },
            ("raisedTo", value, Some(Value::Label(l))) => {
                let rt_value = RuntimeValue::from(value)
                    .with_value_label(l);
                Ok(rt_value)
            }
            _ => todo!()
        }
    }

    fn run_case(&mut self, value: RuntimeValue, clauses: Vec<MatchClause>) -> Result<RuntimeValue, RuntimeError> {
        for clause in clauses {
            if let Some(bindings) = self.match_pattern(&clause.pattern, &value)? {
                let old_env = self.env.clone();

                let new_env = Environment::new_child(self.env.clone());
                new_env.extend(bindings);

                self.env = new_env;

                // Evaluate guard if present
                let guard_ok = if let Some(guard) = clause.guard {
                    match self.eval(guard)?.value {
                        Value::Boolean(b) => b,
                        _ => return Err(RuntimeError::RuntimeError)
                    }
                } else {
                    true
                };

                if guard_ok {
                    let result = self.eval(clause.body);
                    self.env = old_env;
                    return result
                }

                // Guard failed, restore env and continue
                self.env = old_env;
            }
        }

        Err(RuntimeError::RuntimeError)
    }

    fn match_pattern(&mut self, pattern: &Pattern, value: &RuntimeValue) -> Result<Option<HashMap<String, RuntimeValue>>, RuntimeError> {
        match pattern {
            Pattern::Empty => Ok(Some(HashMap::new())),
            Pattern::Variable(name) => {
                let mut map = HashMap::new();
                map.insert(name.clone(), value.clone());
                Ok(Some(map))
            }
            Pattern::Value(ast) => {
                let litt_value: Value = (*ast.clone()).try_into()?;
                if litt_value == value.value {
                    Ok(Some(HashMap::new()))
                } else {
                    Ok(None)
                }
            }
            Pattern::Tuple(patterns) => match &value.value {
                Value::Tuple(values) => {
                    if patterns.len() != values.len() {
                        return Ok(None);
                    }

                    // The new bindings to be done recursively
                    let mut bindings = HashMap::new();

                    for (p, v) in patterns.iter().zip(values) {
                        match self.match_pattern(p, v)? {
                            Some(b) => bindings.extend(b),
                            None => return Ok(None)
                        }
                    }

                    Ok(Some(bindings))
                }
                _ => Ok(None)
            }
        }
    }

    fn eval_closure(&mut self, parameter: Option<String>, argument: Option<RuntimeValue>, body: AST, env: HashMap<String, RuntimeValue>) -> Result<RuntimeValue, RuntimeError> {
        let call_env = Environment::new_child(self.env.clone());
        call_env.extend(env);

        if let Some(param) = parameter && let Some(arg_val) = argument {
            call_env.insert(param, arg_val);
        }

        let old_env = self.env.clone();
        self.env = call_env;

        let result = self.eval(body);

        self.env = old_env;
        result
    }
}

fn free_variables(body: &AST) -> HashSet<String> {
    let mut fv = HashSet::new();
    compute_fv(body, &mut fv);
    fv
}

fn compute_fv(body: &AST, acc: &mut HashSet<String>) {
    match body {
        AST::Let { name, value, body, rec } => {
            compute_fv(body, acc);
            remove_bound_variables(name, acc); // We remove the bound values from the body
            compute_fv(value, acc);
            if *rec {
                // If the function is recursive, we must remove the function's bindings from its value too
                remove_bound_variables(name, acc);
            }
        },
        AST::FunctionCall { callee, argument } => {
            compute_fv(callee, acc);
            compute_fv(argument, acc);
        },
        AST::Operation(_, members) => members.iter().for_each(|m| compute_fv(m, acc)),
        AST::Case(value, clauses) => {
            compute_fv(value, acc);
            clauses.iter().for_each(|cl| {
                if let Some(g) = &cl.guard {
                    compute_fv(g, acc);
                }
                compute_fv(&cl.body, acc);
                remove_bound_variables(&cl.pattern, acc);
            });
        },
        AST::Conditional(condition, branch1, branch2) => {
            compute_fv(condition, acc);
            compute_fv(branch1, acc);
            if let Some(b2) = branch2.as_ref() {
                compute_fv(b2, acc);
            }
        },
        AST::Tuple(values) | AST::List(values) => values.iter().for_each(|v| compute_fv(v, acc)),
        AST::Unit | AST::Number(_) | AST::StringLiteral(_) | AST::Boolean(_) | AST::SecurityLevel(_) | AST::Unreachable => (),
        AST::Identifier(id) => { acc.insert(id.clone()); }
        AST::Lambda(param, body) => {
            compute_fv(body, acc);
            if let Some(p) = param.as_ref() {
                acc.remove(p);
            }
        }
    }
}

fn remove_bound_variables(pat: &Pattern, acc: &mut HashSet<String>) {
    match pat {
        Pattern::Empty | Pattern::Value(_) => (),
        Pattern::Variable(s) => { acc.remove(s); }
        Pattern::Tuple(pats) => pats.iter().for_each(|p| remove_bound_variables(p, acc)),
    }
}