use std::collections::{HashMap, HashSet};

use crate::{
    parser::{Pattern, AST},
    type_checker::{deep_substitute, scheme::Scheme, types::Type, Constraint},
};

pub struct Ctx {
    fresh: u32,
    constraints: Vec<Constraint>,
    name_map: HashMap<String, Scheme>,
}

impl Ctx {
    pub fn new() -> Self {
        Ctx {
            fresh: 0,
            constraints: Vec::new(),
            name_map: HashMap::new(),
        }
    }

    pub fn push_constraint(&mut self, c: Constraint) {
        self.constraints.push(c)
    }

    pub fn get_builtin_type(&mut self, s: &str) -> Option<Scheme> {
        let t = match s {
            "authority" => Type::Authority,
            "exitAfterTimeout" => Type::Lambda(
                Box::new(Type::Authority),
                Box::new(Type::Lambda(
                    Box::new(Type::Int),
                    Box::new(Type::Lambda(
                        Box::new(Type::Int),
                        Box::new(Type::Lambda(Box::new(Type::String), Box::new(Type::Unit))),
                    )),
                )),
            ),
            "sleep" => Type::Lambda(Box::new(Type::Int), Box::new(Type::Unit)),
            "send" => Type::Lambda(Box::new(self.get_fresh()), Box::new(Type::Unit)),
            "receive" => Type::Lambda(
                Box::new(Type::List(Box::new(Type::Unit))),
                Box::new(self.get_fresh()),
            ),
            "self" | "mkuuid" => Type::Lambda(Box::new(Type::Unit), Box::new(Type::Int)),
            "spawn" => Type::Lambda(
                Box::new(Type::Lambda(
                    Box::new(Type::Unit),
                    Box::new(self.get_fresh()),
                )),
                Box::new(Type::Int),
            ),
            "print" => Type::Lambda(Box::new(self.get_fresh()), Box::new(Type::Unit)),
            "crash" => Type::Lambda(Box::new(Type::Unit), Box::new(Type::Unit)),
            _ => return None,
        };

        Some(self.generalize(t))
    }

    pub fn get_fresh(&mut self) -> Type {
        self.fresh += 1;
        Type::Gen(self.fresh - 1)
    }

    pub fn insert(&mut self, var: String, t: Scheme) {
        self.name_map.insert(var, t);
    }

    pub fn remove(&mut self, var: &str) -> Option<Scheme> {
        self.name_map.remove(var)
    }

    pub fn get(&self, var: &str) -> Option<&Scheme> {
        self.name_map.get(var)
    }

    pub fn insert_new_variable_with_set(&mut self, var: &Pattern, t: Type, free_vars: &[u32]) {
        match var {
            Pattern::Single(child) => match &child as &AST {
                AST::Identifier(name) => {
                    self.insert(name.to_owned(), Scheme::new(t, free_vars.to_vec()))
                }
                _ => (),
            },
            Pattern::Tuple(variables) => {
                let fresh_types: Vec<_> = variables.iter().map(|_| self.get_fresh()).collect();

                self.constraints.push((t, Type::Tuple(fresh_types.clone())));

                for (var, t) in variables.iter().zip(fresh_types) {
                    self.insert_new_variable_with_set(var, t, free_vars);
                }
            }
            _ => (),
        };
    }

    pub fn insert_new_variable(&mut self, var: &Pattern, t: Type) {
        let free_variables = self.get_free_variables(&t);
        self.insert_new_variable_with_set(var, t, &free_variables);
    }

    pub fn remove_variable(&mut self, var: &Pattern) {
        match var {
            Pattern::Single(child) => match &child as &AST {
                AST::Identifier(name) => {
                    self.remove(name);
                }
                _ => (),
            },
            Pattern::Tuple(variables) => {
                for var in variables {
                    self.remove_variable(var)
                }
            }
            _ => (),
        };
    }

    pub fn drop_constraints(self) -> Vec<Constraint> {
        self.constraints
    }

    pub fn instanciate(&mut self, Scheme { vars, mut ty }: Scheme) -> Type {
        for variable in vars {
            deep_substitute(&mut ty, &Type::Gen(variable), &self.get_fresh());
        }
        ty
    }

    pub fn get_context_free_variables(&self) -> HashSet<u32> {
        let mut result = HashSet::new();

        for scheme in self.name_map.values() {
            let mut fv = scheme.ty.get_free_variables();

            for v in &scheme.vars {
                fv.remove(v);
            }

            result.extend(fv);
        }

        result
    }

    pub fn get_free_variables(&self, t: &Type) -> Vec<u32> {
        let context_variables = self.get_context_free_variables();
        let free_variables = t.get_free_variables();
        free_variables
            .into_iter()
            .filter(|x| !context_variables.contains(x))
            .collect::<Vec<_>>()
    }

    pub fn generalize(&self, t: Type) -> Scheme {
        let set = self.get_free_variables(&t);
        Scheme::new(t, set)
    }
}
