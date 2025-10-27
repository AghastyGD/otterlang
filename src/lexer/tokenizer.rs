use super::token::{Span, Token, TokenKind};
use crate::utils::errors::{Diagnostic, DiagnosticSeverity};
use thiserror::Error;

#[derive(Debug, Error, Clone)]
pub enum LexerError {
    #[error("tabs are not allowed for indentation (line {line}, column {column})")]
    TabsNotAllowed {
        line: usize,
        column: usize,
        span: Span,
    },
    #[error("indentation mismatch: expected {expected} spaces, found {found} (line {line})")]
    IndentationMismatch {
        line: usize,
        expected: usize,
        found: usize,
        span: Span,
    },
    #[error("unterminated string literal (line {line}, column {column})")]
    UnterminatedString {
        line: usize,
        column: usize,
        span: Span,
    },
    #[error("unexpected character `{ch}` (line {line}, column {column})")]
    UnexpectedCharacter {
        ch: char,
        line: usize,
        column: usize,
        span: Span,
    },
}

impl LexerError {
    pub fn to_diagnostic(&self, source_id: &str) -> Diagnostic {
        match self {
            LexerError::TabsNotAllowed { span, .. } => Diagnostic::new(
                DiagnosticSeverity::Error,
                source_id,
                span.clone(),
                self.to_string(),
            ),
            LexerError::IndentationMismatch { span, .. } => Diagnostic::new(
                DiagnosticSeverity::Error,
                source_id,
                span.clone(),
                self.to_string(),
            ),
            LexerError::UnterminatedString { span, .. } => Diagnostic::new(
                DiagnosticSeverity::Error,
                source_id,
                span.clone(),
                self.to_string(),
            ),
            LexerError::UnexpectedCharacter { span, .. } => Diagnostic::new(
                DiagnosticSeverity::Error,
                source_id,
                span.clone(),
                self.to_string(),
            ),
        }
    }
}

pub type LexResult<T> = Result<T, Vec<LexerError>>;

pub fn tokenize(source: &str) -> LexResult<Vec<Token>> {
    let mut tokens = Vec::new();
    let mut indent_stack = vec![0usize];
    let mut errors = Vec::new();
    let mut offset = 0usize;

    for (line_idx, chunk) in source.split_inclusive('\n').enumerate() {
        let has_newline = chunk.ends_with('\n');
        let line_number = line_idx + 1;
        let line_without_newline = if has_newline {
            &chunk[..chunk.len() - 1]
        } else {
            chunk
        };
        let line_offset = offset;

        let mut idx = 0usize;
        let mut indent_width = 0usize;
        let mut column = 1usize;

        while idx < line_without_newline.len() {
            match line_without_newline.as_bytes()[idx] {
                b' ' => {
                    indent_width += 1;
                    idx += 1;
                    column += 1;
                }
                b'\t' => {
                    let span = Span::new(line_offset + idx, line_offset + idx + 1);
                    errors.push(LexerError::TabsNotAllowed {
                        line: line_number,
                        column,
                        span,
                    });
                    idx += 1;
                    column += 1;
                }
                _ => break,
            }
        }

        let rest = &line_without_newline[idx..];
        let is_blank = rest.trim().is_empty();
        let is_comment = rest.starts_with('#');

        if is_blank || is_comment {
            offset += chunk.len();
            continue;
        }

        let current_indent = indent_width;
        let last_indent = *indent_stack.last().unwrap();

        if current_indent > last_indent {
            indent_stack.push(current_indent);
            let span = Span::new(line_offset + last_indent, line_offset + current_indent);
            tokens.push(Token::new(TokenKind::Indent, span));
        } else if current_indent < last_indent {
            while current_indent < *indent_stack.last().unwrap() {
                let top = indent_stack.pop().unwrap();
                let span = Span::new(line_offset + current_indent, line_offset + top);
                tokens.push(Token::new(TokenKind::Dedent, span));
            }
            if current_indent != *indent_stack.last().unwrap() {
                let span = Span::new(
                    line_offset + current_indent,
                    line_offset + current_indent + 1,
                );
                errors.push(LexerError::IndentationMismatch {
                    line: line_number,
                    expected: *indent_stack.last().unwrap(),
                    found: current_indent,
                    span,
                });
            }
        }

        let mut i = idx;
        while i < line_without_newline.len() {
            let absolute_start = line_offset + i;
            let column_index = i + 1;

            // Handle multi-character operators first
            if i + 1 < line_without_newline.len() && line_without_newline.is_char_boundary(i) && line_without_newline.is_char_boundary(i + 2) {
                let two_chars = &line_without_newline[i..i + 2];
                match two_chars {
                    "==" => {
                        tokens.push(Token::new(
                            TokenKind::EqEq,
                            Span::new(absolute_start, absolute_start + 2),
                        ));
                        i += 2;
                        continue;
                    }
                    "!=" => {
                        tokens.push(Token::new(
                            TokenKind::Neq,
                            Span::new(absolute_start, absolute_start + 2),
                        ));
                        i += 2;
                        continue;
                    }
                    "<=" => {
                        tokens.push(Token::new(
                            TokenKind::LtEq,
                            Span::new(absolute_start, absolute_start + 2),
                        ));
                        i += 2;
                        continue;
                    }
                    ">=" => {
                        tokens.push(Token::new(
                            TokenKind::GtEq,
                            Span::new(absolute_start, absolute_start + 2),
                        ));
                        i += 2;
                        continue;
                    }
                    "->" => {
                        tokens.push(Token::new(
                            TokenKind::Arrow,
                            Span::new(absolute_start, absolute_start + 2),
                        ));
                        i += 2;
                        continue;
                    }
                    ".." => {
                        tokens.push(Token::new(
                            TokenKind::DoubleDot,
                            Span::new(absolute_start, absolute_start + 2),
                        ));
                        i += 2;
                        continue;
                    }
                    "+=" => {
                        tokens.push(Token::new(
                            TokenKind::PlusEq,
                            Span::new(absolute_start, absolute_start + 2),
                        ));
                        i += 2;
                        continue;
                    }
                    "-=" => {
                        tokens.push(Token::new(
                            TokenKind::MinusEq,
                            Span::new(absolute_start, absolute_start + 2),
                        ));
                        i += 2;
                        continue;
                    }
                    "*=" => {
                        tokens.push(Token::new(
                            TokenKind::StarEq,
                            Span::new(absolute_start, absolute_start + 2),
                        ));
                        i += 2;
                        continue;
                    }
                    "/=" => {
                        tokens.push(Token::new(
                            TokenKind::SlashEq,
                            Span::new(absolute_start, absolute_start + 2),
                        ));
                        i += 2;
                        continue;
                    }
                    _ => {}
                }
            }

            match line_without_newline.as_bytes()[i] {
                b' ' | b'\t' => {
                    i += 1;
                }
                b'#' => {
                    break;
                }
                b'(' => {
                    tokens.push(Token::new(
                        TokenKind::LParen,
                        Span::new(absolute_start, absolute_start + 1),
                    ));
                    i += 1;
                }
                b')' => {
                    tokens.push(Token::new(
                        TokenKind::RParen,
                        Span::new(absolute_start, absolute_start + 1),
                    ));
                    i += 1;
                }
                b'{' => {
                    tokens.push(Token::new(
                        TokenKind::LBrace,
                        Span::new(absolute_start, absolute_start + 1),
                    ));
                    i += 1;
                }
                b'}' => {
                    tokens.push(Token::new(
                        TokenKind::RBrace,
                        Span::new(absolute_start, absolute_start + 1),
                    ));
                    i += 1;
                }
                b'[' => {
                    tokens.push(Token::new(
                        TokenKind::LBracket,
                        Span::new(absolute_start, absolute_start + 1),
                    ));
                    i += 1;
                }
                b']' => {
                    tokens.push(Token::new(
                        TokenKind::RBracket,
                        Span::new(absolute_start, absolute_start + 1),
                    ));
                    i += 1;
                }
                b',' => {
                    tokens.push(Token::new(
                        TokenKind::Comma,
                        Span::new(absolute_start, absolute_start + 1),
                    ));
                    i += 1;
                }
                b'.' => {
                    tokens.push(Token::new(
                        TokenKind::Dot,
                        Span::new(absolute_start, absolute_start + 1),
                    ));
                    i += 1;
                }
                b'+' => {
                    tokens.push(Token::new(
                        TokenKind::Plus,
                        Span::new(absolute_start, absolute_start + 1),
                    ));
                    i += 1;
                }
                b'-' => {
                    tokens.push(Token::new(
                        TokenKind::Minus,
                        Span::new(absolute_start, absolute_start + 1),
                    ));
                    i += 1;
                }
                b'*' => {
                    tokens.push(Token::new(
                        TokenKind::Star,
                        Span::new(absolute_start, absolute_start + 1),
                    ));
                    i += 1;
                }
                b'/' => {
                    tokens.push(Token::new(
                        TokenKind::Slash,
                        Span::new(absolute_start, absolute_start + 1),
                    ));
                    i += 1;
                }
                b'%' => {
                    tokens.push(Token::new(
                        TokenKind::Percent,
                        Span::new(absolute_start, absolute_start + 1),
                    ));
                    i += 1;
                }
                b'|' => {
                    tokens.push(Token::new(
                        TokenKind::Pipe,
                        Span::new(absolute_start, absolute_start + 1),
                    ));
                    i += 1;
                }
                b'&' => {
                    tokens.push(Token::new(
                        TokenKind::Amp,
                        Span::new(absolute_start, absolute_start + 1),
                    ));
                    i += 1;
                }
                b'!' => {
                    tokens.push(Token::new(
                        TokenKind::Bang,
                        Span::new(absolute_start, absolute_start + 1),
                    ));
                    i += 1;
                }
                b':' => {
                    tokens.push(Token::new(
                        TokenKind::Colon,
                        Span::new(absolute_start, absolute_start + 1),
                    ));
                    i += 1;
                }
                b'=' => {
                    tokens.push(Token::new(
                        TokenKind::Equals,
                        Span::new(absolute_start, absolute_start + 1),
                    ));
                    i += 1;
                }
                b'<' => {
                    tokens.push(Token::new(
                        TokenKind::Lt,
                        Span::new(absolute_start, absolute_start + 1),
                    ));
                    i += 1;
                }
                b'>' => {
                    tokens.push(Token::new(
                        TokenKind::Gt,
                        Span::new(absolute_start, absolute_start + 1),
                    ));
                    i += 1;
                }
                b'f' => {
                    // Check if this is followed by a quote (f-string)
                    if i + 1 < line_without_newline.len() && line_without_newline.as_bytes()[i + 1] == b'"' {
                        let start = i;
                        i += 2; // Skip f"
                        let mut value = String::new();

                        // Parse until closing "
                        while i < line_without_newline.len() && line_without_newline.as_bytes()[i] != b'"' {
                            value.push(line_without_newline.as_bytes()[i] as char);
                            i += 1;
                        }

                        let span = Span::new(line_offset + start, line_offset + i + 1);
                        tokens.push(Token::new(
                            TokenKind::FString(value),
                            span,
                        ));
                        i += 1;
                    } else {
                        // Regular identifier starting with 'f'
                        let start = i;
                        i += 1;
                        while i < line_without_newline.len()
                            && (line_without_newline.as_bytes()[i].is_ascii_alphanumeric()
                                || line_without_newline.as_bytes()[i] == b'_')
                        {
                            i += 1;
                        }
                        let value = &line_without_newline[start..i];
                        let span = Span::new(line_offset + start, line_offset + i);
                        let kind = match value {
                            "fn" => TokenKind::Fn,
                            "for" => TokenKind::For,
                            "false" => TokenKind::False,
                            _ => TokenKind::Identifier(value.to_string()),
                        };
                        tokens.push(Token::new(kind, span));
                    }
                }
                b'"' => {
                    let start = i;
                    i += 1;
                    let mut value = String::new();

                    // Regular string (f-strings are handled by the 'f' case above)
                    while i < line_without_newline.len() && line_without_newline.as_bytes()[i] != b'"' {
                        value.push(line_without_newline.as_bytes()[i] as char);
                        i += 1;
                    }

                    let span = Span::new(line_offset + start, line_offset + i + 1);
                    tokens.push(Token::new(
                        TokenKind::StringLiteral(value),
                        span,
                    ));
                    i += 1;
                }
                ch if ch.is_ascii_digit() => {
                    let start = i;
                    i += 1;
                    while i < line_without_newline.len()
                        && (line_without_newline.as_bytes()[i].is_ascii_digit()
                            || line_without_newline.as_bytes()[i] == b'_')
                    {
                        i += 1;
                    }
                    // Check for decimal point, but not if it's followed by another dot (range operator)
                    if i < line_without_newline.len() 
                        && line_without_newline.as_bytes()[i] == b'.'
                        && (i + 1 >= line_without_newline.len() || line_without_newline.as_bytes()[i + 1] != b'.')
                    {
                        i += 1;
                        while i < line_without_newline.len()
                            && (line_without_newline.as_bytes()[i].is_ascii_digit()
                                || line_without_newline.as_bytes()[i] == b'_')
                        {
                            i += 1;
                        }
                    }
                    let value = &line_without_newline[start..i];
                    let span = Span::new(line_offset + start, line_offset + i);
                    tokens.push(Token::new(TokenKind::Number(value.to_string()), span));
                }
                ch if ch.is_ascii_alphabetic() || ch == b'_' => {
                    let start = i;
                    i += 1;
                    while i < line_without_newline.len()
                        && (line_without_newline.as_bytes()[i].is_ascii_alphanumeric()
                            || line_without_newline.as_bytes()[i] == b'_')
                    {
                        i += 1;
                    }
                    let value = &line_without_newline[start..i];
                    let span = Span::new(line_offset + start, line_offset + i);
                    let kind = match value {
                        "fn" => TokenKind::Fn,
                        "let" => TokenKind::Let,
                        "return" => TokenKind::Return,
                        "if" => TokenKind::If,
                        "else" => TokenKind::Else,
                        "elif" => TokenKind::Elif,
                        "for" => TokenKind::For,
                        "while" => TokenKind::While,
                        "break" => TokenKind::Break,
                        "continue" => TokenKind::Continue,
                        "in" => TokenKind::In,
                        "use" => TokenKind::Use,
                        "from" => TokenKind::From,
                        "as" => TokenKind::As,
                        "async" => TokenKind::Async,
                        "await" => TokenKind::Await,
                        "spawn" => TokenKind::Spawn,
                        "match" => TokenKind::Match,
                        "case" => TokenKind::Case,
                        "true" => TokenKind::True,
                        "false" => TokenKind::False,
                        "print" => TokenKind::Print,
                        _ => TokenKind::Identifier(value.to_string()),
                    };
                    tokens.push(Token::new(kind, span));
                }
                ch if ch > 127 => {
                    // Unicode identifier (like π, α, Δ)
                    let start = i;
                    i += 1;
                    while i < line_without_newline.len() {
                        let next_ch = line_without_newline.as_bytes()[i];
                        if next_ch.is_ascii_alphanumeric() || next_ch == b'_' || next_ch > 127 {
                            i += 1;
                        } else {
                            break;
                        }
                    }
                    let value = &line_without_newline[start..i];
                    let span = Span::new(line_offset + start, line_offset + i);
                    tokens.push(Token::new(TokenKind::UnicodeIdentifier(value.to_string()), span));
                }
                other => {
                    let span = Span::new(absolute_start, absolute_start + 1);
                    errors.push(LexerError::UnexpectedCharacter {
                        ch: other as char,
                        line: line_number,
                        column: column_index,
                        span,
                    });
                    i += 1;
                }
            }
        }

        let newline_span = Span::new(
            line_offset + line_without_newline.len(),
            line_offset + line_without_newline.len() + 1,
        );
        tokens.push(Token::new(TokenKind::Newline, newline_span));

        offset += chunk.len();
    }

    while indent_stack.len() > 1 {
        indent_stack.pop();
        let span = Span::new(offset, offset);
        tokens.push(Token::new(TokenKind::Dedent, span));
    }

    let eof_span = tokens
        .last()
        .map(|token| token.span)
        .unwrap_or_else(|| Span::new(offset, offset));
    tokens.push(Token::new(TokenKind::Eof, eof_span));

    if errors.is_empty() {
        Ok(tokens)
    } else {
        Err(errors)
    }
}
