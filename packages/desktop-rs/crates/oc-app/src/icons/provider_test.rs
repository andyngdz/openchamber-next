use super::{ProviderLogo, provider_logo};

#[test]
fn bundled_logos_win_by_alias() {
    assert_eq!(
        provider_logo("claude"),
        ProviderLogo::Bundled {
            id: "anthropic".to_owned()
        }
    );
    assert_eq!(
        provider_logo("models.openai"),
        ProviderLogo::Bundled {
            id: "openai".to_owned()
        }
    );
}

#[test]
fn an_unbundled_provider_gets_a_models_dev_id() {
    assert_eq!(
        provider_logo("perf"),
        ProviderLogo::Remote {
            id: "perf".to_owned()
        }
    );
    assert_eq!(provider_logo("perf").image_id(), Some("perf"));
}

#[test]
fn no_id_has_no_logo_and_no_candidate() {
    assert_eq!(provider_logo("  "), ProviderLogo::Fallback { icon: None });
}
