use smol_str::SmolStr;
use tokr_ast as ast;
use tokr_diagnostics::DiagnosticBag;
use tokr_lexer::{Token, TokenKind};
use tokr_span::Span;

pub struct Parser<'t> {
    src: &'t str,
    toks: &'t [Token],
    pos: usize,
    diags: &'t mut DiagnosticBag,
}

impl<'t> Parser<'t> {
    pub fn new(src: &'t str, toks: &'t [Token], diags: &'t mut DiagnosticBag) -> Self {
        Self {
            src,
            toks,
            pos: 0,
            diags,
        }
    }

    pub fn parse_file(mut self) -> ast::File {
        let mut decls = Vec::new();
        while !self.at_eof() {
            if self.check(|k| matches!(k, TokenKind::ThemeComment(_))) {
                match self.parse_theme_decl() {
                    Ok(decl) => decls.push(decl),
                    Err(_) => self.synchronize(),
                }
            } else {
                self.synchronize();
            }
        }
        ast::File { decls }
    }

    fn check(&self, pred: impl Fn(&TokenKind) -> bool) -> bool {
        if self.at_eof() {
            false
        } else {
            pred(&self.toks[self.pos].kind)
        }
    }

    fn peek(&self) -> &TokenKind {
        if self.at_eof() {
            &TokenKind::Eof
        } else {
            &self.toks[self.pos].kind
        }
    }

    fn advance(&mut self) -> &Token {
        if self.at_eof() {
            &self.toks[self.toks.len() - 1]
        } else {
            let tok = &self.toks[self.pos];
            self.pos += 1;
            tok
        }
    }

    fn cur_span(&self) -> Span {
        if self.at_eof() {
            self.toks
                .last()
                .map(|t| t.span)
                .unwrap_or_else(|| Span::new(0, 0))
        } else {
            self.toks[self.pos].span
        }
    }

    fn expect(&mut self, kind: TokenKind) -> Result<&Token, ()> {
        if self.check(|k| k == &kind) {
            Ok(self.advance())
        } else {
            let span = self.cur_span();
            self.diags
                .error("TC0102", format!("expected {:?}", kind), span);
            Err(())
        }
    }

    fn parse_theme_decl(&mut self) -> Result<ast::ThemeDecl, ()> {
        let comment_tok = self.advance().clone();
        let raw_path = match &comment_tok.kind {
            TokenKind::ThemeComment(s) => s.clone(),
            _ => return Err(()), // unreachable
        };
        let path = self.parse_path_from_comment_text(&raw_path, comment_tok.span)?;

        let (var_name, is_sass_var, value, decl_span) = self.parse_var_decl()?;
        Ok(ast::ThemeDecl {
            path,
            path_span: comment_tok.span,
            var_name,
            is_sass_var,
            value,
            span: comment_tok.span.merge(decl_span),
        })
    }

    fn parse_path_from_comment_text(&mut self, text: &str, span: Span) -> Result<ast::Path, ()> {
        let mut path = Vec::new();
        let mut chars = text.chars().peekable();

        let skip_ws = |chars: &mut std::iter::Peekable<std::str::Chars>| {
            while let Some(&c) = chars.peek() {
                if c.is_ascii_whitespace() {
                    chars.next();
                } else {
                    break;
                }
            }
        };

        skip_ws(&mut chars);

        if chars.peek().is_none() {
            self.diags
                .error("TC0103", "empty path in @theme annotation", span);
            return Err(());
        }

        loop {
            skip_ws(&mut chars);
            let mut ident = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_alphanumeric() || c == '_' || c == '$' || c == '-' {
                    ident.push(c);
                    chars.next();
                } else {
                    break;
                }
            }
            if ident.is_empty() {
                self.diags.error("TC0104", "expected path segment", span);
                return Err(());
            }
            path.push(ast::PathSegment::Field(SmolStr::new(ident)));

            skip_ws(&mut chars);

            while let Some(&c) = chars.peek() {
                if c == '[' {
                    chars.next(); // consume '['
                    skip_ws(&mut chars);
                    let mut num = String::new();
                    while let Some(&n) = chars.peek() {
                        if n.is_ascii_digit() {
                            num.push(n);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    if num.is_empty() {
                        self.diags
                            .error("TC0105", "expected number in index suffix", span);
                        return Err(());
                    }
                    skip_ws(&mut chars);
                    if chars.next() != Some(']') {
                        self.diags.error("TC0106", "expected ']' after index", span);
                        return Err(());
                    }
                    path.push(ast::PathSegment::Index(num.parse().unwrap()));
                    skip_ws(&mut chars);
                } else {
                    break;
                }
            }

            if chars.peek() == Some(&'.') {
                chars.next(); // consume '.'
            } else {
                break;
            }
        }

        if chars.peek().is_some() {
            self.diags
                .error("TC0107", "trailing characters in path", span);
            return Err(());
        }

        Ok(path)
    }

    fn parse_var_decl(&mut self) -> Result<(SmolStr, bool, ast::Value, Span), ()> {
        let (name, is_sass_var) = match self.peek() {
            TokenKind::Dollar => {
                self.advance();
                match self.peek() {
                    TokenKind::Ident(name) => {
                        let name = name.clone();
                        self.advance();
                        (name, true)
                    }
                    _ => {
                        self.diags.error(
                            "TC0108",
                            "expected identifier after '$'",
                            self.cur_span(),
                        );
                        return Err(());
                    }
                }
            }
            TokenKind::CssCustomProp(name) => {
                let name = name.clone();
                self.advance();
                let stripped_name = SmolStr::new(&name.as_str()[2..]);
                (stripped_name, false)
            }
            _ => {
                self.diags.error(
                    "TC0108",
                    "expected '$' or '--' for variable declaration",
                    self.cur_span(),
                );
                return Err(());
            }
        };

        self.expect(TokenKind::Colon)?;

        let value = self.parse_value()?;

        let end_span = self.expect(TokenKind::Semicolon)?.span;
        Ok((name, is_sass_var, value, end_span))
    }

    fn parse_value(&mut self) -> Result<ast::Value, ()> {
        let start_span = self.cur_span();
        match self.peek() {
            TokenKind::VarKw => {
                self.advance();
                self.expect(TokenKind::LParen)?;
                let css_var = match self.peek() {
                    TokenKind::CssCustomProp(p) => {
                        let p = p.clone();
                        self.advance();
                        p
                    }
                    _ => {
                        self.diags.error(
                            "TC0109",
                            "expected css custom property in var()",
                            self.cur_span(),
                        );
                        return Err(());
                    }
                };
                let end_span = self.expect(TokenKind::RParen)?.span;
                Ok(ast::Value::VarRef {
                    css_var,
                    span: start_span.merge(end_span),
                })
            }
            _ => {
                let start_idx = start_span.lo as usize;
                let mut end_span = start_span;
                let mut has_tokens = false;

                while !self.at_eof() {
                    if self.check(|k| {
                        matches!(
                            k,
                            TokenKind::Semicolon | TokenKind::ThemeComment(_) | TokenKind::Dollar
                        )
                    }) {
                        break;
                    }
                    end_span = self.advance().span;
                    has_tokens = true;
                }

                if !has_tokens {
                    self.diags
                        .error("TC0110", "expected value", self.cur_span());
                    return Err(());
                }

                let end_idx = end_span.hi as usize;
                let raw_text = self.src[start_idx..end_idx].trim().to_string();

                Ok(ast::Value::Raw {
                    text: SmolStr::new(raw_text),
                    span: start_span.merge(end_span),
                })
            }
        }
    }

    fn at_eof(&self) -> bool {
        self.pos >= self.toks.len() || self.toks[self.pos].kind == TokenKind::Eof
    }

    fn synchronize(&mut self) {
        if self.at_eof() {
            return;
        }

        self.advance();

        while !self.at_eof() {
            if self.check(|k| matches!(k, TokenKind::ThemeComment(_))) {
                break;
            }
            self.advance();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokr_lexer::Lexer;

    fn parse(src: &str) -> (ast::File, DiagnosticBag) {
        let (tokens, lex_diags) = Lexer::new(src).tokenize();
        let mut bag = DiagnosticBag::default();
        let parser = Parser::new(src, &tokens, &mut bag);
        let file = parser.parse_file();

        let mut diags = DiagnosticBag::default();
        diags.extend(lex_diags);
        diags.extend(bag.into_vec());
        (file, diags)
    }

    #[test]
    fn test_valid_decl() {
        let src = "/* @theme palette.accent */\n$accent: var(--accent);";
        let (file, diags) = parse(src);
        insta::assert_debug_snapshot!(file);
        assert!(!diags.has_errors());
    }

    #[test]
    fn test_valid_decl_array() {
        let src = "/* @theme boxShadow[0] */\n$shadow: var(--shadow);";
        let (file, diags) = parse(src);
        insta::assert_debug_snapshot!(file);
        assert!(!diags.has_errors());
    }

    #[test]
    fn test_recovery_missing_semicolon() {
        let src = "/* @theme a */\n$a: 1\n/* @theme b */\n$b: 2;";
        let (file, diags) = parse(src);
        insta::assert_debug_snapshot!(file);
        assert!(diags.has_errors());
    }

    #[test]
    fn test_stray_token() {
        let src = "$stray: 1; /* @theme a */\n$a: 1;";
        let (file, diags) = parse(src);
        insta::assert_debug_snapshot!(file);
        assert!(!diags.has_errors());
    }

    #[test]
    fn test_sass_var_without_theme() {
        let src = "$color: var(--color);";
        let (file, diags) = parse(src);
        assert!(file.decls.is_empty());
        assert!(!diags.has_errors());
    }

    #[test]
    fn test_css_custom_prop_without_theme() {
        let src = "--spacing: 1rem;";
        let (file, diags) = parse(src);
        assert!(file.decls.is_empty());
        assert!(!diags.has_errors());
    }

    #[test]
    fn test_non_variable_statement_no_warning() {
        let src = "@use \"sass:color\" as color;";
        let (file, diags) = parse(src);
        assert!(file.decls.is_empty());
        assert!(!diags.has_errors());
    }
}
