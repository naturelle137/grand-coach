// SPDX-License-Identifier: EUPL-1.2

//! Full Fluent formatting for the Godot frontend, including coach selectors.

use fluent_bundle::{FluentArgs, FluentBundle, FluentResource};
use serde_json::Value;

/// Embedded bilingual resources; no source-tree files are needed at runtime.
pub struct Localization {
    de: FluentBundle<FluentResource>,
    en: FluentBundle<FluentResource>,
}

fn bundle(language: &str, source: &str) -> FluentBundle<FluentResource> {
    let resource = FluentResource::try_new(source.to_owned())
        .unwrap_or_else(|(_, errors)| panic!("{language}.ftl: {errors:?}"));
    let mut bundle = FluentBundle::new(vec![language.parse().expect("language id")]);
    bundle.set_use_isolating(false);
    bundle.add_resource(resource).expect("unique message ids");
    bundle
}

impl Default for Localization {
    fn default() -> Self {
        Self {
            de: bundle("de", include_str!("../../../assets/i18n/de.ftl")),
            en: bundle("en", include_str!("../../../assets/i18n/en.ftl")),
        }
    }
}

impl Localization {
    /// Formats a message using typed JSON arguments. Unknown messages remain
    /// visible as keys, as in the existing frontend.
    pub fn format(&self, language: &str, key: &str, arguments: &Value) -> Result<String, String> {
        let bundle = match language {
            "de" => &self.de,
            "en" => &self.en,
            _ => return Err(format!("Unsupported language: {language}")),
        };
        let Some(pattern) = bundle.get_message(key).and_then(|message| message.value()) else {
            return Ok(key.to_owned());
        };
        let values = arguments.as_object().ok_or("Expected argument object")?;
        let mut args = FluentArgs::new();
        for (name, value) in values {
            match value {
                Value::String(text) => args.set(name, text.as_str()),
                Value::Number(number) => {
                    args.set(name, number.as_f64().ok_or("Invalid numeric argument")?);
                }
                _ => return Err(format!("Invalid argument: {name}")),
            }
        }
        let mut errors = Vec::new();
        let text = bundle.format_pattern(pattern, Some(&args), &mut errors);
        if errors.is_empty() {
            Ok(text.into_owned())
        } else {
            Err(format!("{key}: {errors:?}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn formats_both_languages_and_multiline_coach_selectors() {
        let texts = Localization::default();
        assert_eq!(
            texts
                .format("de", "status-bid", &json!({"value":18}))
                .unwrap(),
            "Reizwert: 18"
        );
        assert_eq!(
            texts
                .format("en", "status-bid", &json!({"value":18}))
                .unwrap(),
            "Bid: 18"
        );
        for language in ["de", "en"] {
            let zero = texts
                .format(
                    language,
                    "coach-lesson-remaining-trumpf-own",
                    &json!({"count":0,"cards":"Kreuz-Bube"}),
                )
                .unwrap();
            let one = texts
                .format(
                    language,
                    "coach-lesson-remaining-trumpf-own",
                    &json!({"count":1,"cards":"Kreuz-Bube"}),
                )
                .unwrap();
            assert_ne!(zero, one);
            assert!(one.contains("Kreuz-Bube"));
            assert!(!one.contains("->"));
        }
    }

    #[test]
    fn rejects_invalid_arguments_and_reports_missing_variables() {
        let texts = Localization::default();
        assert!(
            texts
                .format("fr", "status-bid", &json!({"value":18}))
                .is_err()
        );
        assert!(texts.format("de", "status-bid", &json!([])).is_err());
        assert!(
            texts
                .format("de", "status-bid", &json!({"value":true}))
                .is_err()
        );
        assert!(texts.format("de", "status-bid", &json!({})).is_err());
        assert_eq!(
            texts.format("de", "unknown-key", &json!({})).unwrap(),
            "unknown-key"
        );
    }
}
