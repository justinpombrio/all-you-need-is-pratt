use crate::{Lexeme, Prec};
use std::iter;

/// Convert a token stream into reverse polish notation. For example, `1 * 2 + 3 * 4` would become
/// `1 2 * 3 4 * +`.
///
/// The precedence table is indexed by `Token`, and says what that token's left and right
/// precedence is. Smaller precedence binds tighter.
pub fn shunt<'a, 's: 'a, I>(
    prec_table: &'a Vec<(Prec, Prec)>,
    iter: I,
) -> impl Iterator<Item = Lexeme<'s>> + 'a
where
    I: Iterator<Item = Lexeme<'s>> + 'a,
{
    Shunter {
        prec_table,
        stack: vec![],
        iter: iter.peekable(),
        pop_mode: false,
    }
}

struct Shunter<'a, 's, I>
where
    I: Iterator<Item = Lexeme<'s>>,
{
    prec_table: &'a Vec<(Prec, Prec)>,
    stack: Vec<Lexeme<'s>>,
    iter: iter::Peekable<I>,
    pop_mode: bool,
}

impl<'a, 's, I: Iterator<Item = Lexeme<'s>>> Shunter<'a, 's, I> {
    fn top_rprec(&self) -> Prec {
        self.stack
            .last()
            .map(|lex| self.prec_table[lex.token].1)
            .unwrap_or(Prec::MAX)
    }
}

impl<'a, 's, I: Iterator<Item = Lexeme<'s>>> Iterator for Shunter<'a, 's, I> {
    type Item = Lexeme<'s>;

    fn next(&mut self) -> Option<Lexeme<'s>> {
        loop {
            if self.pop_mode {
                let lexeme = self.stack.pop().unwrap();
                let lprec = self.prec_table[lexeme.token].0;
                let rprec = self.top_rprec();
                if rprec > lprec {
                    self.pop_mode = false;
                }
                return Some(lexeme);
            } else if let Some(lexeme) = self.iter.peek().copied() {
                let rprec = self.top_rprec();
                let lprec = self.prec_table[lexeme.token].0;
                if rprec >= lprec {
                    self.stack.push(lexeme);
                    self.iter.next();
                } else {
                    self.pop_mode = true;
                }
            } else {
                return self.stack.pop();
            }
        }
    }
}

#[test]
fn test_shunting() {
    use crate::{Position, Token, TOKEN_BLANK, TOKEN_ERROR, TOKEN_JUXTAPOSE};

    const TOKEN_ID: Token = 3;
    const TOKEN_TIMES: Token = 4;
    const TOKEN_PLUS: Token = 5;
    const TOKEN_NEG: Token = 6;
    const TOKEN_MINUS: Token = 7;
    const TOKEN_BANG: Token = 8;
    const TOKEN_OPEN: Token = 9;
    const TOKEN_CLOSE: Token = 10;
    const NUM_TOKENS: usize = 11;

    fn lex<'s>(src: &'s str) -> impl Iterator<Item = Lexeme<'s>> {
        let mut lexemes = vec![];
        for i in 0..src.len() {
            let ch = src[i..i + 1].chars().next().unwrap();
            if ch == ' ' {
                continue;
            }
            let token = match ch {
                '_' => TOKEN_BLANK,
                '.' => TOKEN_JUXTAPOSE,
                'a'..='z' => TOKEN_ID,
                '*' => TOKEN_TIMES,
                '+' => TOKEN_PLUS,
                '~' => TOKEN_NEG,
                '-' => TOKEN_MINUS,
                '!' => TOKEN_BANG,
                '(' => TOKEN_OPEN,
                ')' => TOKEN_CLOSE,
                _ => TOKEN_ERROR,
            };
            let pos = Position::start(); // we don't care about positions in this test
            lexemes.push(Lexeme::new(token, &src[i..i + 1], pos, pos));
        }
        lexemes.into_iter()
    }

    fn show_stream<'s>(stream: &mut impl Iterator<Item = Lexeme<'s>>) -> String {
        stream
            .map(|lex| if lex.lexeme == "" { "_" } else { lex.lexeme })
            .collect::<Vec<_>>()
            .join(" ")
    }

    let mut prec_table = Vec::new();
    for _ in 0..NUM_TOKENS {
        prec_table.push((0, 0));
    }
    prec_table[TOKEN_ERROR] = (0, 0);
    prec_table[TOKEN_BLANK] = (0, 0);
    prec_table[TOKEN_JUXTAPOSE] = (10, 10);
    prec_table[TOKEN_ID] = (0, 0);
    prec_table[TOKEN_BANG] = (50, 0);
    prec_table[TOKEN_TIMES] = (60, 60);
    prec_table[TOKEN_PLUS] = (100, 99);
    prec_table[TOKEN_MINUS] = (100, 99);
    prec_table[TOKEN_NEG] = (0, 80);
    prec_table[TOKEN_OPEN] = (0, 1000);
    prec_table[TOKEN_CLOSE] = (1000, 0);

    let src = "_";
    let lexemes = &mut shunt(&prec_table, lex(src));
    assert_eq!(show_stream(lexemes), "_");

    let src = "_+_";
    let lexemes = &mut shunt(&prec_table, lex(src));
    assert_eq!(show_stream(lexemes), "_ _ +");

    let src = "1-2+3*4*5!-~6";
    let lexemes = &mut shunt(&prec_table, lex(src));
    assert_eq!(show_stream(lexemes), "1 2 - 3 4 5 ! * * + 6 ~ -");

    let src = "(~_)";
    let lexemes = &mut shunt(&prec_table, lex(src));
    assert_eq!(show_stream(lexemes), "_ ~ ) (");

    let src = "%";
    let lexemes = &mut shunt(&prec_table, lex(src));
    assert_eq!(show_stream(lexemes), "%");

    let src = "1 + %";
    let lexemes = &mut shunt(&prec_table, lex(src));
    assert_eq!(show_stream(lexemes), "1 % +");
}
