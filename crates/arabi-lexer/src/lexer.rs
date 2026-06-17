use arabi_core::span::{Position, Span};
use arabi_core::token::{Token, SpannedToken, Operator, Delimiter};
use arabi_core::error::{ArabiError, Result};
use crate::keywords::KeywordMap;

pub struct Lexer<'a> {
    chars: Vec<char>,
    position: usize,
    line: usize,
    column: usize,
    keywords: KeywordMap,
    _marker: std::marker::PhantomData<&'a str>,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Lexer {
            chars: source.chars().collect(),
            position: 0,
            line: 1,
            column: 1,
            keywords: KeywordMap::new(),
            _marker: std::marker::PhantomData,
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<SpannedToken>> {
        let mut tokens = Vec::new();
        let mut indent_stack = vec![0];
        let mut at_line_start = true;

        loop {
            // Handle indentation at line start before skipping whitespace
            if at_line_start && !self.is_at_end() {
                let _saved_pos = self.position;
                let indent = self.count_indent();
                let current_indent = *indent_stack.last().unwrap();

                // Skip indent changes on blank lines (only whitespace + newline/comment/eof)
                let is_blank_line = matches!(
                    self.current_char(),
                    None | Some('\n') | Some('\r') | Some('#')
                );

                if !is_blank_line {
                    if indent > current_indent {
                        indent_stack.push(indent);
                        let start = self.current_position();
                        let span = Span::single(start);
                        tokens.push(SpannedToken::new(Token::Indent(indent), span));
                    } else if indent < current_indent {
                        while let Some(&top) = indent_stack.last() {
                            if top <= indent {
                                break;
                            }
                            indent_stack.pop();
                            let start = self.current_position();
                            let span = Span::single(start);
                            tokens.push(SpannedToken::new(Token::Dedent(indent), span));
                        }
                    }
                }
                at_line_start = false;
            }

            // Skip inline whitespace (but not newlines)
            self.skip_whitespace();

            if self.is_at_end() {
                break;
            }

            // Handle newlines
            if self.current_char() == Some('\n') {
                let start = self.current_position();
                let span = Span::single(start);
                tokens.push(SpannedToken::new(Token::Newline, span));
                self.advance();
                self.line += 1;
                self.column = 1;
                at_line_start = true;
                continue;
            }

            if self.current_char() == Some('#') {
                self.skip_comment();
                // After comment, check for newline
                if self.current_char() == Some('\n') {
                    let start = self.current_position();
                    let span = Span::single(start);
                    tokens.push(SpannedToken::new(Token::Newline, span));
                    self.advance();
                    self.line += 1;
                    self.column = 1;
                    at_line_start = true;
                }
                continue;
            }

            match self.read_token() {
                Ok(Some(token)) => {
                    tokens.push(token);
                }
                Ok(None) => continue,
                Err(e) => return Err(e),
            }
        }

        // Close remaining indents
        while indent_stack.len() > 1 {
            indent_stack.pop();
            let start = self.current_position();
            let span = Span::single(start);
            tokens.push(SpannedToken::new(Token::Dedent(0), span));
        }

        let start = self.current_position();
        let span = Span::single(start);
        tokens.push(SpannedToken::new(Token::Eof, span));

        Ok(tokens)
    }

    fn read_token(&mut self) -> Result<Option<SpannedToken>> {
        let _start = self.current_position();

        match self.current_char() {
            None => Ok(None),
            Some('\n') => Ok(None),
            Some(c) => {
                if c == '#' {
                    self.skip_comment();
                    Ok(None)
                } else if c == '"' || c == '\'' {
                    self.read_string(c).map(Some)
                } else if (c == 'م' || c == 'm' || c == 'ف' || c == 'f') && (self.peek() == Some('"') || self.peek() == Some('\'')) {
                    self.read_f_string(self.peek().unwrap()).map(Some)
                } else if c.is_ascii_digit() {
                    self.read_number().map(Some)
                } else if self.is_delimiter_char(c) {
                    self.read_delimiter().map(Some)
                } else if self.is_operator_char(c) {
                    self.read_operator().map(Some)
                } else if self.is_arabic_char(c) || c == '_' || c.is_ascii_alphanumeric() {
                    self.read_identifier().map(Some)
                } else {
                    self.advance();
                    Ok(None)
                }
            }
        }
    }

    fn read_string(&mut self, quote: char) -> Result<SpannedToken> {
        let start = self.current_position();
        self.advance(); // Skip opening quote
        let mut value = String::new();

        loop {
            match self.current_char() {
                None => return Err(ArabiError::LexError {
                    message: "نص غير مكتمل".to_string(),
                    span: Span::single(start),
                }),
                Some(c) if c == quote => {
                    self.advance();
                    break;
                }
                Some('\\') => {
                    self.advance();
                    match self.current_char() {
                        Some('س') => { value.push('\n'); self.advance(); }
                        Some('ت') => { value.push('\t'); self.advance(); }
                        Some('\\') => { value.push('\\'); self.advance(); }
                        Some(c) => { value.push(c); self.advance(); }
                        None => {}
                    }
                }
                Some(c) => {
                    value.push(c);
                    self.advance();
                }
            }
        }

        let end = self.current_position();
        let span = Span::new(start, end);
        Ok(SpannedToken::new(Token::String(value), span))
    }

    fn read_f_string(&mut self, quote: char) -> Result<SpannedToken> {
        let start = self.current_position();
        self.advance(); // Skip prefix (م/م/ف/f)
        self.advance(); // Skip opening quote
        let mut value = String::new();

        loop {
            match self.current_char() {
                None => return Err(ArabiError::LexError {
                    message: "نص منسق غير مكتمل".to_string(),
                    span: Span::single(start),
                }),
                Some(c) if c == quote => {
                    self.advance();
                    break;
                }
                Some('\\') => {
                    self.advance();
                    match self.current_char() {
                        Some('س') => { value.push('\n'); self.advance(); }
                        Some('ت') => { value.push('\t'); self.advance(); }
                        Some('\\') => { value.push('\\'); self.advance(); }
                        Some('"') => { value.push('"'); self.advance(); }
                        Some('\'') => { value.push('\''); self.advance(); }
                        Some('{') => { value.push('{'); self.advance(); }
                        Some('}') => { value.push('}'); self.advance(); }
                        Some(c) => { value.push('\\'); value.push(c); self.advance(); }
                        None => {}
                    }
                }
                Some(c) => {
                    value.push(c);
                    self.advance();
                }
            }
        }

        let end = self.current_position();
        let span = Span::new(start, end);
        Ok(SpannedToken::new(Token::FString(value), span))
    }

    fn read_number(&mut self) -> Result<SpannedToken> {
        let start = self.current_position();
        let mut value = String::new();
        let mut is_float = false;

        while self.current_char().is_some_and(|c| c.is_ascii_digit()) {
            value.push(self.current_char().unwrap());
            self.advance();
        }

        if self.current_char() == Some('.') && self.peek().is_some_and(|c| c.is_ascii_digit()) {
            is_float = true;
            value.push('.');
            self.advance();

            while self.current_char().is_some_and(|c| c.is_ascii_digit()) {
                value.push(self.current_char().unwrap());
                self.advance();
            }
        }

        let end = self.current_position();
        let span = Span::new(start, end);

        if is_float {
            let num: f64 = value.parse().map_err(|_| ArabiError::LexError {
                message: format!("رقم عشري غير صالح: {}", value),
                span,
            })?;
            Ok(SpannedToken::new(Token::Float(num), span))
        } else {
            let num: i64 = value.parse().map_err(|_| ArabiError::LexError {
                message: format!("رقم صحيح غير صالح: {}", value),
                span,
            })?;
            Ok(SpannedToken::new(Token::Integer(num), span))
        }
    }

    fn read_identifier(&mut self) -> Result<SpannedToken> {
        let start = self.current_position();
        let mut value = String::new();

        while self.current_char().is_some_and(|c| self.is_identifier_char(c)) {
            value.push(self.current_char().unwrap());
            self.advance();
        }

        let end = self.current_position();
        let span = Span::new(start, end);

        // Check for keywords
        if let Some(keyword) = self.keywords.lookup(&value) {
            match keyword {
                arabi_core::token::Keyword::True => Ok(SpannedToken::new(Token::Boolean(true), span)),
                arabi_core::token::Keyword::False => Ok(SpannedToken::new(Token::Boolean(false), span)),
                arabi_core::token::Keyword::None => Ok(SpannedToken::new(Token::Null, span)),
                _ => Ok(SpannedToken::new(Token::Keyword(*keyword), span)),
            }
        } else if value == "والا" {
            // Handle 'والا' as both keyword and operator
            Ok(SpannedToken::new(Token::Keyword(arabi_core::token::Keyword::Else), span))
        } else {
            Ok(SpannedToken::new(Token::Identifier(value), span))
        }
    }

    fn read_operator(&mut self) -> Result<SpannedToken> {
        let start = self.current_position();
        let c = self.current_char().unwrap();
        let next = self.peek();

        let (op, consume_count) = match c {
            '+' => match next {
                Some('=') => (Operator::PlusEq, 2),
                _ => (Operator::Plus, 1),
            },
            '-' => match next {
                Some('=') => (Operator::MinusEq, 2),
                Some('>') => (Operator::Arrow, 2),
                _ => (Operator::Minus, 1),
            },
            '*' => match next {
                Some('*') => match self.peek_ahead(2) {
                    Some('=') => (Operator::DoubleStarEq, 3),
                    _ => (Operator::DoubleStar, 2),
                },
                Some('=') => (Operator::StarEq, 2),
                _ => (Operator::Star, 1),
            },
            '/' => match next {
                Some('=') => (Operator::SlashEq, 2),
                _ => (Operator::Slash, 1),
            },
            '\\' => match next {
                Some('\\') => match self.peek_ahead(2) {
                    Some('=') => (Operator::DoubleBackslashEq, 3),
                    _ => (Operator::DoubleBackslash, 2),
                },
                Some('=') => (Operator::BackslashEq, 2),
                _ => (Operator::Backslash, 1),
            },
            '^' => match next {
                Some('=') => (Operator::CaretEq, 2),
                _ => (Operator::Caret, 1),
            },
            '%' => match next {
                Some('=') => (Operator::PercentEq, 2),
                _ => (Operator::Percent, 1),
            },
            '=' => match next {
                Some('=') => (Operator::Eq, 2),
                _ => (Operator::Assign, 1),
            },
            '!' => match next {
                Some('=') => (Operator::NotEq, 2),
                _ => return Err(ArabiError::LexError {
                    message: "عامل غير متوقع: !".to_string(),
                    span: Span::single(start),
                }),
            },
            '<' => match next {
                Some('<') => match self.peek_ahead(2) {
                    Some('=') => (Operator::ShlEq, 3),
                    _ => (Operator::Shl, 2),
                },
                Some('=') => (Operator::LtEq, 2),
                _ => (Operator::Lt, 1),
            },
            '>' => match next {
                Some('>') => match self.peek_ahead(2) {
                    Some('=') => (Operator::ShrEq, 3),
                    _ => (Operator::Shr, 2),
                },
                Some('=') => (Operator::GtEq, 2),
                _ => (Operator::Gt, 1),
            },
            '&' => match next {
                Some('=') => (Operator::AmpersandEq, 2),
                _ => (Operator::Ampersand, 1),
            },
            '|' => match next {
                Some('=') => (Operator::PipeEq, 2),
                _ => (Operator::Pipe, 1),
            },
            '~' => (Operator::Tilde, 1),
            '@' => (Operator::At, 1),
            _ => return Err(ArabiError::LexError {
                message: format!("عامل غير معروف: {}", c),
                span: Span::single(start),
            }),
        };

        for _ in 0..consume_count {
            self.advance();
        }

        let end = self.current_position();
        let span = Span::new(start, end);
        Ok(SpannedToken::new(Token::Operator(op), span))
    }

    fn read_delimiter(&mut self) -> Result<SpannedToken> {
        let start = self.current_position();
        let c = self.current_char().unwrap();

        let delim = match c {
            ':' => {
                if self.peek() == Some('=') {
                    self.advance(); // consume ':'
                    self.advance(); // consume '='
                    let end = self.current_position();
                    let span = Span::new(start, end);
                    return Ok(SpannedToken::new(Token::Operator(Operator::WalrusEq), span));
                }
                Delimiter::Colon
            },
            '؛' => Delimiter::Semicolon,
            ',' | '،' => Delimiter::Comma,
            '.' => Delimiter::Dot,
            '(' => Delimiter::LParen,
            ')' => Delimiter::RParen,
            '[' => Delimiter::LBrack,
            ']' => Delimiter::RBrack,
            '{' => Delimiter::LBrace,
            '}' => Delimiter::RBrace,
            _ => return Err(ArabiError::LexError {
                message: format!("فاصل غير معروف: {}", c),
                span: Span::single(start),
            }),
        };

        self.advance();
        let end = self.current_position();
        let span = Span::new(start, end);
        Ok(SpannedToken::new(Token::Delimiter(delim), span))
    }

    fn skip_whitespace(&mut self) {
        while self.current_char().is_some_and(|c| c == ' ' || c == '\t' || c == '\r') {
            self.advance();
        }
    }

    fn skip_comment(&mut self) {
        while self.current_char().is_some_and(|c| c != '\n') {
            self.advance();
        }
    }

    fn count_indent(&mut self) -> usize {
        let mut count = 0;
        while self.current_char().is_some_and(|c| c == ' ' || c == '\t') {
            if self.current_char() == Some('\t') {
                count += 4;
            } else {
                count += 1;
            }
            self.advance();
        }
        count
    }

    fn current_char(&self) -> Option<char> {
        self.chars.get(self.position).copied()
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.position + 1).copied()
    }

    fn peek_ahead(&self, offset: usize) -> Option<char> {
        self.chars.get(self.position + offset).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.chars.get(self.position).copied();
        if c.is_some() {
            self.position += 1;
            self.column += 1;
        }
        c
    }

    fn is_at_end(&self) -> bool {
        self.position >= self.chars.len()
    }

    fn current_position(&self) -> Position {
        Position::new(self.line, self.column)
    }

    fn is_arabic_char(&self, c: char) -> bool {
        let cp = c as u32;
        (0x0600..=0x06FF).contains(&cp) ||
        (0x0750..=0x077F).contains(&cp) ||
        (0x08A0..=0x08FF).contains(&cp) ||
        (0xFB50..=0xFDFF).contains(&cp) ||
        (0xFE70..=0xFEFE).contains(&cp)
    }

    fn is_identifier_char(&self, c: char) -> bool {
        (self.is_arabic_char(c) && !matches!(c, '؛' | '،')) || c == '_' || c.is_ascii_alphanumeric()
    }

    fn is_operator_char(&self, c: char) -> bool {
        matches!(c, '+' | '-' | '*' | '/' | '\\' | '^' | '%' | '=' | '!' | '<' | '>' | '@' | '&' | '|' | '~')
    }

    fn is_delimiter_char(&self, c: char) -> bool {
        matches!(c, ':' | '؛' | ',' | '،' | '.' | '(' | ')' | '[' | ']' | '{' | '}')
    }
}
