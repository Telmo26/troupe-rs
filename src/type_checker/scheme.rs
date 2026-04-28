use crate::type_checker::types::Type;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Scheme {
    pub vars: Vec<u32>,
    pub ty: Type,
}

impl Scheme {
    pub fn empty(ty: Type) -> Self {
        Self {
            vars: Vec::new(),
            ty,
        }
    }

    pub fn get_gen(x: u32) -> Self {
        Scheme {
            vars: vec![x],
            ty: Type::Gen(x),
        }
    }

    pub fn string() -> Self {
        Self::empty(Type::String)
    }
    pub fn wildcard() -> Self {
        Self::empty(Type::Wildcard)
    }
    pub fn int() -> Self {
        Self::empty(Type::Int)
    }
    pub fn unit() -> Self {
        Self::empty(Type::Unit)
    }
    pub fn bool() -> Self {
        Self::empty(Type::Bool)
    }
    pub fn label() -> Self {
        Self::empty(Type::Label)
    }
    pub fn authority() -> Self {
        Self::empty(Type::Authority)
    }
}
