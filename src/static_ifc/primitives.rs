use crate::parser::AST;

pub enum PrimitiveApp<'a> {
    Declassify {
        value: &'a AST,
        authority: &'a AST,
        target: &'a AST,
    },
}

pub fn match_primitive<'a>(ast: &'a AST) -> Option<PrimitiveApp<'a>> {
    match ast {
        AST::FunctionCall { callee, argument } => {
            if let AST::Identifier(name) = callee.as_ref() {
                if name == "declassify" {
                    if let AST::Tuple(values) = argument.as_ref() {
                        if let [value, authority, target] = values.as_slice() {
                            return Some(PrimitiveApp::Declassify {
                                value,
                                authority,
                                target,
                            });
                        }
                    }
                }
            }

            None
        }

        _ => None,
    }
}
