use std::collections::HashMap;

use crate::{
    parser::{Pattern, AST},
    type_checker::{scheme::Scheme, types::Type, Constraint},
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
                Box::new(Scheme::authority()),
                Box::new(Scheme::empty(Type::Lambda(
                    Box::new(Scheme::int()),
                    Box::new(Scheme::empty(Type::Lambda(
                        Box::new(Scheme::int()),
                        Box::new(Scheme::empty(Type::Lambda(
                            Box::new(Scheme::string()),
                            Box::new(Scheme::unit()),
                        ))),
                    ))),
                ))),
            ),
            "sleep" => Type::Lambda(Box::new(Scheme::int()), Box::new(Scheme::unit())),
            "send" => Type::Lambda(Box::new(self.get_fresh()), Box::new(Scheme::unit())),
            "receive" => Type::Lambda(
                Box::new(Scheme::empty(Type::List(Box::new(Scheme::empty(
                    Type::Unit,
                ))))),
                Box::new(self.get_fresh()),
            ),
            "self" | "mkuuid" => Type::Lambda(Box::new(Scheme::unit()), Box::new(Scheme::int())),
            "spawn" => Type::Lambda(
                Box::new(Scheme::empty(Type::Lambda(
                    Box::new(Scheme::unit()),
                    Box::new(self.get_fresh()),
                ))),
                Box::new(Scheme::int()),
            ),
            "print" => Type::Lambda(Box::new(Scheme::string()), Box::new(Scheme::unit())),
            _ => return None,
        };

        Some(Scheme::empty(t))
    }

    pub fn get_fresh(&mut self) -> Scheme {
        self.fresh += 1;
        Scheme::get_gen(self.fresh - 1)
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

    pub fn insert_new_variable(&mut self, var: &Pattern, t: Scheme) {
        match var {
            Pattern::Single(child) => match &child as &AST {
                AST::Identifier(name) => self.insert(name.to_owned(), t),
                _ => (),
            },
            Pattern::Tuple(variables) => {
                let fresh_types: Vec<_> = variables.iter().map(|_| self.get_fresh()).collect();

                self.constraints
                    .push((t, Scheme::empty(Type::Tuple(fresh_types.clone()))));

                for (var, t) in variables.iter().zip(fresh_types) {
                    self.insert_new_variable(var, t);
                }
            }
            _ => (),
        };
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
}
