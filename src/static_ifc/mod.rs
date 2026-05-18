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

type IfcExpEval = (Level, bool);

#[derive(Clone)]
struct Ctx {
    level: Level,
    strict: bool,
    map: HashMap<String, IfcExpEval>,
}

impl Ctx {
    fn new(strict: bool) -> Self {
        let map = [
            ("authority".to_string(), (Level::new(), false)),
            ("exitAfterTimeout".to_string(), (Level::new(), false)),
            ("sleep".to_string(), (Level::new(), false)),
            ("send".to_string(), (Level::new(), false)),
            ("print".to_string(), (Level::new(), false)),
            ("receive".to_string(), (Level::new(), false)),
            ("spawn".to_string(), (Level::new(), false)),
            ("self".to_string(), (Level::new(), false)),
            ("mkuuid".to_string(), (Level::new(), false)),
            ("declassify".to_string(), (Level::new(), false)),
        ]
        .into_iter()
        .collect::<HashMap<_, _>>();

        Self {
            map,
            level: Level::new(),
            strict,
        }
    }

    fn insert_variable(&mut self, name: String, e: IfcExpEval) {
        self.map.insert(name, e);
    }

    fn get(&self, name: &str) -> Option<&IfcExpEval> {
        self.map.get(name)
    }
}

fn unite(values: &[AST], ctx: &mut Ctx) -> Result<IfcExpEval, StaticIfcError> {
    let mut level = Level::new();
    let mut is_safe = true;
    for value in values {
        let res = ifc_check(value, ctx)?;
        level.extend(res.0);
        is_safe &= res.1;
    }

    Ok((level, is_safe))
}

fn bind_pattern_variables(pattern: &Pattern, e: &IfcExpEval, ctx: &mut Ctx) {
    match pattern {
        Pattern::Variable(name) => {
            ctx.insert_variable(name.clone(), e.clone());
        }

        Pattern::Tuple(patterns) => {
            for pattern in patterns {
                bind_pattern_variables(pattern, e, ctx);
            }
        }

        Pattern::Empty | Pattern::Value(_) => {}
    }
}

fn with_pattern_bound<F, R>(pattern: &Pattern, e: &IfcExpEval, ctx: &Ctx, f: F) -> R
where
    F: FnOnce(&mut Ctx) -> R,
{
    let mut fresh_ctx = ctx.clone();

    bind_pattern_variables(pattern, e, &mut fresh_ctx);

    f(&mut fresh_ctx)
}

fn ifc_check(ast: &AST, ctx: &mut Ctx) -> Result<(Level, bool), StaticIfcError> {
    Ok(match ast {
        AST::Let {
            name,
            value,
            body,
            rec,
        } => {
            let (value_level, var_is_safe) = if *rec {
                let mut rec_ctx = ctx.clone();

                let initial = (
                    if ctx.strict {
                        strict_level()
                    } else {
                        Level::new()
                    },
                    false,
                );

                bind_pattern_variables(name, &initial, &mut rec_ctx);

                ifc_check(value, &mut rec_ctx)?
            } else {
                ifc_check(value, ctx)?
            };

            with_pattern_bound(
                name,
                &(value_level.clone(), var_is_safe),
                ctx,
                |fresh_ctx| {
                    let (mut body_level, inexpr_is_safe) = ifc_check(body, fresh_ctx)?;
                    body_level.extend(value_level.clone());
                    Ok((body_level, inexpr_is_safe))
                },
            )?
        }

        AST::Case(expr, clauses) => {
            let (expr_level, expr_is_safe) = ifc_check(expr, ctx)?;

            let old_pc = ctx.level.clone();

            ctx.level.extend(expr_level.clone());

            let mut result_level = expr_level.clone();
            let mut result_is_safe = true;

            for clause in clauses {
                let (clause_level, clause_is_safe) = with_pattern_bound(
                    &clause.pattern,
                    &(expr_level.clone(), expr_is_safe),
                    ctx,
                    |clause_ctx| {
                        if let Some(guard) = &clause.guard {
                            let (guard_level, _) = ifc_check(guard, clause_ctx)?;

                            clause_ctx.level.extend(guard_level.clone());

                            let (mut body_level, body_is_safe) =
                                ifc_check(&clause.body, clause_ctx)?;
                            body_level.extend(guard_level);

                            Ok((body_level, body_is_safe))
                        } else {
                            ifc_check(&clause.body, clause_ctx)
                        }
                    },
                )?;

                result_level.extend(clause_level);
                result_is_safe &= clause_is_safe;
            }

            ctx.level = old_pc;

            (result_level, result_is_safe)
        }
        AST::FunctionCall { callee, argument } => {
            let mut arg_ctx = ctx.clone();

            let mut body_ctx = ctx.clone();
            let (mut callee_level, calle_is_safe) = ifc_check(callee, &mut body_ctx)?;

            if let Some(primitive) = match_primitive(ast) {
                match primitive {
                    PrimitiveApp::Declassify {
                        value,
                        authority,
                        target,
                    } => {
                        ifc_check(value, ctx)?;
                        ifc_check(authority, ctx)?;

                        return ifc_check(target, ctx);
                    }
                }
            }

            let (arg_level, arg_is_safe) = ifc_check(argument, &mut arg_ctx)?;
            callee_level.extend(arg_level);

            let is_safe = calle_is_safe && arg_is_safe;

            if !is_safe && (!ctx.level.is_empty() || !callee_level.is_empty()) {
                return Err(StaticIfcError::IOOperationOnSecretVariables);
            }

            (callee_level, is_safe)
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
                fresh_ctx.insert_variable(arg.to_owned(), (arg_level, true));
            }

            ifc_check(body, &mut fresh_ctx)?
        }
        AST::Conditional(ife, then, els) => {
            let old_ctx_level = ctx.level.clone();

            // We first check if and extend the ctx level.
            let (if_level, if_is_safe) = ifc_check(ife, ctx)?;
            let mut level = if_level.clone();
            let mut is_safe = if_is_safe;
            ctx.level.extend(if_level);

            // Then check the then
            let (then_level, then_is_safe) = ifc_check(then, ctx)?;
            level.extend(then_level.clone());
            is_safe &= then_is_safe;

            if let Some(els) = els {
                // If there is an else, we need to extend the ctx level
                // since it depends on the then branch.
                ctx.level.extend(then_level);
                let (els_level, els_is_safe) = ifc_check(els, ctx)?;
                is_safe &= els_is_safe;
                level.extend(els_level);

                // And we also need to recheck the then branch since it depends on els.
                // The level evaluation should be the same so we drop it.
                ifc_check(then, ctx)?;
            }

            ctx.level = old_ctx_level;

            (level, is_safe)
        }
        AST::Tuple(values) | AST::List(values) | AST::Operation(_, values) => unite(values, ctx)?,
        AST::Unit | AST::Unreachable | AST::Number(_) | AST::StringLiteral(_) | AST::Boolean(_) => {
            (Level::new(), true)
        }
        AST::Identifier(ident) => match ctx.get(ident) {
            Some(level) => level.to_owned(),
            None => return Err(StaticIfcError::UnknownVariable(ident.to_owned())),
        },
        AST::SecurityLevel(level) => {
            let level = if level == "{}" {
                Level::new()
            } else {
                Level::from([level.to_owned()])
            };
            (level, true)
        }
    })
}

pub fn static_ifc_check(ast: &AST, strict: bool) -> Result<Level, StaticIfcError> {
    let mut ctx = Ctx::new(strict);
    ifc_check(ast, &mut ctx).map(|r| r.0)
}
