use crate::type_checker::scheme::Scheme;

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Type {
    String,
    Wildcard,
    Int,
    Unit,
    Bool,
    Label,
    Authority,
    Lambda(Box<Scheme>, Box<Scheme>),
    List(Box<Scheme>),
    Tuple(Vec<Scheme>),
    Gen(u32),
}
