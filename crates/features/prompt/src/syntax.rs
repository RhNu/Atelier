use logos::Logos;
use rowan::{GreenNode, GreenNodeBuilder, Language, SyntaxKind as RowanSyntaxKind};

use crate::ast::PromptAst;
use crate::diagnostics::PromptDiagnostic;
use crate::dialect::PromptSyntaxProfile;
use crate::formatter::FormatterOptions;
use crate::functions::FunctionRegistry;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct PromptSpan {
    pub start: usize,
    pub end: usize,
}

impl PromptSpan {
    #[must_use]
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum PromptTokenKind {
    Whitespace,
    Text,
    Identifier,
    Number,
    InvalidNumber,
    String,
    UnterminatedString,
    Escaped,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    LParen,
    RParen,
    Comma,
    Pipe,
    DoublePipe,
    Colon,
    DoubleColon,
    At,
    Equals,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PromptToken {
    pub kind: PromptTokenKind,
    pub span: PromptSpan,
    pub text: String,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PromptLanguage {}

impl Language for PromptLanguage {
    type Kind = PromptSyntaxKind;

    fn kind_from_raw(raw: RowanSyntaxKind) -> Self::Kind {
        PromptSyntaxKind::from_raw(raw.0)
    }

    fn kind_to_raw(kind: Self::Kind) -> RowanSyntaxKind {
        RowanSyntaxKind(kind as u16)
    }
}

pub type PromptSyntaxNode = rowan::SyntaxNode<PromptLanguage>;

#[repr(u16)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PromptSyntaxKind {
    Root,
    Strengthening,
    Weakening,
    NumericEmphasis,
    Randomizer,
    ExtensionCall,
    Whitespace,
    Text,
    Identifier,
    Number,
    InvalidNumber,
    String,
    UnterminatedString,
    Escaped,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    LParen,
    RParen,
    Comma,
    Pipe,
    DoublePipe,
    Colon,
    DoubleColon,
    At,
    Equals,
    Error,
}

impl PromptSyntaxKind {
    const fn from_raw(raw: u16) -> Self {
        match raw {
            0 => Self::Root,
            1 => Self::Strengthening,
            2 => Self::Weakening,
            3 => Self::NumericEmphasis,
            4 => Self::Randomizer,
            5 => Self::ExtensionCall,
            6 => Self::Whitespace,
            7 => Self::Text,
            8 => Self::Identifier,
            9 => Self::Number,
            10 => Self::InvalidNumber,
            11 => Self::String,
            12 => Self::UnterminatedString,
            13 => Self::Escaped,
            14 => Self::LBrace,
            15 => Self::RBrace,
            16 => Self::LBracket,
            17 => Self::RBracket,
            18 => Self::LParen,
            19 => Self::RParen,
            20 => Self::Comma,
            21 => Self::Pipe,
            22 => Self::DoublePipe,
            23 => Self::Colon,
            24 => Self::DoubleColon,
            25 => Self::At,
            26 => Self::Equals,
            _ => Self::Error,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ParsedPrompt {
    source: String,
    tokens: Vec<PromptToken>,
    green: GreenNode,
}

impl ParsedPrompt {
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    #[must_use]
    pub fn tokens(&self) -> &[PromptToken] {
        &self.tokens
    }

    #[must_use]
    pub fn syntax(&self) -> PromptSyntaxNode {
        PromptSyntaxNode::new_root(self.green.clone())
    }

    #[must_use]
    pub fn ast(&self) -> PromptAst {
        PromptAst::from_tokens(&self.tokens)
    }

    #[must_use]
    pub fn to_lossless_text(&self) -> String {
        self.tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect()
    }

    #[must_use]
    pub fn diagnostics(&self, profile: &PromptSyntaxProfile) -> Vec<PromptDiagnostic> {
        self.diagnostics_with_functions(profile, &FunctionRegistry::empty())
    }

    #[must_use]
    pub fn diagnostics_with_functions(
        &self,
        profile: &PromptSyntaxProfile,
        functions: &FunctionRegistry,
    ) -> Vec<PromptDiagnostic> {
        crate::diagnostics::diagnose(self, profile, functions)
    }

    #[must_use]
    pub fn format(&self, options: &FormatterOptions) -> String {
        crate::formatter::format_parsed_prompt(self, *options)
    }
}

#[must_use]
pub fn parse_prompt(source: &str) -> ParsedPrompt {
    let tokens = lex_prompt(source);
    let green = build_green_tree(&tokens);
    ParsedPrompt {
        source: source.to_owned(),
        tokens,
        green,
    }
}

#[derive(Logos, Copy, Clone, Debug, PartialEq, Eq)]
enum LexToken {
    #[regex(r"[ \t\r\n]+")]
    Whitespace,
    #[token("||")]
    DoublePipe,
    #[token("::")]
    DoubleColon,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token(",")]
    Comma,
    #[token("|")]
    Pipe,
    #[token(":")]
    Colon,
    #[token("@")]
    At,
    #[token("=")]
    Equals,
    #[regex(r#""([^"\\]|\\.)*""#)]
    String,
    #[regex(r#""([^"\\]|\\.)*"#)]
    UnterminatedString,
    #[regex(r"-?[0-9]+(\.[0-9]*){2,}", priority = 4)]
    InvalidNumber,
    #[regex(r"-?[0-9]+(\.[0-9]+)?", priority = 3)]
    Number,
    #[regex(r"[\p{XID_Start}_][\p{XID_Continue}_-]*", priority = 3)]
    Identifier,
    #[regex(r#"\\."#)]
    Escaped,
    #[regex(r#"[^{}\[\](),|@=:\s"]+"#, priority = 1)]
    Text,
}

fn lex_prompt(source: &str) -> Vec<PromptToken> {
    let mut lexer = LexToken::lexer(source);
    let mut tokens = Vec::new();
    while let Some(result) = lexer.next() {
        let span = lexer.span();
        let kind = result.map_or(PromptTokenKind::Error, token_kind);
        tokens.push(PromptToken {
            kind,
            span: PromptSpan::new(span.start, span.end),
            text: source[span].to_owned(),
        });
    }
    tokens
}

const fn token_kind(token: LexToken) -> PromptTokenKind {
    match token {
        LexToken::Whitespace => PromptTokenKind::Whitespace,
        LexToken::DoublePipe => PromptTokenKind::DoublePipe,
        LexToken::DoubleColon => PromptTokenKind::DoubleColon,
        LexToken::LBrace => PromptTokenKind::LBrace,
        LexToken::RBrace => PromptTokenKind::RBrace,
        LexToken::LBracket => PromptTokenKind::LBracket,
        LexToken::RBracket => PromptTokenKind::RBracket,
        LexToken::LParen => PromptTokenKind::LParen,
        LexToken::RParen => PromptTokenKind::RParen,
        LexToken::Comma => PromptTokenKind::Comma,
        LexToken::Pipe => PromptTokenKind::Pipe,
        LexToken::Colon => PromptTokenKind::Colon,
        LexToken::At => PromptTokenKind::At,
        LexToken::Equals => PromptTokenKind::Equals,
        LexToken::String => PromptTokenKind::String,
        LexToken::UnterminatedString => PromptTokenKind::UnterminatedString,
        LexToken::InvalidNumber => PromptTokenKind::InvalidNumber,
        LexToken::Number => PromptTokenKind::Number,
        LexToken::Identifier => PromptTokenKind::Identifier,
        LexToken::Escaped => PromptTokenKind::Escaped,
        LexToken::Text => PromptTokenKind::Text,
    }
}

fn build_green_tree(tokens: &[PromptToken]) -> GreenNode {
    let mut builder = GreenNodeBuilder::new();
    builder.start_node(PromptSyntaxKind::Root.into());
    let mut parser = CstParser {
        tokens,
        index: 0,
        builder,
    };
    parser.parse_until(None);
    parser.builder.finish_node();
    parser.builder.finish()
}

impl From<PromptSyntaxKind> for RowanSyntaxKind {
    fn from(value: PromptSyntaxKind) -> Self {
        PromptLanguage::kind_to_raw(value)
    }
}

struct CstParser<'a> {
    tokens: &'a [PromptToken],
    index: usize,
    builder: GreenNodeBuilder<'static>,
}

impl CstParser<'_> {
    fn parse_until(&mut self, end: Option<PromptTokenKind>) {
        while !self.is_at_end() {
            if end.is_some_and(|kind| self.at(kind)) {
                return;
            }
            self.parse_item();
        }
    }

    fn parse_item(&mut self) {
        if self.starts_numeric_emphasis() {
            self.parse_numeric_emphasis();
            return;
        }
        if self.starts_extension_call() {
            self.parse_extension_call();
            return;
        }
        match self.current_kind() {
            Some(PromptTokenKind::LBrace) => {
                self.parse_wrapped(PromptSyntaxKind::Strengthening, PromptTokenKind::RBrace);
            }
            Some(PromptTokenKind::LBracket) => {
                self.parse_wrapped(PromptSyntaxKind::Weakening, PromptTokenKind::RBracket);
            }
            Some(PromptTokenKind::DoublePipe) => {
                self.parse_wrapped(PromptSyntaxKind::Randomizer, PromptTokenKind::DoublePipe);
            }
            Some(_) => self.bump(),
            None => {}
        }
    }

    fn parse_wrapped(&mut self, node: PromptSyntaxKind, close: PromptTokenKind) {
        self.builder.start_node(node.into());
        self.bump();
        self.parse_until(Some(close));
        if self.at(close) {
            self.bump();
        }
        self.builder.finish_node();
    }

    fn parse_extension_call(&mut self) {
        self.builder
            .start_node(PromptSyntaxKind::ExtensionCall.into());
        self.bump();
        self.bump();
        self.bump();
        while !self.is_at_end() && !self.at(PromptTokenKind::RParen) {
            self.bump();
        }
        if self.at(PromptTokenKind::RParen) {
            self.bump();
        }
        self.builder.finish_node();
    }

    fn parse_numeric_emphasis(&mut self) {
        self.builder
            .start_node(PromptSyntaxKind::NumericEmphasis.into());
        self.bump();
        if self.at(PromptTokenKind::DoubleColon) {
            self.bump();
        }
        self.parse_until(Some(PromptTokenKind::DoubleColon));
        if self.at(PromptTokenKind::DoubleColon) {
            self.bump();
        }
        self.builder.finish_node();
    }

    fn starts_numeric_emphasis(&self) -> bool {
        matches!(
            self.current_kind(),
            Some(PromptTokenKind::Number | PromptTokenKind::InvalidNumber)
        ) && self.peek_kind(1) == Some(PromptTokenKind::DoubleColon)
    }

    fn starts_extension_call(&self) -> bool {
        self.current_kind() == Some(PromptTokenKind::At)
            && self.peek_kind(1) == Some(PromptTokenKind::Identifier)
            && self.peek_kind(2) == Some(PromptTokenKind::LParen)
    }

    fn bump(&mut self) {
        if let Some(token) = self.tokens.get(self.index) {
            self.builder
                .token(token_syntax_kind(token.kind).into(), token.text.as_str());
            self.index += 1;
        }
    }

    fn at(&self, kind: PromptTokenKind) -> bool {
        self.current_kind() == Some(kind)
    }

    fn current_kind(&self) -> Option<PromptTokenKind> {
        self.peek_kind(0)
    }

    fn peek_kind(&self, offset: usize) -> Option<PromptTokenKind> {
        self.tokens.get(self.index + offset).map(|token| token.kind)
    }

    const fn is_at_end(&self) -> bool {
        self.index >= self.tokens.len()
    }
}

const fn token_syntax_kind(kind: PromptTokenKind) -> PromptSyntaxKind {
    match kind {
        PromptTokenKind::Whitespace => PromptSyntaxKind::Whitespace,
        PromptTokenKind::Text => PromptSyntaxKind::Text,
        PromptTokenKind::Identifier => PromptSyntaxKind::Identifier,
        PromptTokenKind::Number => PromptSyntaxKind::Number,
        PromptTokenKind::InvalidNumber => PromptSyntaxKind::InvalidNumber,
        PromptTokenKind::String => PromptSyntaxKind::String,
        PromptTokenKind::UnterminatedString => PromptSyntaxKind::UnterminatedString,
        PromptTokenKind::Escaped => PromptSyntaxKind::Escaped,
        PromptTokenKind::LBrace => PromptSyntaxKind::LBrace,
        PromptTokenKind::RBrace => PromptSyntaxKind::RBrace,
        PromptTokenKind::LBracket => PromptSyntaxKind::LBracket,
        PromptTokenKind::RBracket => PromptSyntaxKind::RBracket,
        PromptTokenKind::LParen => PromptSyntaxKind::LParen,
        PromptTokenKind::RParen => PromptSyntaxKind::RParen,
        PromptTokenKind::Comma => PromptSyntaxKind::Comma,
        PromptTokenKind::Pipe => PromptSyntaxKind::Pipe,
        PromptTokenKind::DoublePipe => PromptSyntaxKind::DoublePipe,
        PromptTokenKind::Colon => PromptSyntaxKind::Colon,
        PromptTokenKind::DoubleColon => PromptSyntaxKind::DoubleColon,
        PromptTokenKind::At => PromptSyntaxKind::At,
        PromptTokenKind::Equals => PromptSyntaxKind::Equals,
        PromptTokenKind::Error => PromptSyntaxKind::Error,
    }
}
