#![allow(dead_code)]

use std::collections::HashMap;

use crate::parser::{Pattern, AST};

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Type {
    String,
    Wildcard,
    Int,
    Unit,
    Bool,
    Label,
    Lambda(Box<Type>, Box<Type>),
    List(Box<Type>),
    Tuple(Vec<Type>),
    Gen(u32),
}

type Constraint = (Type, Type);

#[derive(Debug)]
pub enum TypeError {
    InvalidType,
    UnknownVariable(String),
    InvalidOperator(String),
    TypeMismatchConstraint(Type, Type),
    ToManyOpArguments,
    TypeCycle,
    InvalidMatch,
}

struct Ctx {
    fresh: u32,
    constraints: Vec<Constraint>,
    name_map: HashMap<String, Type>,
}

impl Ctx {
    fn new() -> Self {
        Ctx {
            fresh: 0,
            constraints: Vec::new(),
            name_map: HashMap::new(),
        }
    }

    fn get_builtin_type(&mut self, s: &str) -> Option<Type> {
        Some(match s {
            "sleep" => Type::Lambda(Box::new(Type::Int), Box::new(Type::Unit)),
            "send" => Type::Lambda(Box::new(self.get_fresh()), Box::new(Type::Unit)),
            "receive" => {
                let t = Box::new(self.get_fresh());
                Type::Lambda(Box::new(Type::Lambda(Box::new(Type::Unit), t.clone())), t)
            }
            "self" | "mkuuid" => Type::Lambda(Box::new(Type::Unit), Box::new(Type::Int)),
            "spawn" => Type::Lambda(
                Box::new(Type::Lambda(
                    Box::new(Type::Unit),
                    Box::new(self.get_fresh()),
                )),
                Box::new(Type::Int),
            ),
            "print" => Type::Lambda(Box::new(Type::String), Box::new(Type::Unit)),
            _ => return None,
        })
    }

    fn get_fresh(&mut self) -> Type {
        self.fresh += 1;
        Type::Gen(self.fresh - 1)
    }

    fn insert(&mut self, var: String, t: Type) {
        self.name_map.insert(var, t);
    }

    fn remove(&mut self, var: &str) -> Option<Type> {
        self.name_map.remove(var)
    }

    fn get(&self, var: &str) -> Option<&Type> {
        self.name_map.get(var)
    }

    fn insert_new_variable(&mut self, var: &AST, t: Type) -> Result<(), TypeError> {
        match var {
            AST::Identifier(name) => self.insert(name.to_owned(), t),
            AST::Tuple(variables) => {
                let fresh_types: Vec<_> = variables.iter().map(|_| self.get_fresh()).collect();

                self.constraints.push((t, Type::Tuple(fresh_types.clone())));

                for (var, t) in variables.iter().zip(fresh_types) {
                    self.insert_new_variable(var, t)?;
                }
            }
            _ => (),
        };
        Ok(())
    }

    fn remove_variable(&mut self, var: &AST) -> Result<(), TypeError> {
        match var {
            AST::Identifier(name) => {
                self.remove(name);
            }
            AST::Tuple(variables) => {
                for var in variables {
                    self.remove_variable(var)?
                }
            }
            _ => (),
        };
        Ok(())
    }
}

fn get_type_constraints(ast: &AST, ctx: &mut Ctx) -> Result<Type, TypeError> {
    Ok(match ast {
        AST::Let {
            name,
            value,
            body,
            rec,
        } => match name {
            Pattern::Single(child) => {
                let t = if *rec {
                    let rec_gen = ctx.get_fresh();
                    ctx.insert_new_variable(&child, rec_gen.clone())?;
                    let t = get_type_constraints(value, ctx)?;
                    ctx.constraints.push((rec_gen, t.clone()));
                    t
                } else {
                    get_type_constraints(value, ctx)?
                };
                ctx.insert_new_variable(child, t)?;
                let res = get_type_constraints(body, ctx)?;
                ctx.remove_variable(child)?;
                res
            }
            Pattern::Tuple(variables) => {
                let variable = AST::Tuple(variables.to_owned());
                let t_value = get_type_constraints(value, ctx)?;

                ctx.insert_new_variable(&variable, t_value)?;
                let res = get_type_constraints(body, ctx)?;
                ctx.remove_variable(&variable)?;

                res
            }
            Pattern::Empty => {
                get_type_constraints(value, ctx)?;
                get_type_constraints(body, ctx)?
            }
        },
        AST::FunctionCall { callee, argument } => {
            let arg_fresh = ctx.get_fresh();
            let res_fresh = ctx.get_fresh();

            let t_callee = get_type_constraints(callee, ctx)?;
            let t_arg = get_type_constraints(argument, ctx)?;

            ctx.constraints.push((t_arg, arg_fresh.clone()));
            ctx.constraints.push((
                t_callee,
                Type::Lambda(Box::new(arg_fresh), Box::new(res_fresh.clone())),
            ));

            res_fresh
        }
        AST::Operation(op, values) => {
            let (expected, res) = match &op as &str {
                "raisedTo" => {
                    let fresh = ctx.get_fresh();
                    (vec![fresh.clone(), Type::Label], fresh)
                }
                "andalso" => (vec![Type::Bool; 2], Type::Bool),
                "orelse" => (vec![Type::Bool; 2], Type::Bool),
                "+" => (vec![Type::Int; 2], Type::Int),
                "-" => (vec![Type::Int; 2], Type::Int),
                "*" => (vec![Type::Int; 2], Type::Int),
                "/" => (vec![Type::Int; 2], Type::Int),
                "<=" => (vec![Type::Int; 2], Type::Bool),
                "=" => {
                    let t1 = ctx.get_fresh();
                    let t2 = ctx.get_fresh();
                    ctx.constraints.push((t1.clone(), t2.clone()));
                    (vec![t1, t2], Type::Bool)
                }
                ">=" => (vec![Type::Int; 2], Type::Bool),
                "<" => (vec![Type::Int; 2], Type::Bool),
                "=>" => (vec![Type::Int; 2], Type::Bool),
                ">" => (vec![Type::Int; 2], Type::Bool),
                "::" => {
                    let t = ctx.get_fresh();
                    let list = Type::List(Box::new(t.clone()));
                    (vec![t, list.clone()], list)
                }
                _ => return Err(TypeError::InvalidOperator(op.to_string())),
            };

            let actual: Vec<_> = values
                .iter()
                .map(|v| get_type_constraints(v, ctx))
                .collect::<Result<Vec<Type>, TypeError>>()?;

            ctx.constraints
                .append(&mut actual.into_iter().zip(expected).collect::<Vec<_>>());

            res
        }
        AST::Match(_, body) => {
            get_type_constraints(body, ctx)?;
            Type::Bool
        }
        AST::Conditional(ife, then, els) => match &ife as &AST {
            AST::Match(pat, body) => {
                get_type_constraints(ife, ctx)?;

                let t1 = get_type_constraints(
                    &AST::Let {
                        name: pat.to_owned(),
                        value: body.to_owned(),
                        body: then.to_owned(),
                        rec: false,
                    },
                    ctx,
                )?;

                let t2 = if let Some(els) = els {
                    get_type_constraints(els, ctx)?
                } else {
                    Type::Unit
                };

                ctx.constraints.push((t2.clone(), t1));
                t2
            }
            _ => {
                let t1 = get_type_constraints(ife, ctx)?;
                ctx.constraints.push((t1, Type::Bool));

                let t2 = get_type_constraints(then, ctx)?;

                let t3 = if let Some(els) = els {
                    get_type_constraints(els, ctx)?
                } else {
                    Type::Unit
                };

                ctx.constraints.push((t2.clone(), t3));
                t2
            }
        },
        AST::Tuple(values) => {
            let mut types = Vec::with_capacity(values.len());
            for v in values {
                types.push(get_type_constraints(v, ctx)?);
            }

            Type::Tuple(types)
        }
        AST::List(values) if values.is_empty() => Type::List(Box::new(ctx.get_fresh())),
        AST::List(values) => {
            let t1 = get_type_constraints(&values[0], ctx)?;

            for v in &values[1..] {
                let t2 = get_type_constraints(v, ctx)?;
                ctx.constraints.push((t1.clone(), t2));
            }

            Type::List(Box::new(t1))
        }
        AST::Lambda(arg, body) => {
            let (t1, to_remove) = if let Some(arg) = arg {
                let t1 = ctx.get_fresh();
                ctx.insert(arg.to_string(), t1.clone());
                (t1, Some(arg.to_string()))
            } else {
                (Type::Unit, None)
            };

            let t2 = get_type_constraints(body, ctx)?;
            if let Some(arg) = to_remove {
                ctx.remove(&arg);
            }

            Type::Lambda(Box::new(t1), Box::new(t2))
        }
        AST::Unit => Type::Unit,
        AST::Wildcard => Type::Wildcard,
        AST::Number(_) => Type::Int,
        AST::StringLiteral(_) => Type::String,
        AST::Boolean(_) => Type::Bool,
        AST::Identifier(ident) => match ctx
            .get(ident)
            .cloned()
            .or_else(|| ctx.get_builtin_type(ident))
        {
            Some(t) => t,
            None => return Err(TypeError::UnknownVariable(ident.to_string())),
        },
        AST::SecurityLevel(_) => Type::Label,
    })
}

fn deep_substitute(current: &mut Type, from: &Type, to: &Type) {
    if current == from {
        *current = to.clone();
        return;
    }

    match current {
        Type::Lambda(t1, t2) => {
            deep_substitute(t1, from, to);
            deep_substitute(t2, from, to);
        }
        Type::List(t) => deep_substitute(t, from, to),
        Type::Tuple(types) => {
            for t in types {
                deep_substitute(t, from, to)
            }
        }
        _ => (),
    }
}

fn type_contains(t1: &Type, t2: &Type) -> bool {
    if t1 == t2 {
        true
    } else {
        match t1 {
            Type::Lambda(from, to) => type_contains(from, t2) || type_contains(to, t2),
            Type::List(t) => type_contains(t, t2),
            Type::Tuple(types) => types.iter().any(|t| type_contains(t, t2)),
            _ => false,
        }
    }
}

fn process_substitution(
    t: &Type,
    g: &Type,
    constraints: &mut Vec<Constraint>,
) -> Result<(), TypeError> {
    if type_contains(t, g) {
        return Err(TypeError::TypeCycle);
    }
    for constraint in constraints.iter_mut() {
        deep_substitute(&mut constraint.0, g, t);
        deep_substitute(&mut constraint.1, g, t);
    }

    Ok(())
}

fn resolve_constraints(mut constraints: Vec<Constraint>) -> Result<(), TypeError> {
    constraints.reverse();
    while let Some((t1, t2)) = constraints.pop() {
        match (t1, t2) {
            (t1, t2) if t1 == t2 => continue,
            (Type::Lambda(a1, b1), Type::Lambda(a2, b2)) => {
                constraints.push((*a1, *a2));
                constraints.push((*b1, *b2));
            }
            (Type::Tuple(t1), Type::Tuple(t2)) if t1.len() == t2.len() => {
                for (a, b) in t1.into_iter().zip(t2) {
                    constraints.push((a, b));
                }
            }
            (Type::List(t1), Type::List(t2)) => {
                constraints.push((*t1, *t2));
            }
            (Type::Gen(g), t) | (t, Type::Gen(g)) => {
                process_substitution(&t, &Type::Gen(g), &mut constraints)?
            }
            (t1, t2) => return Err(TypeError::TypeMismatchConstraint(t1, t2)),
        }
    }

    Ok(())
}

pub fn type_check(ast: AST) -> Result<(), TypeError> {
    let mut ctx = Ctx::new();
    get_type_constraints(&ast, &mut ctx)?;
    resolve_constraints(ctx.constraints)
}
