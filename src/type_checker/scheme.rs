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

    pub fn new(ty: Type, vars: Vec<u32>) -> Self {
        Self { vars, ty }
    }
}
