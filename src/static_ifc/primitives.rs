use crate::parser::AST;

pub enum PrimitiveApp<'a> {
    Declassify {
        value: &'a AST,
        authority: &'a AST,
        target: &'a AST,
    },
    Send {
        target: &'a AST,
        value: &'a AST
    }
}

pub fn match_primitive<'a>(ast: &'a AST) -> Option<PrimitiveApp<'a>> {
    match ast {
        AST::FunctionCall { callee, argument } if let AST::Identifier(name) = callee.as_ref() => {
            match name.as_str() {
                "declassify" if let AST::Tuple(values) = argument.as_ref() => {
                    if let [value, authority, target] = values.as_slice() {
                        Some(PrimitiveApp::Declassify {
                            value,
                            authority,
                            target,
                        })
                    } else {
                        None
                    }
                }

                "send" if let AST::Tuple(values) = argument.as_ref() => {
                    if let [target, value] = values.as_slice() {
                        Some(PrimitiveApp::Send { target, value })
                    } else {
                        None
                    }
                }

                _ => None
            }
        }

        _ => None,
    }
}
