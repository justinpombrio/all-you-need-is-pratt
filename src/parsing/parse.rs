use super::grammar::{Parser, Pattern, Token};
use crate::lexing::Span;
use crate::rpn_visitor::Stack as RpnStack;
use crate::rpn_visitor::Visitor as RpnVisitor;
use crate::rpn_visitor::VisitorIter as RpnVisitorIter;
use crate::shunting::{Fixity, Node, ShuntError};
use std::error::Error;
use std::fmt;

// TODO: Get line&col nums
#[derive(Debug, Clone)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

#[derive(Debug)]
pub struct Parsed<'a> {
    source: &'a str,
    stack: RpnStack<Node<'a, Token>>,
}

#[derive(Debug, Clone, Copy)]
pub struct Visitor<'a> {
    source: &'a str,
    visitor: RpnVisitor<'a, Node<'a, Token>>,
}

#[derive(Debug, Clone)]
pub enum ParseError {
    LexError {
        lexeme: String,
        pos: Position,
    },
    ExtraSeparator {
        separator: String,
        pos: Position,
    },
    MissingSeparator {
        op_name: String,
        separator: String,
        pos: Position,
    },
}

impl Error for ParseError {}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use ParseError::*;

        match self {
            LexError{lexeme, pos} => write!(
                f,
                "Lexing failed. It did not recognize the characters '{}'. Line {} ({}:{})",
                lexeme, pos.line, pos.line, pos.column
            ),
            ExtraSeparator{separator, pos} => write!(
               f,
               "Parsing failed. It did not expect to find '{}' on its own. Line {} ({}:{})",
               separator, pos.line, pos.line, pos.column
            ),
            MissingSeparator{op_name, separator, pos} => write!(
            f,
            "Parsing failed. It expected to find '{}' as part of {}, but could not. Line {} ({}:{})",
            op_name, separator, pos.line, pos.line, pos.column
            ),
        }
    }
}

impl Parser {
    pub fn parse<'s>(&'s self, source: &'s str) -> Result<Parsed<'s>, ParseError> {
        let tokens = self.lexer.lex(source);
        let rpn = self.shunter.shunt(tokens);
        let mut stack = RpnStack::new();
        for node in rpn {
            match node {
                Err(ShuntError::LexError(lexeme)) => {
                    let pos = Position {
                        line: 0,
                        column: lexeme.span.0 + 1,
                    };
                    let lexeme = source[lexeme.span.0..lexeme.span.1].to_owned();
                    return Err(ParseError::LexError { lexeme, pos });
                }
                Err(ShuntError::ExtraSep(lexeme)) => {
                    let pos = Position {
                        line: 0,
                        column: lexeme.span.0 + 1,
                    };
                    let separator = source[lexeme.span.0..lexeme.span.1].to_owned();
                    return Err(ParseError::ExtraSeparator { separator, pos });
                }
                Err(ShuntError::MissingSep {
                    op_name,
                    span,
                    token,
                }) => {
                    let pos = Position {
                        line: 0,
                        column: span.0 + 1,
                    };
                    let separator = match self.token_patterns.get(&token).unwrap() {
                        Pattern::Constant(constant) => format!("{}", constant),
                        Pattern::Regex(regex) => format!("/{}/", regex),
                    };
                    return Err(ParseError::MissingSeparator {
                        op_name,
                        separator,
                        pos,
                    });
                }
                Ok(node) => stack.push(node),
            }
        }
        Ok(Parsed { source, stack })
    }
}

impl<'a> Parsed<'a> {
    pub fn source(&self) -> &'a str {
        self.source
    }

    pub fn groups(&self) -> VisitorIter {
        VisitorIter {
            source: self.source,
            iter: self.stack.groups(),
        }
    }
}

impl<'a> Visitor<'a> {
    pub fn name(&self) -> &'a str {
        &self.visitor.node().op.name()
    }

    pub fn fixity(&self) -> Fixity {
        self.visitor.node().op.fixity()
    }

    pub fn op_patterns<'p>(&self, parser: &'p Parser) -> Vec<Option<&'p Pattern>> {
        self.visitor
            .node()
            .op
            .tokens()
            .iter()
            .map(|tok| parser.token_patterns.get(tok))
            .collect()
    }

    pub fn span(&self) -> Span {
        self.visitor.node().span
    }

    pub fn arity(&self) -> usize {
        self.visitor.node().arity()
    }

    pub fn text(&self) -> &'a str {
        self.visitor.node().text(self.source)
    }

    pub fn children(&self) -> VisitorIter<'a> {
        VisitorIter {
            source: self.source,
            iter: self.visitor.children(),
        }
    }

    pub fn expect_2_children(&self) -> (Visitor<'a>, Visitor<'a>) {
        let mut children = self.children();
        assert_eq!(
            children.len(),
            2,
            "Visitor.expected_2_children: there weren't 2 children"
        );
        let child_1 = children.next().unwrap();
        let child_2 = children.next().unwrap();
        (child_1, child_2)
    }

    pub fn expect_3_children(&self) -> (Visitor<'a>, Visitor<'a>, Visitor<'a>) {
        let mut children = self.children();
        assert_eq!(
            children.len(),
            3,
            "Visitor.expected_3_children: there weren't 3 children"
        );
        let child_1 = children.next().unwrap();
        let child_2 = children.next().unwrap();
        let child_3 = children.next().unwrap();
        (child_1, child_2, child_3)
    }

    pub fn expect_4_children(&self) -> (Visitor<'a>, Visitor<'a>, Visitor<'a>, Visitor<'a>) {
        let mut children = self.children();
        assert_eq!(
            children.len(),
            4,
            "Visitor.expected_4_children: there weren't 4 children"
        );
        let child_1 = children.next().unwrap();
        let child_2 = children.next().unwrap();
        let child_3 = children.next().unwrap();
        let child_4 = children.next().unwrap();
        (child_1, child_2, child_3, child_4)
    }
}

pub struct VisitorIter<'a> {
    source: &'a str,
    iter: RpnVisitorIter<'a, Node<'a, Token>>,
}

impl<'a> Iterator for VisitorIter<'a> {
    type Item = Visitor<'a>;
    fn next(&mut self) -> Option<Visitor<'a>> {
        match self.iter.next() {
            None => None,
            Some(v) => Some(Visitor {
                source: self.source,
                visitor: v,
            }),
        }
    }
}

impl<'a> ExactSizeIterator for VisitorIter<'a> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}
