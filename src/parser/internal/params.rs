use crate::lexer::token::TokenKind;
use crate::parser::ast::Arg;
use crate::parser::ast::Expression;
use crate::parser::ast::MethodFlag;
use crate::parser::ast::Param;
use crate::parser::ast::ParamList;
use crate::parser::ast::PropertyFlag;
use crate::parser::error::ParseError;
use crate::parser::error::ParseResult;
use crate::parser::internal::precedence::Precedence;
use crate::parser::state::Scope;
use crate::parser::state::State;
use crate::parser::Parser;

use crate::expect_token;

impl Parser {
    pub(in crate::parser) fn param_list(&self, state: &mut State) -> Result<ParamList, ParseError> {
        let mut params = ParamList::new();

        let mut class_name = String::new();
        let construct: i8 = match state.scope()? {
            Scope::Function(_) | Scope::AnonymousFunction(_) | Scope::ArrowFunction(_) => 0,
            Scope::Method(name, flags) => {
                if name.to_string() != "__construct" {
                    0
                } else {
                    match state.parent()? {
                        // can only have abstract ctor
                        Scope::Interface(_) => 1,
                        // can only have concret ctor
                        Scope::AnonymousClass => {
                            class_name = state.named(&"class@anonymous".into());

                            2
                        }
                        // can have either abstract or concret ctor,
                        // depens on method flag.
                        Scope::Class(name, _) | Scope::Trait(name) => {
                            if flags.contains(&MethodFlag::Abstract) {
                                1
                            } else {
                                class_name = state.named(name);

                                2
                            }
                        }
                        _ => unreachable!(),
                    }
                }
            }
            _ => unreachable!(),
        };

        while !state.is_eof() && state.current.kind != TokenKind::RightParen {
            let mut param_type = None;

            let flags: Vec<PropertyFlag> = self
                .promoted_property_flags(state)?
                .iter()
                .map(|f| f.into())
                .collect();

            if !flags.is_empty() {
                match construct {
                    0 => {
                        return Err(ParseError::PromotedPropertyOutsideConstructor(
                            state.current.span,
                        ));
                    }
                    1 => {
                        return Err(ParseError::PromotedPropertyOnAbstractConstructor(
                            state.current.span,
                        ));
                    }
                    _ => {}
                }
            }

            if !matches!(
                state.current.kind,
                TokenKind::Variable(_) | TokenKind::Ellipsis | TokenKind::Ampersand
            ) {
                // Try to parse the type.
                param_type = Some(self.get_type(state)?);
            }

            let mut variadic = false;
            let mut by_ref = false;

            if matches!(state.current.kind, TokenKind::Ampersand) {
                state.next();
                by_ref = true;
            }

            if matches!(state.current.kind, TokenKind::Ellipsis) {
                state.next();
                if !flags.is_empty() {
                    return Err(ParseError::VariadicPromotedProperty(state.current.span));
                }

                variadic = true;
            }

            // 2. Then expect a variable.
            let var = expect_token!([
                TokenKind::Variable(v) => v
            ], state, "a varaible");

            if flags.contains(&PropertyFlag::Readonly) && param_type.is_none() {
                return Err(ParseError::MissingTypeForReadonlyProperty(
                    class_name,
                    var.to_string(),
                    state.current.span,
                ));
            }

            let mut default = None;
            if state.current.kind == TokenKind::Equals {
                state.next();
                default = Some(self.expression(state, Precedence::Lowest)?);
            }

            params.push(Param {
                name: Expression::Variable { name: var },
                r#type: param_type,
                variadic,
                default,
                flags,
                by_ref,
            });

            self.optional_comma(state)?;
        }

        Ok(params)
    }

    pub(in crate::parser) fn args_list(&self, state: &mut State) -> ParseResult<Vec<Arg>> {
        let mut args = Vec::new();

        while !state.is_eof() && state.current.kind != TokenKind::RightParen {
            let mut name = None;
            let mut unpack = false;
            if matches!(state.current.kind, TokenKind::Identifier(_))
                && state.peek.kind == TokenKind::Colon
            {
                name = Some(self.ident_maybe_reserved(state)?);
                state.next();
            } else if state.current.kind == TokenKind::Ellipsis {
                state.next();
                unpack = true;
            }

            if unpack && state.current.kind == TokenKind::RightParen {
                args.push(Arg {
                    name: None,
                    unpack: false,
                    value: Expression::VariadicPlaceholder,
                });

                break;
            }

            let value = self.expression(state, Precedence::Lowest)?;

            args.push(Arg {
                name,
                unpack,
                value,
            });

            self.optional_comma(state)?;
        }

        Ok(args)
    }
}