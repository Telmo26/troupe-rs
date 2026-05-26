use std::collections::{HashMap, BTreeSet};

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

type Level = BTreeSet<String>;

fn strict_level() -> Level {
    BTreeSet::from(["@strict_private_label".to_string()])
}

#[derive(Clone, Debug, Default)]
struct IfcExpEval {
    level: Level,
    is_arg: bool,
    argument_constraints: Vec<Option<Level>>,
}

impl IfcExpEval {
    fn new(level: Level, is_arg: bool, argument_constraints: Vec<Option<Level>>) -> Self {
        Self {
            level,
            is_arg,
            argument_constraints
        }
    }
}

#[derive(Debug, Clone)]
struct Ctx {
    level: Level,
    strict: bool,
    map: HashMap<String, IfcExpEval>,
    trustmap: Option<TrustMap>
}

impl Ctx {
    fn new(strict: bool, trustmap: Option<TrustMap>) -> Self {
        let map = [
            (
                "authority".to_string(), 
                IfcExpEval::default()
            ),
            (
                "exitAfterTimeout".to_string(),
                IfcExpEval::default(),
            ),
            (
                "sleep".to_string(),
                IfcExpEval::default(),
            ),
            (
                "send".to_string(),
                IfcExpEval::new(Level::new(), false, vec![Some(BTreeSet::new())]),
            ),
            (
                "adv".to_string(),
                IfcExpEval::new(Level::new(), false, vec![Some(BTreeSet::new())]),
            ),
            (
                "print".to_string(),
                IfcExpEval::default(),
            ),
            (
                "printString".to_string(),
                IfcExpEval::default(),
            ),
            (
                "receive".to_string(),
                IfcExpEval::default(),
            ),
            (
                "spawn".to_string(),
                IfcExpEval::default(),
            ),
            (
                "self".to_string(),
                IfcExpEval::default(),
            ),
            (
                "mkuuid".to_string(),
                IfcExpEval::default(),
            ),
            (
                "declassify".to_string(), 
                IfcExpEval::default()
            ),
            (
                "node".to_string(),
                IfcExpEval::default(),
            ),
            (
                "whereis".to_string(),
                IfcExpEval::default()
            ),
            (
                "register".to_string(),
                IfcExpEval::default(),
            ),
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
    let mut e = IfcExpEval::default();
    for value in values {
        let res = ifc_check(value, ctx)?;
        e.level.extend(res.level);
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
                    false,
                    vec![None]
                );

                bind_pattern_variables(name, &initial, &mut rec_ctx);

                ifc_check(value, &mut rec_ctx)?
            } else {
                ifc_check(value, ctx)?
            };

            with_pattern_bound(
                name,
                &IfcExpEval::new(var_e.level.clone(), false, var_e.argument_constraints),
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

            let mut res = IfcExpEval::new(expr_e.level.clone(), false, vec![None]);

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
                res.is_arg |= clause_e.is_arg;
            }

            ctx.level = old_pc;

            res
        }
        AST::FunctionCall { callee, argument } => {
            let mut arg_ctx = ctx.clone();

            let mut body_ctx = ctx.clone();
            let mut callee_e = ifc_check(callee, &mut body_ctx)?;

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
                    PrimitiveApp::Send { 
                        target, 
                        value 
                    } => {
                        let ifc_exp = ifc_check(value, ctx)?;
                        
                        match &ctx.trustmap {
                            Some(map) if let Some(allowed_levels) = map.get(&target) => {
                                if allowed_levels.is_superset(&ifc_exp.level) {
                                    return Ok(IfcExpEval::new(
                                        Level::new(), 
                                        ifc_exp.is_arg,
                                        vec![Some(allowed_levels.clone())]
                                    ))
                                } else {
                                    return Err(StaticIfcError::IOOperationOnSecretVariables)
                                }
                            },
                            _ => if ifc_exp.level.is_empty() {
                                return Ok(IfcExpEval::new(Level::new(), ifc_exp.is_arg, vec![Some(BTreeSet::new())]))
                            } else {
                                return Err(StaticIfcError::IOOperationOnSecretVariables)
                            }
                        }
                    }
                }
            }

            let arg_e = ifc_check(argument, &mut arg_ctx)?;

            // If the argument is a parameter, the constraint must not be removed,
            // since we do not know the label of the value to be substituted in
            if !arg_e.is_arg {
                // We get the constraint for this level of the call
                let current_constraint = callee_e.argument_constraints.pop();

                // If the PC label or the value's label is higher than the one authorized by the called function
                if let Some(Some(allowed_levels)) = current_constraint {
                    if !allowed_levels.is_superset(&ctx.level) || !allowed_levels.is_superset(&arg_e.level) {
                        return Err(StaticIfcError::IOOperationOnSecretVariables);
                    }
                }
            }
            
            // We propagate the constraints
            let mut result = IfcExpEval::new(
                arg_e.level,
                callee_e.is_arg || arg_e.is_arg,
                callee_e.argument_constraints,
            );
            result.level.extend(callee_e.level);

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
                    IfcExpEval::new(arg_level, true, vec![None]),
                );
            }

            let mut result = ifc_check(body, &mut fresh_ctx)?;

            // If it doesn't use the parameter, we add an additional no-constraint on it
            if !result.is_arg { 
                result.argument_constraints.push(None);
            }

            result
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
            result.is_arg |= then_e.is_arg;

            if let Some(els) = els {
                // If there is an else, we need to extend the ctx level
                // since it depends on the then branch.
                ctx.level.extend(then_e.level);
                let els_e = ifc_check(els, ctx)?;
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
            IfcExpEval::default()
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
            IfcExpEval::new(level,false, vec![None])
        }
    })
}

pub fn static_ifc_check(ast: &AST, strict: bool, trustmap: Option<TrustMap>) -> Result<Level, StaticIfcError> {
    let mut ctx = Ctx::new(strict, trustmap);
    ifc_check(ast, &mut ctx).map(|r| r.level)
}
