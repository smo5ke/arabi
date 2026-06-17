pub mod ast;
pub mod parser;

pub use parser::Parser;

#[cfg(test)]
mod tests {
    use super::Parser;
    use arabi_lexer::Lexer;
    use crate::ast::*;

    fn parse(source: &str) -> Program {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        parser.parse().unwrap()
    }

    #[test]
    fn test_variable_assignment() {
        let program = parse("س = 9");
        assert_eq!(program.stmts.len(), 1);
        match &program.stmts[0] {
            Stmt::Assign { target, value } => {
                assert!(matches!(target, Expr::Identifier(name) if name == "س"));
                assert!(matches!(value, Expr::Integer(9)));
            }
            _ => panic!("Expected Assign statement"),
        }
    }

    #[test]
    fn test_binary_operation() {
        let program = parse("س = 5 + 3");
        assert_eq!(program.stmts.len(), 1);
        match &program.stmts[0] {
            Stmt::Assign { value, .. } => {
                assert!(matches!(value, Expr::BinaryOp { .. }));
            }
            _ => panic!("Expected Assign statement"),
        }
    }

    #[test]
    fn test_if_statement() {
        let program = parse("اذا س > 5:\n    اطبع(س)");
        assert_eq!(program.stmts.len(), 1);
        match &program.stmts[0] {
            Stmt::If { condition, body, .. } => {
                assert!(matches!(condition, Expr::BinaryOp { .. }));
                assert!(!body.stmts.is_empty());
            }
            _ => panic!("Expected If statement"),
        }
    }

    #[test]
    fn test_function_def() {
        let program = parse("دالة جمع(ا، ب):\n    ارجع ا + ب");
        assert_eq!(program.stmts.len(), 1);
        match &program.stmts[0] {
            Stmt::FunctionDef { name, params, body } => {
                assert_eq!(name, "جمع");
                assert_eq!(params.len(), 2);
                assert!(!body.stmts.is_empty());
            }
            _ => panic!("Expected FunctionDef statement"),
        }
    }

    #[test]
    fn test_function_call() {
        let program = parse("اطبع(5)");
        assert_eq!(program.stmts.len(), 1);
        match &program.stmts[0] {
            Stmt::Expr(Expr::Call { function, args, .. }) => {
                assert!(matches!(function.as_ref(), Expr::Identifier(name) if name == "اطبع"));
                assert_eq!(args.len(), 1);
            }
            _ => panic!("Expected function call"),
        }
    }

    #[test]
    fn test_list_literal() {
        let program = parse("س = [1, 2, 3]");
        assert_eq!(program.stmts.len(), 1);
        match &program.stmts[0] {
            Stmt::Assign { value, .. } => {
                assert!(matches!(value, Expr::List(items) if items.len() == 3));
            }
            _ => panic!("Expected Assign with List"),
        }
    }

    #[test]
    fn test_dict_literal() {
        let program = parse("س = {\"ا\": 1, \"ب\": 2}");
        assert_eq!(program.stmts.len(), 1);
        match &program.stmts[0] {
            Stmt::Assign { value, .. } => {
                assert!(matches!(value, Expr::Dict(items) if items.len() == 2));
            }
            _ => panic!("Expected Assign with Dict"),
        }
    }
}
