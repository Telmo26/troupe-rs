use crate::parser::AST;

pub enum PrimitiveApp<'a> {
    Declassify {
        value: &'a AST,
        authority: &'a AST,
        target: &'a AST,
    },
    Send {
        target: String,
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
                },

                "declassifydeep" if let AST::Tuple(values) = argument.as_ref() => {
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
                        let target = match target {
                            AST::StringLiteral(s) => s.clone(),
                            AST::FunctionCall { callee, argument } if let AST::Identifier(s) = callee.as_ref() && s == "whereis" => {
                                if let AST::Tuple(args) = argument.as_ref() 
                                    && let Some(AST::StringLiteral(dest)) = &args.get(0) {
                                    dest.clone()
                                } else {
                                    return None
                                }
                            }
                            _ => return None
                        };
                        return Some(PrimitiveApp::Send { target, value })
                    };

                    return None
                }
                _ => None
            }
        }
        _ => None,
    }
}
