// SPDX-License-Identifier: EUPL-1.2

//! Translations (D-15, D-32): Fluent files from `assets/i18n/`, embedded at
//! compile time.

use fluent_bundle::{FluentArgs, FluentBundle, FluentResource, FluentValue};

/// German messages.
pub const DE_FTL: &str = include_str!("../../../assets/i18n/de.ftl");
/// English messages.
pub const EN_FTL: &str = include_str!("../../../assets/i18n/en.ftl");

/// UI language.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Language {
    /// German.
    De,
    /// English.
    En,
}

impl Language {
    /// All languages.
    pub const ALL: [Language; 2] = [Language::De, Language::En];
}

/// Both message bundles and the selected language.
pub struct I18n {
    de: FluentBundle<FluentResource>,
    en: FluentBundle<FluentResource>,
    /// Selected language.
    pub language: Language,
}

fn bundle(lang: &str, source: &str) -> FluentBundle<FluentResource> {
    let resource = FluentResource::try_new(source.to_owned())
        .unwrap_or_else(|(_, errors)| panic!("{lang}.ftl does not parse: {errors:?}"));
    let mut bundle = FluentBundle::new(vec![lang.parse().expect("valid language id")]);
    // egui shows the Unicode isolation marks Fluent adds by default.
    bundle.set_use_isolating(false);
    bundle
        .add_resource(resource)
        .unwrap_or_else(|errors| panic!("{lang}.ftl has duplicate ids: {errors:?}"));
    bundle
}

impl I18n {
    /// Loads both bundles.
    pub fn new(language: Language) -> I18n {
        I18n {
            de: bundle("de", DE_FTL),
            en: bundle("en", EN_FTL),
            language,
        }
    }

    fn current(&self) -> &FluentBundle<FluentResource> {
        match self.language {
            Language::De => &self.de,
            Language::En => &self.en,
        }
    }

    /// The message `id` without arguments.
    pub fn t(&self, id: &str) -> String {
        self.tf(id, &[])
    }

    /// The message `id` with arguments. A missing id shows the id itself;
    /// tests make sure every id exists in both languages.
    pub fn tf(&self, id: &str, args: &[(&str, FluentValue<'_>)]) -> String {
        let bundle = self.current();
        let Some(pattern) = bundle.get_message(id).and_then(|m| m.value()) else {
            return id.to_owned();
        };
        let mut fluent_args = FluentArgs::new();
        for (name, value) in args {
            fluent_args.set(*name, value.clone());
        }
        let mut errors = Vec::new();
        bundle
            .format_pattern(pattern, Some(&fluent_args), &mut errors)
            .into_owned()
    }
}

/// Message ids defined in a Fluent source.
pub fn message_ids(source: &str) -> Vec<String> {
    source
        .lines()
        .filter_map(|line| {
            let (id, _) = line.split_once(" =")?;
            let valid = !id.is_empty()
                && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
                && !line.starts_with([' ', '#', '-']);
            valid.then(|| id.to_owned())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};

    fn ids(source: &str) -> BTreeSet<String> {
        message_ids(source).into_iter().collect()
    }

    /// String literals in the frontend sources that look like message ids.
    fn ids_in_code() -> BTreeSet<String> {
        let mut found = BTreeSet::new();
        for source in [
            include_str!("app.rs"),
            include_str!("text.rs"),
            include_str!("cards.rs"),
            include_str!("coach.rs"),
        ] {
            for literal in source.split('"').skip(1).step_by(2) {
                let looks_like_id = literal.contains('-')
                    && literal
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
                if looks_like_id {
                    found.insert(literal.to_owned());
                }
            }
        }
        found
    }

    #[test]
    fn both_languages_define_the_same_messages() {
        let de = ids(DE_FTL);
        let en = ids(EN_FTL);
        assert!(de.len() > 100, "{}", de.len());
        assert_eq!(de.len(), message_ids(DE_FTL).len(), "duplicate ids");
        assert_eq!(
            de.symmetric_difference(&en).collect::<Vec<_>>(),
            Vec::<&String>::new()
        );
    }

    #[test]
    fn every_id_in_the_code_is_translated_and_every_message_used() {
        let code = ids_in_code();
        let ftl = ids(DE_FTL);
        let jump_id = "jump-bid"; // egui widget id, not a message
        let missing: Vec<_> = code
            .iter()
            .filter(|id| !ftl.contains(*id) && *id != jump_id)
            .collect();
        assert!(missing.is_empty(), "missing translations: {missing:?}");
        let unused: Vec<_> = ftl.iter().filter(|id| !code.contains(*id)).collect();
        assert!(unused.is_empty(), "unused messages: {unused:?}");
    }

    /// The variables each message of `source` uses.
    fn variables(source: &str) -> BTreeMap<String, BTreeSet<String>> {
        let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        let mut current = None;
        for line in source.lines() {
            if let Some((id, _)) = line.split_once(" =")
                && message_ids(line).len() == 1
            {
                current = Some(id.to_owned());
            } else if !line.starts_with(' ') {
                current = None;
            }
            if let Some(id) = &current {
                let vars = out.entry(id.clone()).or_default();
                for part in line.split('$').skip(1) {
                    let name: String = part
                        .chars()
                        .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
                        .collect();
                    vars.insert(name);
                }
            }
        }
        out
    }

    #[test]
    fn both_languages_use_the_same_arguments() {
        let de = variables(DE_FTL);
        let en = variables(EN_FTL);
        assert!(de["coach-lesson-proven-void-evidence"].contains("follow"));
        for (id, vars) in &de {
            assert_eq!(Some(vars), en.get(id), "{id}");
        }
    }

    #[test]
    fn messages_format_without_errors() {
        for language in Language::ALL {
            let i18n = I18n::new(language);
            let source = match language {
                Language::De => DE_FTL,
                Language::En => EN_FTL,
            };
            for id in message_ids(source) {
                let bundle = i18n.current();
                let pattern = bundle.get_message(&id).and_then(|m| m.value()).unwrap();
                let mut args = FluentArgs::new();
                for name in [
                    "number",
                    "total",
                    "player",
                    "value",
                    "seed",
                    "cards",
                    "tricks",
                    "suit",
                    "rank",
                    "augen",
                    "game",
                    "count",
                    "bid",
                    "score",
                    "points",
                    "games",
                    "won",
                    "lost",
                    "phase",
                    "current",
                    "card",
                    "left",
                    "right",
                    "mode",
                    "target",
                    "declarer",
                    "defenders",
                    "holder",
                    "class",
                    "expected",
                    "min",
                    "max",
                    "other",
                    "chance",
                    "party",
                    "se",
                    "finished",
                    "best",
                    "action",
                    "note",
                    "decision",
                    "trick",
                    "judgement",
                    "trap",
                    "kind",
                    "skat",
                    "led",
                    "follow",
                    "case",
                    "certainty",
                    "trumps",
                    "way",
                    "focus",
                    "seat",
                    "role",
                    "dealer",
                    "level",
                    "calls",
                    "call",
                    "winner",
                ] {
                    args.set(name, "x");
                }
                let mut errors = Vec::new();
                let text = bundle.format_pattern(pattern, Some(&args), &mut errors);
                assert!(errors.is_empty(), "{language:?} {id}: {errors:?}");
                assert!(!text.trim().is_empty(), "{language:?} {id} is empty");
            }
        }
        let de = I18n::new(Language::De);
        assert_eq!(de.tf("status-bid", &[("value", 18.into())]), "Reizwert: 18");
        assert_eq!(de.t("no-such-message"), "no-such-message");
    }
}
