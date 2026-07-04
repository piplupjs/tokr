use smol_str::SmolStr;
use tokr_diagnostics::{Diagnostic, DiagnosticBag};
use tokr_span::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    ThemeComment(String),
    Dollar,
    Ident(SmolStr),
    Dot,
    LBracket,
    RBracket,
    Number(u32),
    Colon,
    Semicolon,
    LParen,
    RParen,
    VarKw,
    CssCustomProp(SmolStr),
    AnyChar(char),
    Eof,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

pub struct Lexer<'a> {
    src: &'a str,
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str) -> Self {
        Self {
            src,
            bytes: src.as_bytes(),
            pos: 0,
        }
    }

    pub fn tokenize(mut self) -> (Vec<Token>, Vec<Diagnostic>) {
        let mut out = Vec::new();
        let mut diags = DiagnosticBag::default();
        loop {
            self.skip_whitespace();
            if self.at_eof() {
                out.push(self.eof_token());
                break;
            }
            if let Some(tok) = self.scan_one(&mut diags) {
                out.push(tok);
            }
        }
        (out, diags.into_vec())
    }

    fn skip_whitespace(&mut self) {
        while !self.at_eof() && self.bytes[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }

    fn at_eof(&self) -> bool {
        self.pos >= self.bytes.len()
    }

    fn eof_token(&self) -> Token {
        Token {
            kind: TokenKind::Eof,
            span: Span::new(self.pos as u32, self.pos as u32),
        }
    }

    fn scan_one(&mut self, diags: &mut DiagnosticBag) -> Option<Token> {
        let start = self.pos;
        let c = self.bytes[self.pos];

        if c == b'/' {
            if self.pos + 1 < self.bytes.len() {
                let next = self.bytes[self.pos + 1];
                if next == b'/' {
                    self.skip_line_comment();
                    return None;
                } else if next == b'*' {
                    return self.scan_block_comment(diags);
                }
            }
        }

        self.pos += 1;

        match c {
            b'$' => Some(Token {
                kind: TokenKind::Dollar,
                span: Span::new(start as u32, self.pos as u32),
            }),
            b'.' => Some(Token {
                kind: TokenKind::Dot,
                span: Span::new(start as u32, self.pos as u32),
            }),
            b'[' => Some(Token {
                kind: TokenKind::LBracket,
                span: Span::new(start as u32, self.pos as u32),
            }),
            b']' => Some(Token {
                kind: TokenKind::RBracket,
                span: Span::new(start as u32, self.pos as u32),
            }),
            b':' => Some(Token {
                kind: TokenKind::Colon,
                span: Span::new(start as u32, self.pos as u32),
            }),
            b';' => Some(Token {
                kind: TokenKind::Semicolon,
                span: Span::new(start as u32, self.pos as u32),
            }),
            b'(' => Some(Token {
                kind: TokenKind::LParen,
                span: Span::new(start as u32, self.pos as u32),
            }),
            b')' => Some(Token {
                kind: TokenKind::RParen,
                span: Span::new(start as u32, self.pos as u32),
            }),
            b'0'..=b'9' => {
                let mut num = (c - b'0') as u32;
                while !self.at_eof() && self.bytes[self.pos].is_ascii_digit() {
                    num = num * 10 + (self.bytes[self.pos] - b'0') as u32;
                    self.pos += 1;
                }
                Some(Token {
                    kind: TokenKind::Number(num),
                    span: Span::new(start as u32, self.pos as u32),
                })
            }
            b'-' if self.pos < self.bytes.len() && self.bytes[self.pos] == b'-' => {
                self.pos += 1;
                while !self.at_eof()
                    && (self.bytes[self.pos].is_ascii_alphanumeric()
                        || self.bytes[self.pos] == b'-'
                        || self.bytes[self.pos] == b'_')
                {
                    self.pos += 1;
                }
                let text = &self.src[start..self.pos];
                Some(Token {
                    kind: TokenKind::CssCustomProp(SmolStr::new(text)),
                    span: Span::new(start as u32, self.pos as u32),
                })
            }
            _ if c.is_ascii_alphabetic() || c == b'_' => {
                while !self.at_eof()
                    && (self.bytes[self.pos].is_ascii_alphanumeric()
                        || self.bytes[self.pos] == b'_'
                        || self.bytes[self.pos] == b'-')
                {
                    self.pos += 1;
                }
                let text = &self.src[start..self.pos];
                let kind = if text == "var" {
                    TokenKind::VarKw
                } else {
                    TokenKind::Ident(SmolStr::new(text))
                };
                Some(Token {
                    kind,
                    span: Span::new(start as u32, self.pos as u32),
                })
            }
            _ => Some(Token {
                kind: TokenKind::AnyChar(c as char),
                span: Span::new(start as u32, self.pos as u32),
            }),
        }
    }

    fn skip_line_comment(&mut self) {
        while !self.at_eof() && self.bytes[self.pos] != b'\n' {
            self.pos += 1;
        }
    }

    fn scan_block_comment(&mut self, diags: &mut DiagnosticBag) -> Option<Token> {
        let start = self.pos;
        self.pos += 2; // consume /*

        let mut inner_start = self.pos;
        while inner_start < self.bytes.len() && self.bytes[inner_start].is_ascii_whitespace() {
            inner_start += 1;
        }

        let mut is_theme = false;
        if inner_start + 6 <= self.bytes.len()
            && &self.src[inner_start..inner_start + 6] == "@theme"
        {
            is_theme = true;
            inner_start += 6;
            while inner_start < self.bytes.len() && self.bytes[inner_start].is_ascii_whitespace() {
                inner_start += 1;
            }
        }

        let mut found_close = false;
        let mut inner_end = self.pos;

        while self.pos < self.bytes.len() {
            if self.bytes[self.pos] == b'*' {
                let mut star_count = 1;
                while self.pos + star_count < self.bytes.len()
                    && self.bytes[self.pos + star_count] == b'*'
                {
                    star_count += 1;
                }
                if self.pos + star_count < self.bytes.len()
                    && self.bytes[self.pos + star_count] == b'/'
                {
                    inner_end = self.pos;
                    self.pos += star_count + 1;
                    found_close = true;
                    break;
                }
            }
            self.pos += 1;
        }

        if !found_close {
            diags.error(
                "TC0002",
                "unclosed block comment",
                Span::new(start as u32, self.pos as u32),
            );
            return None;
        }

        if is_theme {
            while inner_end > inner_start && self.bytes[inner_end - 1].is_ascii_whitespace() {
                inner_end -= 1;
            }
            let raw_path = self.src[inner_start..inner_end].to_string();
            Some(Token {
                kind: TokenKind::ThemeComment(raw_path),
                span: Span::new(start as u32, self.pos as u32),
            })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normal_theme_comment() {
        let (tokens, diags) = Lexer::new("/* @theme palette.accent */").tokenize();
        assert!(diags.is_empty());
        assert_eq!(
            tokens[0].kind,
            TokenKind::ThemeComment("palette.accent".to_string())
        );
    }

    #[test]
    fn test_keep_sorted_exclusion() {
        let (tokens, diags) = Lexer::new("/* keep-sorted */").tokenize();
        assert!(diags.is_empty());
        assert_eq!(tokens.len(), 1); // just Eof
        assert_eq!(tokens[0].kind, TokenKind::Eof);
    }

    #[test]
    fn test_star_plus_slash_tolerance() {
        let (tokens, diags) = Lexer::new("/* @theme palette.accent **/").tokenize();
        assert!(diags.is_empty());
        assert_eq!(
            tokens[0].kind,
            TokenKind::ThemeComment("palette.accent".to_string())
        );
    }

    #[test]
    fn test_variables_and_properties() {
        let (tokens, diags) =
            Lexer::new("$accent-foreground: var(--accent-foreground);").tokenize();
        assert!(diags.is_empty());
        assert_eq!(tokens[0].kind, TokenKind::Dollar);
        assert_eq!(
            tokens[1].kind,
            TokenKind::Ident(SmolStr::new("accent-foreground"))
        );
        assert_eq!(tokens[2].kind, TokenKind::Colon);
        assert_eq!(tokens[3].kind, TokenKind::VarKw);
        assert_eq!(tokens[4].kind, TokenKind::LParen);
        assert_eq!(
            tokens[5].kind,
            TokenKind::CssCustomProp(SmolStr::new("--accent-foreground"))
        );
        assert_eq!(tokens[6].kind, TokenKind::RParen);
        assert_eq!(tokens[7].kind, TokenKind::Semicolon);
    }

    #[test]
    fn test_css_variables() {
        let (tokens, diags) = Lexer::new("/* @theme a */ --var: 1;").tokenize();
        assert!(diags.is_empty());
        assert_eq!(tokens[0].kind, TokenKind::ThemeComment("a".to_string()));
        assert_eq!(
            tokens[1].kind,
            TokenKind::CssCustomProp(SmolStr::new("--var"))
        );
        assert_eq!(tokens[2].kind, TokenKind::Colon);
        assert_eq!(tokens[3].kind, TokenKind::Number(1));
        assert_eq!(tokens[4].kind, TokenKind::Semicolon);
    }
}
