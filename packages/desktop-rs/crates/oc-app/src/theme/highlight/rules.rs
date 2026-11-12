//! `TOKEN_RULES` from `packages/ui/src/lib/shiki/zedThemeFromAppTheme.ts`.

use super::names::Token;

/// Which app color a token name paints with.
#[derive(Clone, Copy)]
pub(super) enum Role {
    Base(Token),
    Token(Token),
    Inserted,
    Deleted,
}

/// One entry of `TOKEN_RULES`: a token name, its color role, and its type attributes.
pub(super) struct Rule {
    pub(super) name: Token,
    pub(super) role: Option<Role>,
    pub(super) bold: bool,
    pub(super) italic: bool,
}

impl Rule {
    const fn base(name: Token, key: Token) -> Self {
        Self {
            name,
            role: Some(Role::Base(key)),
            bold: false,
            italic: false,
        }
    }

    const fn token(name: Token, key: Token) -> Self {
        Self {
            name,
            role: Some(Role::Token(key)),
            bold: false,
            italic: false,
        }
    }

    const fn inserted(name: Token) -> Self {
        Self {
            name,
            role: Some(Role::Inserted),
            bold: false,
            italic: false,
        }
    }

    const fn deleted(name: Token) -> Self {
        Self {
            name,
            role: Some(Role::Deleted),
            bold: false,
            italic: false,
        }
    }

    const fn bold(mut self) -> Self {
        self.bold = true;
        self
    }
}

pub(super) const RULES: [Rule; 66] = [
    Rule::token(Token::Attribute, Token::TagAttribute),
    Rule::token(Token::Boolean, Token::Boolean),
    Rule::base(Token::Comment, Token::Comment),
    Rule::token(Token::CommentDotDoc, Token::CommentDoc),
    Rule::token(Token::Constant, Token::Constant),
    Rule::token(Token::ConstantBuiltin, Token::Null),
    Rule::token(Token::Constructor, Token::ClassName),
    Rule::deleted(Token::DiffMinus),
    Rule::inserted(Token::DiffPlus),
    Rule::base(Token::Embedded, Token::Foreground),
    Rule {
        name: Token::Emphasis,
        role: None,
        bold: false,
        italic: true,
    },
    Rule {
        name: Token::EmphasisStrong,
        role: None,
        bold: true,
        italic: false,
    },
    Rule::token(Token::Enum, Token::Enum),
    Rule::base(Token::Function, Token::Function).bold(),
    Rule::token(Token::FunctionDotMethod, Token::Method).bold(),
    Rule::base(Token::Hint, Token::Comment),
    Rule::base(Token::Keyword, Token::Keyword),
    Rule::token(Token::KeywordDotControl, Token::KeywordControl),
    Rule::token(Token::KeywordDeclaration, Token::StorageModifier),
    Rule::token(Token::KeywordDotImport, Token::KeywordImport),
    Rule::base(Token::KeywordJsdoc, Token::Type),
    Rule::token(Token::KeywordDotOperator, Token::KeywordOperator),
    Rule::token(Token::Label, Token::Label),
    Rule::base(Token::LinkText, Token::String),
    Rule::token(Token::LinkUri, Token::Url),
    Rule::token(Token::Namespace, Token::Namespace),
    Rule::base(Token::Number, Token::Number),
    Rule::base(Token::Operator, Token::Operator),
    Rule::base(Token::Predictive, Token::Comment),
    Rule::token(Token::Preproc, Token::Preprocessor),
    Rule::base(Token::Primary, Token::Foreground),
    Rule::token(Token::Property, Token::VariableProperty),
    Rule::token(Token::PropertyJsonKey, Token::Key),
    Rule::token(Token::Punctuation, Token::Punctuation),
    Rule::token(Token::PunctuationBracket, Token::Bracket),
    Rule::token(Token::PunctuationBracketHtml, Token::TagBracket),
    Rule::token(Token::PunctuationBracketJsx, Token::TagBracket),
    Rule::token(Token::PunctuationDelimiter, Token::Delimiter),
    Rule::token(Token::PunctuationDelimiterHtml, Token::TagBracket),
    Rule::token(Token::PunctuationDelimiterJsx, Token::TagBracket),
    Rule::base(Token::PunctuationListMarker, Token::Type),
    Rule::base(Token::PunctuationSpecial, Token::Keyword),
    Rule::token(Token::Selector, Token::Tag),
    Rule::token(Token::SelectorClass, Token::ClassName),
    Rule::base(Token::SelectorId, Token::Function),
    Rule::base(Token::SelectorPseudo, Token::Operator),
    Rule::base(Token::String, Token::String),
    Rule::token(Token::StringDotEscape, Token::StringEscape),
    Rule::token(Token::StringDotRegex, Token::StringRegex),
    Rule::token(Token::StringDotSpecial, Token::StringEscape),
    Rule::token(Token::StringDotSpecialSymbol, Token::Constant),
    Rule::token(Token::Tag, Token::Tag),
    Rule::token(Token::TagComponentJsx, Token::JsxTag),
    Rule::base(Token::TagDoctype, Token::Keyword),
    Rule::base(Token::TextJsx, Token::Foreground),
    Rule::base(Token::TextLiteral, Token::String),
    Rule::base(Token::Title, Token::Keyword).bold(),
    Rule::base(Token::Type, Token::Type),
    Rule::token(Token::TypeDotBuiltin, Token::TypePrimitive),
    Rule::token(Token::TypeDotClass, Token::ClassName),
    Rule::base(Token::TypeDotJsdoc, Token::Type),
    Rule::base(Token::Variable, Token::Variable),
    Rule::token(Token::VariableJsdoc, Token::Parameter),
    Rule::token(Token::VariableDotParameter, Token::Parameter),
    Rule::token(Token::VariableDotSpecial, Token::VariableBuiltin),
    Rule::token(Token::Variant, Token::Constant),
];
