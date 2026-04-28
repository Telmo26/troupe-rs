mod ctx;
mod scheme;
mod types;

use crate::{
    parser::{Pattern, AST},
    type_checker::{ctx::Ctx, scheme::Scheme, types::Type},
};

pub type Constraint = (Scheme, Scheme);

#[derive(Debug)]
#[allow(dead_code)]
pub enum TypeError {
    UnknownVariable(String),
    InvalidOperator(String),
    TypeMismatchConstraint(Scheme, Scheme),
    TypeCycle,
}

fn get_type_constraints(ast: &AST, ctx: &mut Ctx) -> Result<Scheme, TypeError> {
    Ok(match ast {
        AST::Let {
            name,
            value,
            body,
            rec,
        } => match name {
            Pattern::Single(_) => {
                let t = if *rec {
                    let rec_gen = ctx.get_fresh();
                    ctx.insert_new_variable(&name, rec_gen.clone());
                    let t = get_type_constraints(value, ctx)?;
                    ctx.push_constraint((rec_gen, t.clone()));
                    t
                } else {
                    get_type_constraints(value, ctx)?
                };
                ctx.insert_new_variable(&name, t);
                let res = get_type_constraints(body, ctx)?;
                ctx.remove_variable(&name);
                res
            }
            Pattern::Tuple(_) => {
                let t_value = get_type_constraints(value, ctx)?;

                ctx.insert_new_variable(&name, t_value);
                let res = get_type_constraints(body, ctx)?;
                ctx.remove_variable(&name);

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

            ctx.push_constraint((t_arg, arg_fresh.clone()));
            ctx.push_constraint((
                t_callee,
                Scheme::empty(Type::Lambda(
                    Box::new(arg_fresh),
                    Box::new(res_fresh.clone()),
                )),
            ));

            res_fresh
        }
        AST::Operation(op, values) => {
            let (expected, res) = match &op as &str {
                "raisedTo" => {
                    let fresh = ctx.get_fresh();
                    (vec![fresh.clone(), Scheme::label()], fresh)
                }
                "andalso" => (vec![Scheme::bool(); 2], Scheme::bool()),
                "orelse" => (vec![Scheme::bool(); 2], Scheme::bool()),
                "+" => (vec![Scheme::int(); 2], Scheme::int()),
                "-" => (vec![Scheme::int(); 2], Scheme::int()),
                "*" => (vec![Scheme::int(); 2], Scheme::int()),
                "/" => (vec![Scheme::int(); 2], Scheme::int()),
                "<=" => (vec![Scheme::int(); 2], Scheme::bool()),
                "=" => {
                    let t1 = ctx.get_fresh();
                    let t2 = ctx.get_fresh();
                    ctx.push_constraint((t1.clone(), t2.clone()));
                    (vec![t1, t2], Scheme::bool())
                }
                ">=" => (vec![Scheme::int(); 2], Scheme::bool()),
                "<" => (vec![Scheme::int(); 2], Scheme::bool()),
                "=>" => (vec![Scheme::int(); 2], Scheme::bool()),
                ">" => (vec![Scheme::int(); 2], Scheme::bool()),
                "::" => {
                    let t = ctx.get_fresh();
                    let list = Scheme::empty(Type::List(Box::new(t.clone())));
                    (vec![t, list.clone()], list)
                }
                _ => return Err(TypeError::InvalidOperator(op.to_string())),
            };

            let actual: Vec<_> = values
                .iter()
                .map(|v| get_type_constraints(v, ctx))
                .collect::<Result<Vec<_>, TypeError>>()?;

            for (c, e) in actual.into_iter().zip(expected).collect::<Vec<_>>() {
                ctx.push_constraint((c, e));
            }

            res
        }
        AST::Case(expr, clauses) => {
            let t_expr = match expr.as_ref() {
                AST::Identifier(name) if name == "_handlerInput" => {
                    let t = ctx.get_fresh();
                    ctx.insert(name.clone(), t.clone());
                    t
                }
                _ => get_type_constraints(expr, ctx)?,
            };

            let t_result = ctx.get_fresh();

            for clause in clauses {
                ctx.insert_new_variable(&clause.pattern, t_expr.clone());

                if let Some(guard) = &clause.guard {
                    let t_guard = get_type_constraints(guard, ctx)?;
                    ctx.push_constraint((t_guard, Scheme::bool()));
                }

                let t_body = get_type_constraints(&clause.body, ctx)?;
                ctx.push_constraint((t_body, t_result.clone()));

                ctx.remove_variable(&clause.pattern);
            }

            t_result
        }
        AST::Conditional(ife, then, els) => {
            let t1 = get_type_constraints(ife, ctx)?;
            ctx.push_constraint((t1, Scheme::bool()));

            let t2 = get_type_constraints(then, ctx)?;

            let t3 = if let Some(els) = els {
                get_type_constraints(els, ctx)?
            } else {
                Scheme::unit()
            };

            ctx.push_constraint((t2.clone(), t3));
            t2
        }
        AST::Tuple(values) => {
            let mut types = Vec::with_capacity(values.len());
            for v in values {
                types.push(get_type_constraints(v, ctx)?);
            }

            Scheme::empty(Type::Tuple(types))
        }
        AST::List(values) if values.is_empty() => {
            Scheme::empty(Type::List(Box::new(ctx.get_fresh())))
        }
        AST::List(values) => {
            let t1 = get_type_constraints(&values[0], ctx)?;

            for v in &values[1..] {
                let t2 = get_type_constraints(v, ctx)?;
                ctx.push_constraint((t1.clone(), t2));
            }

            Scheme::empty(Type::List(Box::new(t1)))
        }
        AST::Lambda(arg, body) => {
            let (t1, to_remove) = if let Some(arg) = arg {
                let t1 = ctx.get_fresh();
                ctx.insert(arg.to_string(), t1.clone());
                (t1, Some(arg.to_string()))
            } else {
                (Scheme::unit(), None)
            };

            let t2 = get_type_constraints(body, ctx)?;
            if let Some(arg) = to_remove {
                ctx.remove(&arg);
            }

            Scheme::empty(Type::Lambda(Box::new(t1), Box::new(t2)))
        }
        AST::Unit => Scheme::unit(),
        AST::Wildcard => Scheme::wildcard(),
        AST::Number(_) => Scheme::int(),
        AST::StringLiteral(_) => Scheme::string(),
        AST::Boolean(_) => Scheme::bool(),
        AST::Identifier(ident) => match ctx
            .get(ident)
            .cloned()
            .or_else(|| ctx.get_builtin_type(ident))
        {
            Some(t) => t,
            None => return Err(TypeError::UnknownVariable(ident.to_string())),
        },
        AST::SecurityLevel(_) => Scheme::empty(Type::Label),
    })
}

fn deep_substitute(current: &mut Scheme, from: &Scheme, to: &Scheme) {
    if current == from {
        *current = to.clone();
        return;
    }

    match &mut current.ty {
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

fn type_contains(t1: &Scheme, t2: &Scheme) -> bool {
    if t1 == t2 {
        true
    } else {
        match &t1.ty {
            Type::Lambda(from, to) => type_contains(&from, t2) || type_contains(&to, t2),
            Type::List(t) => type_contains(&t, t2),
            Type::Tuple(types) => types.iter().any(|t| type_contains(t, t2)),
            _ => false,
        }
    }
}

fn process_substitution(
    t: &Scheme,
    g: &Scheme,
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
        match (&t1.ty, &t2.ty) {
            (t1, t2) if t1 == t2 => continue,
            (Type::Lambda(a1, b1), Type::Lambda(a2, b2)) => {
                constraints.push((*a1.clone(), *a2.clone()));
                constraints.push((*b1.clone(), *b2.clone()));
            }
            (Type::Tuple(t1), Type::Tuple(t2)) if t1.len() == t2.len() => {
                for (a, b) in t1.into_iter().zip(t2) {
                    constraints.push((a.clone(), b.clone()));
                }
            }
            (Type::List(t1), Type::List(t2)) => {
                constraints.push((*t1.clone(), *t2.clone()));
            }
            (Type::Gen(_), _) => process_substitution(&t2, &t1, &mut constraints)?,
            (_, Type::Gen(_)) => process_substitution(&t1, &t2, &mut constraints)?,
            (_, _) => return Err(TypeError::TypeMismatchConstraint(t1, t2)),
        }
    }

    Ok(())
}

pub fn type_check(ast: AST) -> Result<(), TypeError> {
    let mut ctx = Ctx::new();
    get_type_constraints(&ast, &mut ctx)?;
    resolve_constraints(ctx.drop_constraints())
}
