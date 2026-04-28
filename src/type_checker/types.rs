use std::collections::HashSet;

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Type {
    String,
    Wildcard,
    Int,
    Unit,
    Bool,
    Label,
    Authority,
    Lambda(Box<Type>, Box<Type>),
    List(Box<Type>),
    Tuple(Vec<Type>),
    Gen(u32),
}

impl Type {
    pub fn get_free_variables(&self) -> HashSet<u32> {
        match self {
            Type::Lambda(t1, t2) => {
                let mut set = t1.get_free_variables();
                set.extend(t2.get_free_variables());
                set
            }
            Type::List(t) => t.get_free_variables(),
            Type::Tuple(types) => types.into_iter().fold(HashSet::new(), |mut acc, t| {
                acc.extend(t.get_free_variables());
                acc
            }),
            Type::Gen(x) => HashSet::from([*x]),
            _ => HashSet::new(),
        }
    }
}
