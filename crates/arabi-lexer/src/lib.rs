pub mod lexer;
pub mod keywords;

pub use lexer::Lexer;

#[cfg(test)]
mod tests {
    use super::Lexer;
    use arabi_core::token::{Token, Keyword, Operator, Delimiter};

    #[test]
    fn test_basic_tokens() {
        let source = "س = 9";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        
        assert!(tokens.len() >= 3);
        assert_eq!(tokens[0].token, Token::Identifier("س".to_string()));
        assert_eq!(tokens[1].token, Token::Operator(Operator::Assign));
        assert_eq!(tokens[2].token, Token::Integer(9));
    }

    #[test]
    fn test_arithmetic() {
        let source = "س = 5 + 3 * 2";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        
        assert!(tokens.len() >= 5);
        assert_eq!(tokens[2].token, Token::Integer(5));
        assert_eq!(tokens[3].token, Token::Operator(Operator::Plus));
        assert_eq!(tokens[4].token, Token::Integer(3));
    }

    #[test]
    fn test_keywords() {
        let source = "اذا س > 5:";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        
        assert!(tokens.len() >= 4);
        assert_eq!(tokens[0].token, Token::Keyword(Keyword::If));
        assert_eq!(tokens[2].token, Token::Operator(Operator::Gt));
        assert_eq!(tokens[3].token, Token::Integer(5));
    }

    #[test]
    fn test_string() {
        let source = "س = \"مرحبا\"";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        
        assert!(tokens.len() >= 3);
        assert_eq!(tokens[2].token, Token::String("مرحبا".to_string()));
    }

    #[test]
    fn test_function_def() {
        let source = "دالة جمع(ا، ب):";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        
        assert!(tokens.len() >= 5);
        assert_eq!(tokens[0].token, Token::Keyword(Keyword::Function));
        assert_eq!(tokens[1].token, Token::Identifier("جمع".to_string()));
    }

    #[test]
    fn test_class_def() {
        let source = "صنف شخص:";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        
        assert!(tokens.len() >= 2);
        assert_eq!(tokens[0].token, Token::Keyword(Keyword::Class));
        assert_eq!(tokens[1].token, Token::Identifier("شخص".to_string()));
    }

    #[test]
    fn test_comparison() {
        let source = "س == 5 و ص != 3";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        
        assert!(tokens.len() >= 5);
        assert_eq!(tokens[1].token, Token::Operator(Operator::Eq));
        assert_eq!(tokens[3].token, Token::Keyword(Keyword::And));
    }

    #[test]
    fn test_list() {
        let source = "س = [1, 2, 3]";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        
        let lbrack_pos = tokens.iter().position(|t| t.token == Token::Delimiter(Delimiter::LBrack)).unwrap();
        let rbrack_pos = tokens.iter().position(|t| t.token == Token::Delimiter(Delimiter::RBrack)).unwrap();
        assert_eq!(tokens[lbrack_pos].token, Token::Delimiter(Delimiter::LBrack));
        assert_eq!(tokens[rbrack_pos].token, Token::Delimiter(Delimiter::RBrack));
        assert!(rbrack_pos > lbrack_pos);
    }
}
