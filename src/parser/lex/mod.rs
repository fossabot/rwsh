pub mod sre;

use sre::Token as SREToken;
use crate::util::{BufReadChars,LineReader,ParseError};

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    /// One or more non-newline whitespace characters.
    Space,
    /// The pipe (`|`) character.
    Pipe,
    /// A structural regular expression pipe (`|>`) and its SRE code
    SREPipe(Vec<SREToken>),
    /// A newline.
    Newline,
    /// A sequence of concatenated words.
    ///
    /// The first tuple element is the quote type (`"` or `'`),
    /// or `\0` if none.
    WordString(char, String),
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub pos: (usize, usize),
    pub len: usize,
}

impl PartialEq<Token> for Token {
    fn eq(&self, other: &Token) -> bool {
        self.kind == other.kind
    }
}

/// Transforms text to a sequence of [`Token`s](enum.Token.html).
pub struct Lexer<R: LineReader> {
    input: BufReadChars<R>,
}

impl<R: LineReader> Lexer<R> {
    /// Creates a new lexer based on a `char` iterator,
    /// usually a [`BufReadChars`](../../util/struct.BufReadChars.html).
    pub fn new(input: BufReadChars<R>) -> Lexer<R> {
        Lexer { input }
    }
}

#[macro_export]
macro_rules! tok {
    ($kind:expr, $len:expr, $it:expr) => (
        Token {len: $len, pos: $it.get_pos(), kind: $kind}
    );
}

impl<R: LineReader> Iterator for Lexer<R> {
    type Item = Result<Token, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(&c) = self.input.peek() {
            if is_clear_string_char(c) {
                match read_string('\0', &mut self.input) {
                    Ok(s) => return Some(Ok(tok!(TokenKind::WordString('\0', s), s.len(), self.input))),
                    Err(e) => return Some(Err(e)),
                }
            } else if c == '"' || c == '\'' {
                self.input.next();
                match read_string(c, &mut self.input) {
                    Ok(s) => return Some(Ok(tok!(TokenKind::WordString(c, s), s.len() + 2, self.input))),
                    Err(e) => return Some(Err(e)),
                }
            } else if c == '|' {
                self.input.next();
                /*
                if let Some('>') = self.input.peek() {
                    self.input.next();
                    return Some(Ok(Token::SREPipe));
                }
                */
                return Some(Ok(tok!(TokenKind::Pipe, 1, self.input)));
            } else if c == '\n' {
                self.input.next();
                return Some(Ok(tok!(TokenKind::Newline, 0, self.input)));
            } else if c.is_whitespace() {
                let len = skip_whitespace(&mut self.input);
                return Some(Ok(tok!(TokenKind::Space, len, self.input)));
            } else {
                return Some(Err(self.input.new_error(format!("unexpected character {}", c))));
            }
        }

        None
    }
}

fn skip_whitespace<R: LineReader>(it: &mut BufReadChars<R>) -> usize {
    let mut len: usize = 0;
    while let Some(&c) = it.peek() {
        if !c.is_whitespace() || c == '\n' {
            break;
        }
        len += 1;
        it.next();
    }
    len
}

fn is_special_char(c: char) -> bool {
    c == '|' || c == '\'' || c == '\"' || c == '&'
}

fn is_clear_string_char(c: char) -> bool {
    !(c.is_control() || c.is_whitespace() || is_special_char(c))
}

fn read_string<R: LineReader>(
    quote: char,
    it: &mut BufReadChars<R>,
) -> Result<String, ParseError> {
    let mut s = String::new();
    let mut escaping = false;
    if quote == '\0' {
        while let Some(&c) = it.peek() {
            if escaping {
                s.push(escape(c));
                escaping = false;
            } else if c == '\\' {
                escaping = true;
            } else {
                if !is_clear_string_char(c) {
                    break;
                }
                s.push(c);
            }
            it.next();
        }
    } else {
        let mut closed = false;
        while let Some(&c) = it.peek() {
            if escaping {
                s.push(escape(c));
                escaping = false;
            } else {
                if c == quote {
                    closed = true;
                    it.next();
                    break;
                }
                if c == '\\' {
                    escaping = true;
                } else {
                    s.push(c);
                }
            }
            it.next();
        }
        if !closed {
            return Err(it.new_error(format!("expected {} at the end of string", quote)));
        }
    }
    if escaping {
        Err(it.new_error(format!("expected {} at the end of string", quote)))
    } else {
        Ok(s)
    }
}

fn escape(c: char) -> char {
    match c {
        'n' => '\n',
        't' => '\t',
        'a' => '\x07',
        'b' => '\x08',
        _ => c,
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::common::new_dummy_buf;
    use crate::util::ParseError;
    use super::TokenKind::*;
    use super::Token;

    #[test]
    fn read_string_no_quotes() {
        let s = "hell_o nice \\-meme😀 test";
        let _result = ["hell_o", "nice", "-meme😀", "test"];
        let mut result = _result.iter().peekable();
        let mut buf = new_dummy_buf(s.lines());
        loop {
            let x = super::read_string('\0', &mut buf).unwrap();
            let correct = result.next();
            if correct.is_none() && x != "" {
                panic!("still getting results: {:?}", x);
            } else if x == "" {
                break;
            }
            assert_eq!(x, *(correct.unwrap()));
            buf.next();
        }
        assert_eq!(result.peek(), None);
    }

    #[test]
    fn read_string_quotes() {
        for q in ['\'', '\"'].iter() {
            let s = format!("{0}hell_o{0} {0}nice \\-meme😀 test\\ny{0}", q);
            let _result = ["hell_o", "nice -meme😀 test\ny"];
            let mut result = _result.iter().peekable();
            let mut buf = new_dummy_buf(s.lines());
            loop {
                buf.next();
                if buf.peek().is_none() {
                    break;
                }
                let x = super::read_string(*q, &mut buf).unwrap();
                let correct = result.next();
                if correct.is_none() && x != "" {
                    panic!("still getting results: {:?}", x);
                } else if x == "" {
                    break;
                }
                assert_eq!(x, *(correct.unwrap()));
                buf.next();
            }
            assert_eq!(result.peek(), None);
        }
    }

    #[test]
    fn read_string_error() {
        for q in ['\'', '\"'].iter() {
            let s = format!("{}this is a bad string", q);
            let mut buf = new_dummy_buf(s.lines());
            buf.next();
            let r = super::read_string(*q, &mut buf);
            assert!(r.is_err());
            assert_eq!(
                r.err().unwrap().message,
                format!("expected {} at the end of string", q)
            );
        }
    }

    #[test]
    fn lex() {
        let s = "echo this\\ is\\ a test\". ignore \"'this 'please | cat\nmeow";
        let buf = new_dummy_buf(s.lines());
        macro_rules! tok {
            ($kind:expr) => {
                super::Token{ kind: $kind, len: 0, pos: buf.get_pos() }
            };
        }

        let ok: Vec<Result<Token, ParseError>> = vec![
            Ok(tok!(WordString('\u{0}', "echo".to_owned()))),
            Ok(tok!(Space)),
            Ok(tok!(WordString('\u{0}', "this is a".to_owned()))),
            Ok(tok!(Space)),
            Ok(tok!(WordString('\u{0}', "test".to_owned()))),
            Ok(tok!(WordString('\"', ". ignore ".to_owned()))),
            Ok(tok!(WordString('\'', "this ".to_owned()))),
            Ok(tok!(WordString('\u{0}', "please".to_owned()))),
            Ok(tok!(Space)),
            Ok(tok!(Pipe)),
            Ok(tok!(Space)),
            Ok(tok!(WordString('\u{0}', "cat".to_owned()))),
            Ok(tok!(Newline)),
            Ok(tok!(WordString('\u{0}', "meow".to_owned()))),
            Ok(tok!(Newline)),
        ];
        let l = super::Lexer::new(buf);
        assert_eq!(l.collect::<Vec<_>>(), ok);
    }

    #[test]
    fn lex_err() {
        let s = "long_unimplemented_stuff & | cat";
        let buf = new_dummy_buf(s.lines());
        macro_rules! tok {
            ($kind:expr) => {
                super::Token{ kind: $kind, len: 0, pos: buf.get_pos() }
            };
        }
        let ok: Vec<Result<super::Token, ParseError>> = vec![
            Ok(tok!(WordString('\u{0}', "long_unimplemented_stuff".to_owned()))),
            Ok(tok!(Space)),
            Err(ParseError{
                message: "unexpected character &".to_owned(),
                line: 0,
                col: 0,
            }),
        ];
        let mut l = super::Lexer::new(buf).peekable();
        let mut result: Vec<Result<Token, ParseError>> = Vec::new();
        while let Some(x) = l.peek() {
            result.push(x.clone());
            if let Err(_) = x {
                break;
            }
            l.next();
        }
        assert_eq!(result, ok);
    }
}