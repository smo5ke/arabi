use arabi_core::token::{Token, SpannedToken, Keyword, Operator, Delimiter};
use arabi_core::error::{ArabiError, Result};
use crate::ast::*;

pub struct Parser {
    tokens: Vec<SpannedToken>,
    position: usize,
}

impl Parser {
    pub fn new(tokens: Vec<SpannedToken>) -> Self {
        Parser {
            tokens,
            position: 0,
        }
    }

    pub fn parse(&mut self) -> Result<Program> {
        let mut stmts = Vec::new();
        let mut stmt_lines = Vec::new();

        while !self.is_at_end() {
            if self.check(&Token::Newline) {
                self.advance();
                continue;
            }
            if self.check(&Token::Delimiter(Delimiter::Semicolon)) {
                self.advance();
                continue;
            }
            let line = self.current_line();
            stmt_lines.push(line);
            stmts.push(self.parse_statement()?);
            // Consume optional semicolons and newlines after statement
            while self.check(&Token::Delimiter(Delimiter::Semicolon)) {
                self.advance();
            }
        }

        Ok(Program { stmts, stmt_lines })
    }

    fn parse_statement(&mut self) -> Result<Stmt> {
        match self.current() {
            Token::Operator(Operator::At) | Token::Keyword(Keyword::Decorator) => self.parse_decorator(),
            Token::Keyword(Keyword::If) => self.parse_if(),
            Token::Keyword(Keyword::While) => self.parse_while(),
            Token::Keyword(Keyword::For) => self.parse_for(),
            Token::Keyword(Keyword::Function) => self.parse_function_def(),
            Token::Keyword(Keyword::Class) => self.parse_class_def(),
            Token::Keyword(Keyword::Import) => self.parse_import(),
            Token::Keyword(Keyword::From) => self.parse_import_from(),
            Token::Keyword(Keyword::Try) => self.parse_try(),
            Token::Keyword(Keyword::Raise) => {
                self.advance();
                let value = if self.check(&Token::Newline) || self.is_at_end() {
                    None
                } else {
                    Some(self.parse_expression()?)
                };
                Ok(Stmt::Raise(value))
            }
            Token::Keyword(Keyword::Return) => {
                self.advance();
                let value = if self.check(&Token::Newline) || self.is_at_end() {
                    None
                } else {
                    Some(self.parse_expression()?)
                };
                Ok(Stmt::Return(value))
            }
            Token::Keyword(Keyword::Break) => {
                self.advance();
                Ok(Stmt::Break)
            }
            Token::Keyword(Keyword::Continue) => {
                self.advance();
                Ok(Stmt::Continue)
            }
            Token::Keyword(Keyword::Pass) => {
                self.advance();
                Ok(Stmt::Pass)
            }
            Token::Keyword(Keyword::Delete) => {
                self.advance();
                let expr = self.parse_expression()?;
                Ok(Stmt::Delete(expr))
            }
            Token::Keyword(Keyword::Assert) => {
                self.advance();
                let condition = self.parse_expression()?;
                let message = if self.check(&Token::Delimiter(Delimiter::Comma)) {
                    self.advance();
                    Some(self.parse_expression()?)
                } else {
                    None
                };
                Ok(Stmt::Assert { condition, message })
            }
            Token::Keyword(Keyword::Global) => {
                self.advance();
                let mut names = Vec::new();
                names.push(self.parse_identifier()?);
                while self.check(&Token::Delimiter(Delimiter::Comma)) {
                    self.advance();
                    names.push(self.parse_identifier()?);
                }
                Ok(Stmt::Global(names))
            }
            Token::Keyword(Keyword::Nonlocal) => {
                self.advance();
                let mut names = Vec::new();
                names.push(self.parse_identifier()?);
                while self.check(&Token::Delimiter(Delimiter::Comma)) {
                    self.advance();
                    names.push(self.parse_identifier()?);
                }
                Ok(Stmt::Nonlocal(names))
            }
            Token::Keyword(Keyword::Yield) => {
                self.advance();
                let value = if self.check(&Token::Newline) || self.is_at_end() {
                    None
                } else {
                    Some(self.parse_expression()?)
                };
                Ok(Stmt::Yield(value))
            }
            Token::Keyword(Keyword::YieldFrom) => {
                self.advance();
                let value = self.parse_expression()?;
                Ok(Stmt::YieldFrom(value))
            }
            Token::Keyword(Keyword::With) => self.parse_with(),
            _ => self.parse_expression_statement(),
        }
    }

    fn parse_if(&mut self) -> Result<Stmt> {
        self.consume(Keyword::If)?;
        let condition = self.parse_expression()?;
        self.consume_delim(Delimiter::Colon)?;
        let body = self.parse_block()?;
        let mut elifs = Vec::new();
        let mut else_body = None;

        // Skip newlines after block
        while self.check(&Token::Newline) {
            self.advance();
        }

        while self.check(&Token::Keyword(Keyword::Elif)) {
            self.advance();
            let elif_cond = self.parse_expression()?;
            self.consume_delim(Delimiter::Colon)?;
            let elif_body = self.parse_block()?;
            elifs.push((elif_cond, elif_body));

            // Skip newlines after elif block
            while self.check(&Token::Newline) {
                self.advance();
            }
        }

        // Skip newlines before else
        while self.check(&Token::Newline) {
            self.advance();
        }

        if self.check(&Token::Keyword(Keyword::Else)) {
            self.advance();
            self.consume_delim(Delimiter::Colon)?;
            else_body = Some(self.parse_block()?);
        }

        Ok(Stmt::If {
            condition,
            body,
            elifs,
            else_body,
        })
    }

    fn parse_while(&mut self) -> Result<Stmt> {
        self.consume(Keyword::While)?;
        let condition = self.parse_expression()?;
        self.consume_delim(Delimiter::Colon)?;
        let body = self.parse_block()?;
        while self.check(&Token::Newline) {
            self.advance();
        }
        let else_body = if self.check(&Token::Keyword(Keyword::Else)) {
            self.advance();
            self.consume_delim(Delimiter::Colon)?;
            Some(self.parse_block()?)
        } else {
            None
        };
        Ok(Stmt::While { condition, body, else_body })
    }

    fn parse_for(&mut self) -> Result<Stmt> {
        self.consume(Keyword::For)?;
        let target = if self.check(&Token::Delimiter(Delimiter::LParen)) {
            self.advance();
            let mut targets = Vec::new();
            if let Token::Identifier(name) = self.current().clone() {
                self.advance();
                targets.push(Expr::Identifier(name));
            }
            while self.check(&Token::Delimiter(Delimiter::Comma)) {
                self.advance();
                if let Token::Identifier(name) = self.current().clone() {
                    self.advance();
                    targets.push(Expr::Identifier(name));
                }
            }
            self.consume_delim(Delimiter::RParen)?;
            Expr::Tuple(targets)
        } else {
            Expr::Identifier(self.parse_identifier()?)
        };
        self.consume(Keyword::In)?;
        let iterable = self.parse_expression()?;
        self.consume_delim(Delimiter::Colon)?;
        let body = self.parse_block()?;
        while self.check(&Token::Newline) {
            self.advance();
        }
        let else_body = if self.check(&Token::Keyword(Keyword::Else)) {
            self.advance();
            self.consume_delim(Delimiter::Colon)?;
            Some(self.parse_block()?)
        } else {
            None
        };
        Ok(Stmt::For {
            target,
            iterable,
            body,
            else_body,
        })
    }

    fn parse_function_def(&mut self) -> Result<Stmt> {
        self.consume(Keyword::Function)?;
        let name = self.parse_identifier()?;
        self.consume_delim(Delimiter::LParen)?;
        let params = self.parse_params()?;
        self.consume_delim(Delimiter::RParen)?;
        self.consume_delim(Delimiter::Colon)?;
        let body = self.parse_block()?;
        Ok(Stmt::FunctionDef { name, params, body })
    }

    fn parse_class_def(&mut self) -> Result<Stmt> {
        self.consume(Keyword::Class)?;
        let name = self.parse_identifier()?;
        let mut bases = Vec::new();

        if self.check(&Token::Delimiter(Delimiter::LParen)) {
            self.advance();
            while !self.check(&Token::Delimiter(Delimiter::RParen)) {
                bases.push(self.parse_expression()?);
                if self.check(&Token::Delimiter(Delimiter::Comma)) {
                    self.advance();
                }
            }
            self.consume_delim(Delimiter::RParen)?;
        }

        self.consume_delim(Delimiter::Colon)?;
        let body = self.parse_block()?;
        Ok(Stmt::ClassDef { name, bases, body })
    }

    fn parse_dotted_identifier(&mut self) -> Result<String> {
        let mut name = self.parse_identifier()?;
        while self.check(&Token::Delimiter(Delimiter::Dot)) {
            self.advance();
            let part = self.parse_identifier()?;
            name.push('.');
            name.push_str(&part);
        }
        Ok(name)
    }

    fn parse_import(&mut self) -> Result<Stmt> {
        self.consume(Keyword::Import)?;
        let module = self.parse_dotted_identifier()?;
        let alias = if self.check(&Token::Keyword(Keyword::As)) {
            self.advance();
            Some(self.parse_identifier()?)
        } else {
            None
        };
        Ok(Stmt::Import { module, alias })
    }

    fn parse_import_from(&mut self) -> Result<Stmt> {
        self.consume(Keyword::From)?;
        let module = self.parse_dotted_identifier()?;
        self.consume(Keyword::Import)?;
        let mut names = Vec::new();

        loop {
            let name = self.parse_identifier()?;
            let alias = if self.check(&Token::Keyword(Keyword::As)) {
                self.advance();
                Some(self.parse_identifier()?)
            } else {
                None
            };
            names.push((name, alias));

            if self.check(&Token::Delimiter(Delimiter::Comma)) {
                self.advance();
            } else {
                break;
            }
        }

        Ok(Stmt::ImportFrom { module, names })
    }

    fn parse_try(&mut self) -> Result<Stmt> {
        self.consume(Keyword::Try)?;
        self.consume_delim(Delimiter::Colon)?;
        let body = self.parse_block()?;
        let mut excepts = Vec::new();

        while self.check(&Token::Keyword(Keyword::Except)) {
            self.advance();
            let type_name = if self.check(&Token::Newline) || self.is_at_end() || self.check(&Token::Delimiter(Delimiter::Colon)) || self.check(&Token::Keyword(Keyword::As)) {
                None
            } else {
                Some(self.parse_identifier()?)
            };
            let name = if self.check(&Token::Keyword(Keyword::As)) {
                self.advance();
                Some(self.parse_identifier()?)
            } else {
                None
            };
            self.consume_delim(Delimiter::Colon)?;
            let except_body = self.parse_block()?;
            excepts.push(ExceptClause {
                type_name,
                name,
                body: except_body,
            });
        }

        let else_body = if self.check(&Token::Keyword(Keyword::Else)) {
            self.advance();
            self.consume_delim(Delimiter::Colon)?;
            Some(self.parse_block()?)
        } else {
            None
        };

        // Skip newlines after else block
        while self.check(&Token::Newline) {
            self.advance();
        }

        let finally_body = if self.check(&Token::Keyword(Keyword::Finally)) {
            self.advance();
            self.consume_delim(Delimiter::Colon)?;
            Some(self.parse_block()?)
        } else {
            None
        };

        Ok(Stmt::Try {
            body,
            excepts,
            else_body,
            finally_body,
        })
    }

    fn parse_decorator(&mut self) -> Result<Stmt> {
        let mut decorators = Vec::new();
        while self.check(&Token::Operator(Operator::At)) || self.check(&Token::Keyword(Keyword::Decorator)) {
            self.advance();
            decorators.push(self.parse_expression()?);
            if self.check(&Token::Newline) {
                self.advance();
            }
        }
        let definition = self.parse_statement()?;
        Ok(Stmt::Decorator {
            decorators,
            definition: Box::new(definition),
        })
    }

    fn parse_expression_statement(&mut self) -> Result<Stmt> {
        // Check for leading star (star-unpacking: *ا، ب = ...)
        if self.check(&Token::Operator(Operator::Star)) {
            let mut targets = Vec::new();
            self.advance();
            if let Token::Identifier(name) = self.current().clone() {
                self.advance();
                targets.push((name, true));
            } else {
                return Err(ArabiError::ParseError {
                    message: "المتوقع معرف بعد *".to_string(),
                    span: self.current_span(),
                });
            }
            while self.check(&Token::Delimiter(Delimiter::Comma)) {
                self.advance();
                if self.check(&Token::Operator(Operator::Star)) {
                    self.advance();
                    if let Token::Identifier(name) = self.current().clone() {
                        self.advance();
                        targets.push((name, true));
                    } else {
                        return Err(ArabiError::ParseError {
                            message: "المتوقع معرف بعد *".to_string(),
                            span: self.current_span(),
                        });
                    }
                } else {
                    match self.parse_expression()? {
                        Expr::Identifier(name) => targets.push((name, false)),
                        other => return Err(ArabiError::ParseError {
                            message: format!("المتوقع معرف، وُجد {:?}", other),
                            span: self.current_span(),
                        }),
                    }
                }
            }
            if !self.check(&Token::Operator(Operator::Assign)) {
                return Err(ArabiError::ParseError {
                    message: "المتوقع = بعد الاهداف المتعددة".to_string(),
                    span: self.current_span(),
                });
            }
            self.advance();
            let value = self.parse_expression()?;
            return Ok(Stmt::MultiAssign { targets, value });
        }

        let expr = self.parse_expression()?;

        if self.check(&Token::Operator(Operator::Assign)) {
            self.advance();
            let value = self.parse_expression()?;
            Ok(Stmt::Assign { target: expr, value })
        } else if let Some(op) = self.check_aug_assign() {
            self.advance();
            let value = self.parse_expression()?;
            Ok(Stmt::AugAssign {
                target: expr,
                op,
                value,
            })
        } else if self.check(&Token::Delimiter(Delimiter::Comma)) {
            let mut targets = Vec::new();
            if self.check(&Token::Operator(Operator::Star)) {
                self.advance();
                if let Token::Identifier(name) = self.current().clone() {
                    self.advance();
                    targets.push((name, true));
                } else {
                    return Err(ArabiError::ParseError {
                        message: "المتوقع معرف بعد *".to_string(),
                        span: self.current_span(),
                    });
                }
            } else if let Expr::Identifier(name) = expr {
                targets.push((name, false));
            }
            while self.check(&Token::Delimiter(Delimiter::Comma)) {
                self.advance();
                if self.check(&Token::Operator(Operator::Star)) {
                    self.advance();
                    if let Token::Identifier(name) = self.current().clone() {
                        self.advance();
                        targets.push((name, true));
                    } else {
                        return Err(ArabiError::ParseError {
                            message: "المتوقع معرف بعد *".to_string(),
                            span: self.current_span(),
                        });
                    }
                } else {
                    match self.parse_expression()? {
                        Expr::Identifier(name) => targets.push((name, false)),
                        other => return Err(ArabiError::ParseError {
                            message: format!("المتوقع معرف، وُجد {:?}", other),
                            span: self.current_span(),
                        }),
                    }
                }
            }
            if !self.check(&Token::Operator(Operator::Assign)) {
                return Err(ArabiError::ParseError {
                    message: "المتوقع = بعد الاهداف المتعددة".to_string(),
                    span: self.current_span(),
                });
            }
            self.advance();
            let mut values = vec![self.parse_expression()?];
            while self.check(&Token::Delimiter(Delimiter::Comma)) {
                self.advance();
                values.push(self.parse_expression()?);
            }
            let value = if values.len() == 1 {
                values.remove(0)
            } else {
                Expr::List(values)
            };
            Ok(Stmt::MultiAssign { targets, value })
        } else {
            Ok(Stmt::Expr(expr))
        }
    }

    fn parse_expression(&mut self) -> Result<Expr> {
        self.parse_conditional()
    }

    fn parse_conditional(&mut self) -> Result<Expr> {
        let true_expr = self.parse_or()?;

        if self.check(&Token::Keyword(Keyword::If)) {
            self.advance();
            let condition = self.parse_or()?;
            if self.check(&Token::Keyword(Keyword::Else)) {
                self.advance();
            }
            let false_expr = self.parse_conditional()?;
            Ok(Expr::IfExpr {
                condition: Box::new(condition),
                true_expr: Box::new(true_expr),
                false_expr: Box::new(false_expr),
            })
        } else {
            Ok(true_expr)
        }
    }

    fn parse_or(&mut self) -> Result<Expr> {
        let mut left = self.parse_and()?;

        while self.check(&Token::Keyword(Keyword::Or)) {
            self.advance();
            let right = self.parse_and()?;
            left = Expr::BinaryOp {
                left: Box::new(left),
                op: BinOp::Or,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr> {
        let mut left = self.parse_not()?;

        while self.check(&Token::Keyword(Keyword::And)) {
            self.advance();
            let right = self.parse_not()?;
            left = Expr::BinaryOp {
                left: Box::new(left),
                op: BinOp::And,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_not(&mut self) -> Result<Expr> {
        if self.check(&Token::Keyword(Keyword::Not)) {
            self.advance();
            let operand = self.parse_not()?;
            Ok(Expr::UnaryOp {
                op: UnaryOp::Not,
                operand: Box::new(operand),
            })
        } else {
            self.parse_comparison()
        }
    }

    fn parse_comparison(&mut self) -> Result<Expr> {
        let mut left = self.parse_bitwise_or()?;
        let mut result: Option<Expr> = None;

        loop {
            let op = match self.current() {
                Token::Operator(Operator::Eq) => Some(BinOp::Eq),
                Token::Operator(Operator::NotEq) => Some(BinOp::NotEq),
                Token::Operator(Operator::Lt) => Some(BinOp::Lt),
                Token::Operator(Operator::Gt) => Some(BinOp::Gt),
                Token::Operator(Operator::LtEq) => Some(BinOp::LtEq),
                Token::Operator(Operator::GtEq) => Some(BinOp::GtEq),
                Token::Keyword(Keyword::In) => Some(BinOp::In),
                _ => None,
            };

            if let Some(cmp_op) = op {
                self.advance();
                let right = self.parse_bitwise_or()?;
                let comparison = Expr::BinaryOp {
                    left: Box::new(left.clone()),
                    op: cmp_op,
                    right: Box::new(right.clone()),
                };
                if let Some(prev) = result {
                    result = Some(Expr::BinaryOp {
                        left: Box::new(prev),
                        op: BinOp::And,
                        right: Box::new(comparison),
                    });
                } else {
                    result = Some(comparison);
                }
                left = right;
            } else if self.check(&Token::Keyword(Keyword::Not)) {
                self.advance();
                if self.check(&Token::Keyword(Keyword::In)) {
                    self.advance();
                    let right = self.parse_bitwise_or()?;
                    let comparison = Expr::BinaryOp {
                        left: Box::new(left.clone()),
                        op: BinOp::NotIn,
                        right: Box::new(right.clone()),
                    };
                    if let Some(prev) = result {
                        result = Some(Expr::BinaryOp {
                            left: Box::new(prev),
                            op: BinOp::And,
                            right: Box::new(comparison),
                        });
                    } else {
                        result = Some(comparison);
                    }
                    left = right;
                } else {
                    break;
                }
            } else if self.check(&Token::Keyword(Keyword::Is)) {
                self.advance();
                let op = if self.check(&Token::Keyword(Keyword::Not)) {
                    self.advance();
                    BinOp::IsNot
                } else {
                    BinOp::Is
                };
                let right = self.parse_bitwise_or()?;
                let comparison = Expr::BinaryOp {
                    left: Box::new(left.clone()),
                    op,
                    right: Box::new(right.clone()),
                };
                if let Some(prev) = result {
                    result = Some(Expr::BinaryOp {
                        left: Box::new(prev),
                        op: BinOp::And,
                        right: Box::new(comparison),
                    });
                } else {
                    result = Some(comparison);
                }
                left = right;
            } else {
                break;
            }
        }

        Ok(result.unwrap_or(left))
    }

    fn parse_bitwise_or(&mut self) -> Result<Expr> {
        let mut left = self.parse_bitwise_and()?;
        while self.check(&Token::Operator(Operator::Pipe)) {
            self.advance();
            let right = self.parse_bitwise_and()?;
            left = Expr::BinaryOp {
                left: Box::new(left),
                op: BinOp::BitOr,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_bitwise_and(&mut self) -> Result<Expr> {
        let mut left = self.parse_shifts()?;
        while self.check(&Token::Operator(Operator::Ampersand)) {
            self.advance();
            let right = self.parse_shifts()?;
            left = Expr::BinaryOp {
                left: Box::new(left),
                op: BinOp::BitAnd,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_addition(&mut self) -> Result<Expr> {
        let mut left = self.parse_multiplication()?;

        loop {
            let op = match self.current() {
                Token::Operator(Operator::Plus) => Some(BinOp::Add),
                Token::Operator(Operator::Minus) => Some(BinOp::Sub),
                _ => None,
            };

            if let Some(op) = op {
                self.advance();
                let right = self.parse_multiplication()?;
                left = Expr::BinaryOp {
                    left: Box::new(left),
                    op,
                    right: Box::new(right),
                };
            } else {
                break;
            }
        }

        Ok(left)
    }

    fn parse_shifts(&mut self) -> Result<Expr> {
        let mut left = self.parse_addition()?;

        loop {
            let op = match self.current() {
                Token::Operator(Operator::Shl) => Some(BinOp::Shl),
                Token::Operator(Operator::Shr) => Some(BinOp::Shr),
                _ => None,
            };

            if let Some(op) = op {
                self.advance();
                let right = self.parse_addition()?;
                left = Expr::BinaryOp {
                    left: Box::new(left),
                    op,
                    right: Box::new(right),
                };
            } else {
                break;
            }
        }

        Ok(left)
    }

    fn parse_multiplication(&mut self) -> Result<Expr> {
        let mut left = self.parse_power()?;

        loop {
            let op = match self.current() {
                Token::Operator(Operator::Star) => Some(BinOp::Mul),
                Token::Operator(Operator::Slash) => Some(BinOp::Div),
                Token::Operator(Operator::Backslash) => Some(BinOp::FloorDiv),
                Token::Operator(Operator::Percent) => Some(BinOp::Mod),
                _ => None,
            };

            if let Some(op) = op {
                self.advance();
                let right = self.parse_power()?;
                left = Expr::BinaryOp {
                    left: Box::new(left),
                    op,
                    right: Box::new(right),
                };
            } else {
                break;
            }
        }

        Ok(left)
    }

    fn parse_power(&mut self) -> Result<Expr> {
        let mut left = self.parse_unary()?;

        if self.check(&Token::Operator(Operator::Caret)) || self.check(&Token::Operator(Operator::DoubleStar)) {
            self.advance();
            let right = self.parse_power()?;
            left = Expr::BinaryOp {
                left: Box::new(left),
                op: BinOp::Pow,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr> {
        if self.check(&Token::Operator(Operator::Minus)) {
            self.advance();
            let operand = self.parse_unary()?;
            Ok(Expr::UnaryOp {
                op: UnaryOp::Neg,
                operand: Box::new(operand),
            })
        } else if self.check(&Token::Operator(Operator::Tilde)) {
            self.advance();
            let operand = self.parse_unary()?;
            Ok(Expr::UnaryOp {
                op: UnaryOp::BitNot,
                operand: Box::new(operand),
            })
        } else {
            self.parse_postfix()
        }
    }

    fn parse_postfix(&mut self) -> Result<Expr> {
        let mut expr = self.parse_primary()?;

        loop {
            if self.check(&Token::Delimiter(Delimiter::LParen)) {
                self.advance();
                let mut args = Vec::new();
                let mut kwargs = Vec::new();
                let mut unpack_args = Vec::new();
                let mut unpack_kwargs = Vec::new();

                while !self.check(&Token::Delimiter(Delimiter::RParen)) {
                    if self.check(&Token::Operator(Operator::DoubleStar)) {
                        self.advance();
                        unpack_kwargs.push(self.parse_expression()?);
                    } else if self.check(&Token::Operator(Operator::Star)) {
                        self.advance();
                        unpack_args.push(self.parse_expression()?);
                    } else if let Token::Identifier(name) = self.current().clone() {
                        if self.peek_token() == Some(&Token::Operator(Operator::Assign)) {
                            self.advance();
                            self.advance();
                            let value = self.parse_expression()?;
                            kwargs.push((name, value));
                        } else {
                            args.push(self.parse_expression()?);
                        }
                    } else {
                        args.push(self.parse_expression()?);
                    }

                    if self.check(&Token::Delimiter(Delimiter::Comma)) {
                        self.advance();
                    }
                }

                self.consume_delim(Delimiter::RParen)?;
                expr = Expr::Call {
                    function: Box::new(expr),
                    args,
                    kwargs,
                    unpack_args,
                    unpack_kwargs,
                };
            } else if self.check(&Token::Delimiter(Delimiter::Dot)) {
                self.advance();
                let name = self.parse_identifier()?;
                expr = Expr::Attribute {
                    object: Box::new(expr),
                    name,
                };
            } else if self.check(&Token::Delimiter(Delimiter::LBrack)) {
                self.advance();
                // Check for slice: [:start] or [start:end] or [start:end:step]
                let start = if self.check(&Token::Delimiter(Delimiter::Colon)) {
                    None
                } else {
                    Some(Box::new(self.parse_expression()?))
                };
                if self.check(&Token::Delimiter(Delimiter::Colon)) {
                    self.advance();
                    let end = if self.check(&Token::Delimiter(Delimiter::RBrack)) || self.check(&Token::Delimiter(Delimiter::Colon)) {
                        None
                    } else {
                        Some(Box::new(self.parse_expression()?))
                    };
                    let step = if self.check(&Token::Delimiter(Delimiter::Colon)) {
                        self.advance();
                        if self.check(&Token::Delimiter(Delimiter::RBrack)) {
                            None
                        } else {
                            Some(Box::new(self.parse_expression()?))
                        }
                    } else {
                        None
                    };
                    self.consume_delim(Delimiter::RBrack)?;
                    expr = Expr::Slice {
                        object: Box::new(expr),
                        start,
                        end,
                        step,
                    };
                } else {
                    let index = if let Some(s) = start {
                        s
                    } else {
                        return Err(ArabiError::ParseError {
                            message: "فهرس غير صالح".to_string(),
                            span: arabi_core::span::Span::single(arabi_core::span::Position::start()),
                        });
                    };
                    self.consume_delim(Delimiter::RBrack)?;
                    expr = Expr::Index {
                        object: Box::new(expr),
                        index,
                    };
                }
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr> {
        match self.current().clone() {
            Token::Integer(n) => {
                self.advance();
                Ok(Expr::Integer(n))
            }
            Token::Float(f) => {
                self.advance();
                Ok(Expr::Float(f))
            }
            Token::String(s) => {
                self.advance();
                Ok(Expr::String(s))
            }
            Token::FString(s) => {
                self.advance();
                Ok(Expr::FString(s))
            }
            Token::Boolean(b) => {
                self.advance();
                Ok(Expr::Boolean(b))
            }
            Token::Null => {
                self.advance();
                Ok(Expr::Null)
            }
            Token::Identifier(name) => {
                self.advance();
                // Walrus operator: name := expr
                if self.check(&Token::Operator(Operator::WalrusEq)) {
                    self.advance();
                    let value = self.parse_expression()?;
                    return Ok(Expr::WalrusExpr { name, value: Box::new(value) });
                }
                Ok(Expr::Identifier(name))
            }
            Token::Keyword(Keyword::Self_) => {
                self.advance();
                Ok(Expr::Identifier("هذا".to_string()))
            }
            Token::Keyword(Keyword::Super) => {
                self.advance();
                if self.check(&Token::Delimiter(Delimiter::LParen)) {
                    self.advance();
                    self.consume_delim(Delimiter::RParen)?;
                }
                Ok(Expr::Super)
            }
            Token::Delimiter(Delimiter::LParen) => {
                self.advance();
                if self.check(&Token::Delimiter(Delimiter::RParen)) {
                    self.advance();
                    return Ok(Expr::Tuple(Vec::new()));
                }
                let expr = self.parse_expression()?;
                if self.check(&Token::Delimiter(Delimiter::Comma)) {
                    let mut items = vec![expr];
                    while self.check(&Token::Delimiter(Delimiter::Comma)) {
                        self.advance();
                        items.push(self.parse_expression()?);
                    }
                    self.consume_delim(Delimiter::RParen)?;
                    Ok(Expr::Tuple(items))
                } else {
                    self.consume_delim(Delimiter::RParen)?;
                    Ok(expr)
                }
            }
            Token::Delimiter(Delimiter::LBrack) => {
                self.advance();
                if self.check(&Token::Delimiter(Delimiter::RBrack)) {
                    self.advance();
                    return Ok(Expr::List(Vec::new()));
                }
                let first_expr = self.parse_expression()?;
                // Check for list comprehension: [expr لكل var في iterable]
                if self.check(&Token::Keyword(Keyword::For)) {
                    self.advance();
                    let target = self.parse_identifier()?;
                    self.consume(Keyword::In)?;
                    let iter = self.parse_or()?;
                    let condition = if self.check(&Token::Keyword(Keyword::If)) {
                        self.advance();
                        Some(Box::new(self.parse_expression()?))
                    } else {
                        None
                    };
                    self.consume_delim(Delimiter::RBrack)?;
                    return Ok(Expr::ListComp {
                        expr: Box::new(first_expr),
                        iter: Box::new(iter),
                        target,
                        condition,
                    });
                }
                let mut items = vec![first_expr];
                while self.check(&Token::Delimiter(Delimiter::Comma)) {
                    self.advance();
                    if self.check(&Token::Delimiter(Delimiter::RBrack)) {
                        break;
                    }
                    items.push(self.parse_expression()?);
                }
                self.consume_delim(Delimiter::RBrack)?;
                Ok(Expr::List(items))
            }
            Token::Delimiter(Delimiter::LBrace) => {
                self.advance();
                if self.check(&Token::Delimiter(Delimiter::RBrace)) {
                    self.advance();
                    return Ok(Expr::Dict(Vec::new()));
                }
                let first = self.parse_expression()?;
                if self.check(&Token::Delimiter(Delimiter::Colon)) {
                    let mut items = Vec::new();
                    self.advance();
                    let value = self.parse_expression()?;
                    if self.check(&Token::Keyword(Keyword::For)) {
                        self.advance();
                        let target = self.parse_identifier()?;
                        self.consume(Keyword::In)?;
                        let iter = self.parse_or()?;
                        let condition = if self.check(&Token::Keyword(Keyword::If)) {
                            self.advance();
                            Some(Box::new(self.parse_expression()?))
                        } else {
                            None
                        };
                        self.consume_delim(Delimiter::RBrace)?;
                        return Ok(Expr::DictComp {
                            key: Box::new(first),
                            value: Box::new(value),
                            iter: Box::new(iter),
                            target,
                            condition,
                        });
                    }
                    items.push((first, value));
                    while self.check(&Token::Delimiter(Delimiter::Comma)) {
                        self.advance();
                        let key = self.parse_expression()?;
                        self.consume_delim(Delimiter::Colon)?;
                        let value = self.parse_expression()?;
                        items.push((key, value));
                    }
                    self.consume_delim(Delimiter::RBrace)?;
                    Ok(Expr::Dict(items))
                } else if self.check(&Token::Keyword(Keyword::For)) {
                    self.advance();
                    let target = self.parse_identifier()?;
                    self.consume(Keyword::In)?;
                    let iter = self.parse_or()?;
                    let condition = if self.check(&Token::Keyword(Keyword::If)) {
                        self.advance();
                        Some(Box::new(self.parse_expression()?))
                    } else {
                        None
                    };
                    self.consume_delim(Delimiter::RBrace)?;
                    Ok(Expr::SetComp {
                        expr: Box::new(first),
                        iter: Box::new(iter),
                        target,
                        condition,
                    })
                } else {
                    let mut items = vec![first];
                    while self.check(&Token::Delimiter(Delimiter::Comma)) {
                        self.advance();
                        items.push(self.parse_expression()?);
                    }
                    self.consume_delim(Delimiter::RBrace)?;
                    Ok(Expr::Set(items))
                }
            }
            Token::Keyword(Keyword::Lambda) => {
                self.advance();
                let mut params = Vec::new();
                while !self.check(&Token::Delimiter(Delimiter::Colon)) {
                    params.push(self.parse_identifier()?);
                    if self.check(&Token::Delimiter(Delimiter::Comma)) {
                        self.advance();
                    }
                }
                self.advance();
                let body = self.parse_expression()?;
                Ok(Expr::Lambda {
                    params,
                    body: Box::new(body),
                })
            }
            Token::Keyword(Keyword::Yield) => {
                self.advance();
                if self.check(&Token::Newline) || self.is_at_end() {
                    Ok(Expr::YieldExpr(None))
                } else {
                    Ok(Expr::YieldExpr(Some(Box::new(self.parse_expression()?))))
                }
            }
            _ => Err(ArabiError::ParseError {
                message: format!("token غير متوقع: {:?}", self.current()),
                span: self.current_span(),
            }),
        }
    }

    fn parse_identifier(&mut self) -> Result<String> {
        match self.current() {
            Token::Identifier(name) => {
                let name = name.clone();
                self.advance();
                Ok(name)
            }
            Token::Keyword(Keyword::Self_) => {
                self.advance();
                Ok("هذا".to_string())
            }
            _ => Err(ArabiError::ParseError {
                message: "متغير متوقع".to_string(),
                span: self.current_span(),
            }),
        }
    }

    fn parse_with(&mut self) -> Result<Stmt> {
        self.advance(); // consume 'باستخدام'
        let context = self.parse_expression()?;
        let target = if self.check(&Token::Keyword(Keyword::As)) {
            self.advance();
            Some(self.parse_identifier()?)
        } else {
            None
        };
        self.consume_delim(Delimiter::Colon)?;
        let body = self.parse_block()?;
        Ok(Stmt::With { context, target, body })
    }

    fn parse_params(&mut self) -> Result<Vec<Param>> {
        let mut params = Vec::new();

        while !self.check(&Token::Delimiter(Delimiter::RParen)) {
            let mut is_varargs = false;
            let mut is_kwargs = false;

            if self.check(&Token::Operator(Operator::Star)) {
                self.advance();
                is_varargs = true;
            } else if self.check(&Token::Operator(Operator::Caret)) {
                self.advance();
                is_kwargs = true;
            }

            let name = self.parse_identifier()?;
            let default = if self.check(&Token::Operator(Operator::Assign)) {
                self.advance();
                Some(self.parse_expression()?)
            } else {
                None
            };

            params.push(Param {
                name,
                default,
                is_varargs,
                is_kwargs,
            });

            if self.check(&Token::Delimiter(Delimiter::Comma)) {
                self.advance();
            }
        }

        Ok(params)
    }

    fn parse_block(&mut self) -> Result<Block> {
        let span = self.current_span();
        let mut stmts = Vec::new();

        // Skip to the first indent or statement
        while self.check(&Token::Newline) || matches!(self.current(), Token::Indent(_)) {
            self.advance();
        }

        while !self.is_at_end() && !matches!(self.current(), Token::Dedent(_)) {
            if self.check(&Token::Newline) {
                self.advance();
                continue;
            }
            if matches!(self.current(), Token::Indent(_)) {
                self.advance();
                continue;
            }
            if self.check(&Token::Delimiter(Delimiter::Semicolon)) {
                self.advance();
                continue;
            }
            stmts.push(self.parse_statement()?);
            while self.check(&Token::Delimiter(Delimiter::Semicolon)) {
                self.advance();
            }
        }

        if matches!(self.current(), Token::Dedent(_)) {
            self.advance();
        }

        Ok(Block { stmts, span })
    }

    fn check_aug_assign(&self) -> Option<AugOp> {
        match self.current() {
            Token::Operator(Operator::PlusEq) => Some(AugOp::Add),
            Token::Operator(Operator::MinusEq) => Some(AugOp::Sub),
            Token::Operator(Operator::StarEq) => Some(AugOp::Mul),
            Token::Operator(Operator::SlashEq) => Some(AugOp::Div),
            Token::Operator(Operator::BackslashEq) | Token::Operator(Operator::DoubleBackslashEq) => Some(AugOp::FloorDiv),
            Token::Operator(Operator::PercentEq) => Some(AugOp::Mod),
            Token::Operator(Operator::DoubleStarEq) => Some(AugOp::Pow),
            Token::Operator(Operator::CaretEq) => Some(AugOp::Pow),
            Token::Operator(Operator::AmpersandEq) => Some(AugOp::BitAnd),
            Token::Operator(Operator::PipeEq) => Some(AugOp::BitOr),
            Token::Operator(Operator::ShlEq) => Some(AugOp::Shl),
            Token::Operator(Operator::ShrEq) => Some(AugOp::Shr),
            _ => None,
        }
    }

    fn consume(&mut self, keyword: Keyword) -> Result<()> {
        match self.current() {
            Token::Keyword(k) if *k == keyword => {
                self.advance();
                Ok(())
            }
            _ => Err(ArabiError::ParseError {
                message: format!("كلمة مفتاحية متوقعة: {:?}", keyword),
                span: self.current_span(),
            }),
        }
    }

    fn consume_delim(&mut self, delim: Delimiter) -> Result<()> {
        match self.current() {
            Token::Delimiter(d) if *d == delim => {
                self.advance();
                Ok(())
            }
            _ => Err(ArabiError::ParseError {
                message: format!("فاصل متوقع: {:?}", delim),
                span: self.current_span(),
            }),
        }
    }

    fn check(&self, token: &Token) -> bool {
        self.current() == token
    }

    fn current(&self) -> &Token {
        self.tokens
            .get(self.position)
            .map(|t| &t.token)
            .unwrap_or(&Token::Eof)
    }

    fn current_span(&self) -> arabi_core::span::Span {
        self.tokens
            .get(self.position)
            .map(|t| t.span)
            .unwrap_or_else(|| arabi_core::span::Span::single(arabi_core::span::Position::start()))
    }

    fn current_line(&self) -> usize {
        self.current_span().start.line
    }

    fn peek_token(&self) -> Option<&Token> {
        self.tokens.get(self.position + 1).map(|t| &t.token)
    }

    fn advance(&mut self) -> Token {
        let token = self.tokens
            .get(self.position)
            .map(|t| t.token.clone())
            .unwrap_or(Token::Eof);
        self.position += 1;
        token
    }

    fn is_at_end(&self) -> bool {
        self.position >= self.tokens.len()
            || matches!(self.current(), Token::Eof)
    }
}
