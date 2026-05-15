use std::collections::{HashMap, HashSet};

use crate::{
    parser::{Pattern, AST},
    static_ifc::primitives::{match_primitive, PrimitiveApp},
};

mod primitives;

#[derive(Debug)]
#[allow(dead_code)]
pub enum StaticIfcError {
    ObservableEffectUnderSecretPc,
    IOOperationOnSecretVariables,
    UnknownVariable(String),
    InvalidDeclassification,
}

type Level = HashSet<String>;

fn strict_level() -> Level {
    HashSet::from(["@strict_private_label".to_string()])
}

#[derive(Clone)]
struct Ctx {
    level: Level,
    strict: bool,
    map: HashMap<String, Level>,
}

impl Ctx {
    fn new(strict: bool) -> Self {
        Self {
            map: HashMap::new(),
            level: Level::new(),
            strict,
        }
    }

    fn insert_variable(&mut self, name: String, level: Level) {
        self.map.insert(name, level);
    }

    fn get(&self, name: &str) -> Option<&Level> {
        self.map.get(name)
    }
}

fn unite(values: &[AST], ctx: &mut Ctx) -> Result<Level, StaticIfcError> {
    let mut level = Level::new();
    for value in values {
        let l: Level = ifc_check(value, ctx)?;
        level.extend(l);
    }

    Ok(level)
}

fn check_observable(ctx: &Ctx) -> Result<(), StaticIfcError> {
    if !ctx.level.is_empty() {
        return Err(StaticIfcError::ObservableEffectUnderSecretPc);
    }

    Ok(())
}

fn bind_pattern_variables(pattern: &Pattern, level: &Level, ctx: &mut Ctx) {
    match pattern {
        Pattern::Single(ast) => {
            if let AST::Identifier(name) = ast.as_ref() {
                ctx.insert_variable(name.clone(), level.clone());
            }
        }

        Pattern::Tuple(patterns) => {
            for pattern in patterns {
                bind_pattern_variables(pattern, level, ctx);
            }
        }

        Pattern::Empty => {}
    }
}

fn with_pattern_bound<F, R>(pattern: &Pattern, level: &Level, ctx: &Ctx, f: F) -> R
where
    F: FnOnce(&mut Ctx) -> R,
{
    let mut fresh_ctx = ctx.clone();

    bind_pattern_variables(pattern, level, &mut fresh_ctx);

    f(&mut fresh_ctx)
}

fn ifc_check(ast: &AST, ctx: &mut Ctx) -> Result<Level, StaticIfcError> {
    Ok(match ast {
        AST::Let {
            name,
            value,
            body,
            rec,
        } => {
            let value_level = if *rec {
                let mut rec_ctx = ctx.clone();

                let initial = if ctx.strict {
                    strict_level()
                } else {
                    Level::new()
                };

                bind_pattern_variables(name, &initial, &mut rec_ctx);

                ifc_check(value, &mut rec_ctx)?
            } else {
                ifc_check(value, ctx)?
            };

            with_pattern_bound(name, &value_level, ctx, |fresh_ctx| {
                let mut body_level = ifc_check(body, fresh_ctx)?;
                body_level.extend(value_level.clone());
                Ok(body_level)
            })?
        }

        AST::Case(expr, clauses) => {
            let expr_level = ifc_check(expr, ctx)?;

            let old_pc = ctx.level.clone();

            ctx.level.extend(expr_level.clone());

            let mut result_level = expr_level.clone();

            for clause in clauses {
                let clause_level =
                    with_pattern_bound(&clause.pattern, &expr_level, ctx, |clause_ctx| {
                        if let Some(guard) = &clause.guard {
                            let guard_level = ifc_check(guard, clause_ctx)?;

                            clause_ctx.level.extend(guard_level.clone());

                            let mut body_level = ifc_check(&clause.body, clause_ctx)?;
                            body_level.extend(guard_level);

                            Ok(body_level)
                        } else {
                            ifc_check(&clause.body, clause_ctx)
                        }
                    })?;

                result_level.extend(clause_level);
            }

            ctx.level = old_pc;

            result_level
        }
        AST::FunctionCall { callee, argument } => {
            let mut arg_ctx = ctx.clone();

            let mut body_ctx = ctx.clone();
            let mut callee_level = ifc_check(callee, &mut body_ctx)?;

            if let Some(primitive) = match_primitive(ast) {
                match primitive {
                    PrimitiveApp::Print { value }
                    | PrimitiveApp::Sleep { value }
                    | PrimitiveApp::Send { value }
                    | PrimitiveApp::Receive { value }
                    | PrimitiveApp::Spawn { value }
                    | PrimitiveApp::ExitAfterTimeout { value } => {
                        let value_level = ifc_check(value, ctx)?;

                        if !value_level.is_empty() {
                            return Err(StaticIfcError::IOOperationOnSecretVariables);
                        }

                        check_observable(ctx)?;

                        value_level
                    }

                    PrimitiveApp::Declassify {
                        value,
                        authority,
                        target,
                    } => {
                        check_observable(ctx)?;

                        ifc_check(value, ctx)?;
                        ifc_check(authority, ctx)?;

                        ifc_check(target, ctx)?
                    }

                    PrimitiveApp::Self_ | PrimitiveApp::MkUuid | PrimitiveApp::Authority => {
                        Level::new()
                    }
                }
            } else {
                let arg_level = ifc_check(argument, &mut arg_ctx)?;
                callee_level.extend(arg_level);
                callee_level
            }
        }
        AST::Lambda(arg, body) => {
            let mut fresh_ctx = Ctx::new(ctx.strict);
            fresh_ctx.map = ctx.map.clone();

            if let Some(arg) = arg {
                let arg_level = if ctx.strict {
                    strict_level()
                } else {
                    Level::new()
                };
                fresh_ctx.insert_variable(arg.to_owned(), arg_level);
            }

            ifc_check(body, &mut fresh_ctx)?
        }
        AST::Conditional(ife, then, els) => {
            let old_ctx_level = ctx.level.clone();

            // We first check if and extend the ctx level.
            let if_level = ifc_check(ife, ctx)?;
            let mut level = if_level.clone();
            ctx.level.extend(if_level);

            // Then check the then
            let then_level = ifc_check(then, ctx)?;
            level.extend(then_level.clone());

            if let Some(els) = els {
                // If there is an else, we need to extend the ctx level
                // since it depends on the then branch.
                ctx.level.extend(then_level);
                let els_level = ifc_check(els, ctx)?;
                level.extend(els_level);

                // And we also need to recheck the then branch since it depends on els.
                // The level evaluation should be the same so we drop it.
                ifc_check(then, ctx)?;
            }

            ctx.level = old_ctx_level;

            level
        }
        AST::Tuple(values) | AST::List(values) | AST::Operation(_, values) => unite(values, ctx)?,
        AST::Unit | AST::Wildcard | AST::Number(_) | AST::StringLiteral(_) | AST::Boolean(_) => {
            Level::new()
        }
        AST::Identifier(ident) => match ctx.get(ident) {
            Some(level) => level.to_owned(),
            None => match ident.as_str() {
                "authority" | "exitAfterTimeout" | "sleep" | "send" | "print" | "receive"
                | "spawn" | "self" | "mkuuid" | "declassify" => Level::new(),
                _ => return Err(StaticIfcError::UnknownVariable(ident.to_owned())),
            },
        },
        AST::SecurityLevel(level) => Level::from([level.to_owned()]),
    })
}

pub fn static_ifc_check(ast: &AST, strict: bool) -> Result<Level, StaticIfcError> {
    let mut ctx = Ctx::new(strict);
    ifc_check(ast, &mut ctx)
}
