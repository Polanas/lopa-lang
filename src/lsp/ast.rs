use logos::Logos;

#[derive(Logos, Debug, PartialEq, Clone, Copy)]
pub(super) enum Token {
    #[token("(")]
    LeftParen,
    #[token(")")]
    RightParen,
    #[token("[")]
    LeftBracket,
    #[token("]")]
    RightBracket,
    #[token("{")]
    LeftBrace,
    #[token("}")]
    RightBrace,
    #[token("|")]
    Bar,
    #[token("=")]
    Eq,
    #[token(",")]
    Comma,
    #[token(".")]
    Dot,
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("/")]
    Slash,
    #[token("*")]
    Star,
    #[token(";")]
    Semi,
    #[token("let")]
    Let,
    #[token("fn")]
    Fn,
    #[regex("[_]?[A-Za-z_][0-9A-Za-z_]*")]
    Ident,
    #[regex("[\\d][\\d|_]*(.[\\d]+)?")]
    Num,
    #[regex("\\s+")]
    Whitespace,
    #[end]
    EndOfLine,
}

#[cfg(test)]
mod test {
    use itertools::Itertools;
    use logos::Logos as _;

    use crate::lsp::ast::Token;

    #[test]
    fn simple_fn() {
        let lex = Token::lexer("fn main() { let x = 5; }")
            .map(|t| t.unwrap())
            .filter(|t| *t != Token::Whitespace)
            .collect_vec();

        assert_eq!(lex.as_slice(),&[
            Token::Fn,
            Token::Ident,
            Token::LeftParen,
            Token::RightParen,
            Token::LeftBrace,
            Token::Let,
            Token::Ident,
            Token::Eq,
            Token::Num,
            Token::Semi,
            Token::RightBrace,
        ]);
    }
}
