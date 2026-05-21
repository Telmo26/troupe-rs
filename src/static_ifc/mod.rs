use std::collections::{HashMap, HashSet};

use crate::{
    parser::{AST, Pattern},
    static_ifc::primitives::{PrimitiveApp, match_primitive}, trustmap::TrustMap,
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

#[derive(Clone, Copy, Debug)]
enum Safety {
    Safe,
    Dangerous,
    DangerousMaybe,
}

impl Safety {
    fn with2(self, other: Self) -> Self {
        match (self, other) {
            (Self::Dangerous, _) => self,
            (_, Self::Dangerous | Self::DangerousMaybe) => other,
            _ => self,
        }
    }

    fn with(&mut self, other: Self) {
        *self = self.with2(other)
    }
}

#[derive(Clone, Debug)]
struct IfcExpEval {
    level: Level,
    is_arg: bool,
    safety: Safety,
}

impl IfcExpEval {
    fn new(level: Level, safety: Safety, is_arg: bool) -> Self {
        Self {
            level,
            is_arg,
            safety,
        }
    }

    fn empty(safety: Safety) -> Self {
        Self {
            safety,
            level: HashSet::new(),
            is_arg: false,
        }
    }
}

#[derive(Clone)]
struct Ctx {
    level: Level,
    strict: bool,
    map: HashMap<String, IfcExpEval>,
    trustmap: Option<TrustMap>
}

impl Ctx {
    fn new(strict: bool, trustmap: Option<TrustMap>) -> Self {
        let map = [
            ("authority".to_string(), IfcExpEval::empty(Safety::Safe)),
            (
                "exitAfterTimeout".to_string(),
                IfcExpEval::empty(Safety::DangerousMaybe),
            ),
            (
                "sleep".to_string(),
                IfcExpEval::empty(Safety::DangerousMaybe),
            ),
            (
                "send".to_string(),
                IfcExpEval::empty(Safety::DangerousMaybe),
            ),
            (
                "print".to_string(),
                IfcExpEval::empty(Safety::DangerousMaybe),
            ),
            (
                "receive".to_string(),
                IfcExpEval::empty(Safety::DangerousMaybe),
            ),
            (
                "spawn".to_string(),
                IfcExpEval::empty(Safety::DangerousMaybe),
            ),
            (
                "self".to_string(),
                IfcExpEval::empty(Safety::DangerousMaybe),
            ),
            ("mkuuid".to_string(), IfcExpEval::empty(Safety::Safe)),
            ("declassify".to_string(), IfcExpEval::empty(Safety::Safe)),
        ]
        .into_iter()
        .collect::<HashMap<_, _>>();

        Self {
            map,
            level: Level::new(),
            strict,
            trustmap
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
    let mut e = IfcExpEval::empty(Safety::Safe);
    for value in values {
        let res = ifc_check(value, ctx)?;
        e.level.extend(res.level);
        e.safety.with(res.safety);
        e.is_arg |= res.is_arg;
    }

    Ok(e)
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

fn ifc_check(ast: &AST, ctx: &mut Ctx) -> Result<IfcExpEval, StaticIfcError> {
    Ok(match ast {
        AST::Let {
            name,
            value,
            body,
            rec,
        } => {
            let var_e = if *rec {
                let mut rec_ctx = ctx.clone();

                let initial = IfcExpEval::new(
                    if ctx.strict {
                        strict_level()
                    } else {
                        Level::new()
                    },
                    Safety::Safe,
                    false,
                );

                bind_pattern_variables(name, &initial, &mut rec_ctx);

                ifc_check(value, &mut rec_ctx)?
            } else {
                ifc_check(value, ctx)?
            };

            with_pattern_bound(
                name,
                &IfcExpEval::new(var_e.level.clone(), var_e.safety, false),
                ctx,
                |fresh_ctx| {
                    let mut in_e = ifc_check(body, fresh_ctx)?;
                    in_e.level.extend(var_e.level.clone());
                    Ok(in_e)
                },
            )?
        }

        AST::Case(expr, clauses) => {
            let expr_e = ifc_check(expr, ctx)?;

            let old_pc = ctx.level.clone();

            ctx.level.extend(expr_e.level.clone());

            let mut res = IfcExpEval::new(expr_e.level.clone(), Safety::Safe, false);

            for clause in clauses {
                let clause_e = with_pattern_bound(&clause.pattern, &expr_e, ctx, |clause_ctx| {
                    if let Some(guard) = &clause.guard {
                        let guard_res = ifc_check(guard, clause_ctx)?;

                        clause_ctx.level.extend(guard_res.level.clone());

                        let mut body_e = ifc_check(&clause.body, clause_ctx)?;
                        body_e.level.extend(guard_res.level);

                        Ok(body_e)
                    } else {
                        ifc_check(&clause.body, clause_ctx)
                    }
                })?;

                res.level.extend(clause_e.level);
                res.safety.with(clause_e.safety);
                res.is_arg |= clause_e.is_arg;
            }

            ctx.level = old_pc;

            res
        }
        AST::FunctionCall { callee, argument } => {
            let mut arg_ctx = ctx.clone();

            let mut body_ctx = ctx.clone();
            let callee_e = ifc_check(callee, &mut body_ctx)?;

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
                    },
                    PrimitiveApp::Send { target, value } => {
                        let ifc_exp = ifc_check(value, ctx)?;
                        
                        if let AST::StringLiteral(destination) = target {
                            match &ctx.trustmap {
                                Some(map) if let Some(allowed_levels) = map.get(destination) => {
                                    if allowed_levels.is_superset(&ifc_exp.level) {
                                        return Ok(IfcExpEval::empty(Safety::Safe))
                                    } else {
                                        return Err(StaticIfcError::IOOperationOnSecretVariables)
                                    }
                                },
                                _ => if ifc_exp.level.is_empty() {
                                    return Ok(IfcExpEval::empty(Safety::Safe))
                                } else {
                                    return Err(StaticIfcError::IOOperationOnSecretVariables)
                                }
                            }
                        }
                    }
                }
            }

            let arg_e = ifc_check(argument, &mut arg_ctx)?;

            let mut result = IfcExpEval::new(
                arg_e.level,
                callee_e.safety.with2(arg_e.safety),
                callee_e.is_arg || arg_e.is_arg,
            );
            result.level.extend(callee_e.level);

            match result.safety {
                Safety::Dangerous => {
                    if !ctx.level.is_empty() || !result.level.is_empty() {
                        return Err(StaticIfcError::IOOperationOnSecretVariables);
                    }
                }
                Safety::DangerousMaybe => {
                    if !ctx.level.is_empty() {
                        return Err(StaticIfcError::IOOperationOnSecretVariables);
                    }
                    if result.is_arg {
                        result.safety = Safety::Dangerous
                    }
                }
                _ => (),
            }

            result
        }
        AST::Lambda(arg, body) => {
            let mut fresh_ctx = Ctx::new(ctx.strict, ctx.trustmap.clone());
            fresh_ctx.map = ctx.map.clone();

            if let Some(arg) = arg {
                let arg_level = if ctx.strict {
                    strict_level()
                } else {
                    Level::new()
                };
                fresh_ctx.insert_variable(
                    arg.to_owned(),
                    IfcExpEval::new(arg_level, Safety::Safe, true),
                );
            }

            ifc_check(body, &mut fresh_ctx)?
        }
        AST::Conditional(ife, then, els) => {
            let old_ctx_level = ctx.level.clone();

            // We first check if and extend the ctx level.
            let if_e = ifc_check(ife, ctx)?;

            let mut result = if_e.clone();
            ctx.level.extend(if_e.level);

            // Then check the then
            let then_e = ifc_check(then, ctx)?;
            result.level.extend(then_e.level.clone());
            result.safety.with(then_e.safety);
            result.is_arg |= then_e.is_arg;

            if let Some(els) = els {
                // If there is an else, we need to extend the ctx level
                // since it depends on the then branch.
                ctx.level.extend(then_e.level);
                let els_e = ifc_check(els, ctx)?;
                result.safety.with(els_e.safety);
                result.is_arg |= els_e.is_arg;
                result.level.extend(els_e.level);

                // And we also need to recheck the then branch since it depends on els.
                // The level evaluation should be the same so we drop it.
                ifc_check(then, ctx)?;
            }

            ctx.level = old_ctx_level;

            result
        }
        AST::Tuple(values) | AST::List(values) | AST::Operation(_, values) => unite(values, ctx)?,
        AST::Unit | AST::Unreachable | AST::Number(_) | AST::StringLiteral(_) | AST::Boolean(_) => {
            IfcExpEval::empty(Safety::Safe)
        }
        AST::Identifier(ident) => match ctx.get(ident) {
            Some(level) => level.to_owned(),
            None => return Err(StaticIfcError::UnknownVariable(ident.to_owned())),
        },
        AST::SecurityLevel(level) => {
            let level = if level.is_empty() {
                Level::new()
            } else {
                level.iter()
                    .map(|l| l.to_owned())
                    .collect()
            };
            IfcExpEval::new(level, Safety::Safe, false)
        }
    })
}

pub fn static_ifc_check(ast: &AST, strict: bool, trustmap: Option<TrustMap>) -> Result<Level, StaticIfcError> {
    let mut ctx = Ctx::new(strict, trustmap);
    ifc_check(ast, &mut ctx).map(|r| r.level)
}
