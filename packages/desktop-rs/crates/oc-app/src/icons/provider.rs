//! Provider logos, ported from `packages/ui/src/hooks/useProviderLogo.ts` and
//! `components/ui/ProviderLogo.tsx`.
//!
//! A bundled logo wins and is named by the file React would draw; otherwise the first
//! candidate is the models.dev id React fetches. The capture records either as `img:<id>`.

/// `LOGO_ALIAS` (`useProviderLogo.ts:33-45`).
const ALIASES: &[(&str, &str)] = &[
    ("codex", "openai"),
    ("chatgpt", "openai"),
    ("claude", "anthropic"),
    ("claude-agent", "claude-code"),
    ("cline-pass", "cline"),
    ("gemini", "google"),
    ("evroc-ai", "evroc"),
    ("evrocai", "evroc"),
    ("ollama-cloud", "ollama"),
    ("wafer-ai", "wafer.ai"),
    ("wafer", "wafer.ai"),
    ("copilot", "github-copilot"),
];

/// `getProviderLogoFallbackIcon` (`components/ui/providerLogoFallback.ts`).
const FALLBACK: &[&str] = &[
    "command-code",
    "commandcode",
    "command_code",
    "command code",
];

/// `normalizeProviderId` (`useProviderLogo.ts:48-55`).
fn normalize(provider_id: &str) -> String {
    let mut normalized = provider_id.trim().to_lowercase();
    for prefix in ["models.", "provider."] {
        if let Some(rest) = normalized.strip_prefix(prefix) {
            normalized = rest.to_owned();
        }
    }
    let mut joined = String::with_capacity(normalized.len());
    for part in normalized.split_whitespace() {
        if !joined.is_empty() {
            joined.push('-');
        }
        joined.push_str(part);
    }
    joined
}

fn alias(value: &str) -> Option<&'static str> {
    ALIASES
        .iter()
        .find(|(from, _)| *from == value)
        .map(|(_, to)| *to)
}

/// `buildLogoCandidates` (`useProviderLogo.ts:57-77`), most specific first.
fn candidates(provider_id: &str) -> Vec<String> {
    let normalized = normalize(provider_id);
    if normalized.is_empty() {
        return Vec::new();
    }
    let compact: String = normalized
        .chars()
        .filter(|character| {
            character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || matches!(character, '_' | '-' | '.' | '/' | ':')
        })
        .collect();
    let primary = compact
        .split(['/', ':'])
        .next()
        .filter(|part| !part.is_empty())
        .unwrap_or(&compact);
    let prefix_alias = compact.starts_with("exe-").then_some("exe-dev");
    let mut out: Vec<String> = Vec::new();
    for value in [
        prefix_alias,
        alias(&compact),
        alias(primary),
        Some(compact.as_str()),
        Some(primary),
    ]
    .into_iter()
    .flatten()
    {
        if !value.is_empty() && !out.iter().any(|held| held == value) {
            out.push(value.to_owned());
        }
    }
    out
}

/// The ids a provider's logo can be named under, the provider's own first
/// (`providerLogoIds`, `useProviderLogo.ts:84`).
/// Where a provider's mark comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ProviderLogo {
    /// A bundled file, named by its file stem.
    Bundled { id: String },
    /// A models.dev file, which the app fetches like React does.
    Remote { id: String },
    /// No bundled logo and no candidate; React's fallback icon, if any.
    Fallback { icon: Option<super::RemixIcon> },
}

impl ProviderLogo {
    /// The name the dump and the capture record: `img:<id>` unless the fallback icon shows.
    pub(crate) fn image_id(&self) -> Option<&str> {
        match self {
            Self::Bundled { id } | Self::Remote { id } => Some(id),
            Self::Fallback { .. } => None,
        }
    }
}

/// Resolves a provider's mark the way `useProviderLogo` does.
pub(crate) fn provider_logo(provider_id: &str) -> ProviderLogo {
    let candidates = candidates(provider_id);
    if let Some(id) = candidates.iter().find(|candidate| {
        super::provider_logos::FILES
            .iter()
            .any(|(name, _)| name == candidate)
    }) {
        return ProviderLogo::Bundled { id: id.clone() };
    }
    if let Some(id) = candidates.first() {
        return ProviderLogo::Remote { id: id.clone() };
    }
    let fallback = FALLBACK
        .iter()
        .find(|name| normalize(provider_id) == **name)
        .map(|_| super::RemixIcon::TerminalBox);
    ProviderLogo::Fallback { icon: fallback }
}

#[cfg(test)]
#[path = "provider_test.rs"]
mod provider_test;
