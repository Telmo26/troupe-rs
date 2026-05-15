use crate::parser::AST;

pub enum PrimitiveApp<'a> {
    Declassify {
        value: &'a AST,
        authority: &'a AST,
        target: &'a AST,
    },

    Print {
        value: &'a AST,
    },

    Spawn {
        value: &'a AST,
    },

    Sleep {
        value: &'a AST,
    },

    Receive {
        value: &'a AST,
    },

    Send {
        value: &'a AST,
    },

    ExitAfterTimeout {
        value: &'a AST,
    },

    Self_,

    MkUuid,

    Authority,
}

pub fn match_primitive<'a>(ast: &'a AST) -> Option<PrimitiveApp<'a>> {
    match ast {
        AST::Identifier(name) => match name.as_str() {
            "self" => Some(PrimitiveApp::Self_),
            "mkuuid" => Some(PrimitiveApp::MkUuid),
            "authority" => Some(PrimitiveApp::Authority),
            _ => None,
        },

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

            if let AST::Identifier(name) = callee.as_ref() {
                match name.as_str() {
                    "print" => {
                        return Some(PrimitiveApp::Print { value: argument });
                    }

                    "spawn" => {
                        return Some(PrimitiveApp::Spawn { value: argument });
                    }

                    "sleep" => {
                        return Some(PrimitiveApp::Sleep { value: argument });
                    }

                    "receive" => {
                        return Some(PrimitiveApp::Receive { value: argument });
                    }

                    "send" => {
                        return Some(PrimitiveApp::Send { value: argument });
                    }

                    "exitAfterTimeout" => {
                        return Some(PrimitiveApp::ExitAfterTimeout { value: argument });
                    }

                    _ => {}
                }
            }

            None
        }

        _ => None,
    }
}
