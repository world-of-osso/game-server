//! Pinned expressions.cpp22–154 lexer,333–440 evaluation,1081–1130 precedence.
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug)]
pub(super) enum Operator {
    Or,
    And,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Minimum,
    Maximum,
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
}

impl Operator {
    fn precedence(self) -> u8 {
        match self {
            Self::Or => 1,
            Self::And => 3,
            Self::Equal
            | Self::NotEqual
            | Self::Less
            | Self::LessEqual
            | Self::Greater
            | Self::GreaterEqual => 4,
            Self::Minimum | Self::Maximum => 5,
            Self::Add | Self::Subtract => 6,
            Self::Multiply | Self::Divide | Self::Modulo => 7,
        }
    }

    fn apply(self, left: f64, right: f64) -> f64 {
        match self {
            Self::Equal => truth(left == right),
            Self::NotEqual => truth(left != right),
            Self::Less => truth(left < right),
            Self::LessEqual => truth(left <= right),
            Self::Greater => truth(left > right),
            Self::GreaterEqual => truth(left >= right),
            // Match C++ std::min/max comparisons, including left-operand ties.
            Self::Minimum if right < left => right,
            Self::Maximum if left < right => right,
            Self::Minimum | Self::Maximum => left,
            Self::Add => left + right,
            Self::Subtract => left - right,
            Self::Multiply => left * right,
            Self::Divide => left / right,
            Self::Modulo => left % right,
            Self::And | Self::Or => unreachable!("logical operators use lazy evaluation"),
        }
    }
}

fn truth(value: bool) -> f64 {
    if value { 1.0 } else { 0.0 }
}

#[derive(Clone, Copy, Debug)]
pub(super) enum Unary {
    Not,
    Positive,
    Negative,
}

#[derive(Debug)]
pub(super) enum Node {
    Number(f64),
    Fact(String),
    Unary(Unary, Box<Node>),
    Binary(Operator, Box<Node>, Box<Node>),
}

impl Node {
    pub(super) fn collect_operands<'a>(&'a self, names: &mut BTreeSet<&'a str>) {
        match self {
            Self::Number(_) => {}
            Self::Fact(name) => {
                names.insert(name);
            }
            Self::Unary(_, inner) => inner.collect_operands(names),
            Self::Binary(_, left, right) => {
                left.collect_operands(names);
                right.collect_operands(names);
            }
        }
    }

    pub(super) fn evaluate(&self, values: &BTreeMap<&str, f64>) -> f64 {
        match self {
            Self::Number(value) => *value,
            Self::Fact(name) => *values
                .get(name.as_str())
                .expect("all operands bound before evaluation"),
            Self::Unary(Unary::Not, inner) => truth(inner.evaluate(values) == 0.0),
            Self::Unary(Unary::Positive, inner) => inner.evaluate(values),
            Self::Unary(Unary::Negative, inner) => -inner.evaluate(values),
            Self::Binary(Operator::And, left, right) => {
                truth(left.evaluate(values) != 0.0 && right.evaluate(values) != 0.0)
            }
            Self::Binary(Operator::Or, left, right) => {
                truth(left.evaluate(values) != 0.0 || right.evaluate(values) != 0.0)
            }
            Self::Binary(operator, left, right) => {
                operator.apply(left.evaluate(values), right.evaluate(values))
            }
        }
    }
}

#[derive(Clone, Debug)]
enum Token {
    Number(f64),
    Fact(String),
    Binary(Operator),
    Not,
    Open,
    Close,
    End,
}

const OPERATORS: &[(&str, Token)] = &[
    ("%%", Token::Binary(Operator::Modulo)),
    ("!=", Token::Binary(Operator::NotEqual)),
    ("==", Token::Binary(Operator::Equal)),
    ("<=", Token::Binary(Operator::LessEqual)),
    (">=", Token::Binary(Operator::GreaterEqual)),
    ("<?", Token::Binary(Operator::Maximum)),
    (">?", Token::Binary(Operator::Minimum)),
    ("&&", Token::Binary(Operator::And)),
    ("||", Token::Binary(Operator::Or)),
    ("%", Token::Binary(Operator::Divide)),
    ("*", Token::Binary(Operator::Multiply)),
    ("+", Token::Binary(Operator::Add)),
    ("-", Token::Binary(Operator::Subtract)),
    ("=", Token::Binary(Operator::Equal)),
    ("<", Token::Binary(Operator::Less)),
    (">", Token::Binary(Operator::Greater)),
    ("&", Token::Binary(Operator::And)),
    ("|", Token::Binary(Operator::Or)),
    ("!", Token::Not),
];

fn scan_number(source: &str) -> Result<(Token, usize), String> {
    let mut length = source.bytes().take_while(u8::is_ascii_digit).count();
    if source.as_bytes().get(length) == Some(&b'.') {
        length += 1;
        length += source[length..]
            .bytes()
            .take_while(u8::is_ascii_digit)
            .count();
    }
    let value = source[..length]
        .parse::<f64>()
        .map_err(|error| error.to_string())?;
    Ok((Token::Number(value), length))
}

fn scan_fact(source: &str) -> Result<(Token, usize), String> {
    let length = source
        .bytes()
        .take_while(|byte| byte.is_ascii_alphanumeric() || matches!(*byte, b'_' | b'.'))
        .count();
    let name = &source[..length];
    if name.split('.').any(str::is_empty) {
        return Err(format!("empty operand path segment in {name:?}"));
    }
    Ok((Token::Fact(name.to_owned()), length))
}

fn scan_token(source: &str) -> Result<(Token, usize), String> {
    match source.as_bytes()[0] {
        b'(' => Ok((Token::Open, 1)),
        b')' => Ok((Token::Close, 1)),
        b'0'..=b'9' => scan_number(source),
        b'a'..=b'z' | b'A'..=b'Z' => scan_fact(source),
        _ => OPERATORS
            .iter()
            .find(|(literal, _)| source.starts_with(*literal))
            .map(|(literal, token)| (token.clone(), literal.len()))
            .ok_or_else(|| format!("unsupported syntax at {source:?}")),
    }
}

fn tokenize(mut source: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    while !source.is_empty() {
        let (token, consumed) = scan_token(source)?;
        tokens.push(token);
        source = &source[consumed..];
    }
    tokens.push(Token::End);
    Ok(tokens)
}

struct Parser {
    tokens: Vec<Token>,
    cursor: usize,
}

impl Parser {
    fn next(&mut self) -> Token {
        let token = self.tokens[self.cursor].clone();
        self.cursor += 1;
        token
    }

    fn expression(&mut self, minimum: u8) -> Result<Node, String> {
        let mut left = self.prefix()?;
        while let Token::Binary(operator) = self.tokens[self.cursor] {
            let precedence = operator.precedence();
            if precedence < minimum {
                break;
            }
            self.cursor += 1;
            let right = self.expression(precedence + 1)?;
            left = Node::Binary(operator, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn prefix(&mut self) -> Result<Node, String> {
        match self.next() {
            Token::Number(value) => Ok(Node::Number(value)),
            Token::Fact(name) => Ok(Node::Fact(name)),
            Token::Not => self.unary(Unary::Not),
            Token::Binary(Operator::Add) => self.unary(Unary::Positive),
            Token::Binary(Operator::Subtract) => self.unary(Unary::Negative),
            Token::Open => {
                let inner = self.expression(0)?;
                if !matches!(self.next(), Token::Close) {
                    return Err("expected closing parenthesis".into());
                }
                Ok(inner)
            }
            token => Err(format!(
                "expected number, operand or unary expression, found {token:?}"
            )),
        }
    }

    fn unary(&mut self, operator: Unary) -> Result<Node, String> {
        Ok(Node::Unary(operator, Box::new(self.expression(8)?)))
    }
}

pub(super) fn parse(source: &str) -> Result<Node, String> {
    let mut parser = Parser {
        tokens: tokenize(source)?,
        cursor: 0,
    };
    let root = parser.expression(0)?;
    if !matches!(parser.tokens[parser.cursor], Token::End) {
        return Err("unexpected trailing token".into());
    }
    Ok(root)
}
