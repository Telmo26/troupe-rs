mod lexer;
mod parser;
mod type_checker;
mod static_ifc;
mod interpreter;
mod trustmap;

pub use crate::{
    lexer::Token, 
    parser::{AST, parse}, 
    type_checker::type_check,
    static_ifc::static_ifc_check,
    interpreter::Interpreter, 
    trustmap::TrustMap, 
};