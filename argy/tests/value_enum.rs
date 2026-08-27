//! Tests for the `#[derive(argy::ValueEnum)]` derive macro.

use argy::{FromArgValue, FromArgs, ValueEnum};
use std::str::FromStr;

/// Default casing is `kebab_case` (clap parity).
#[derive(ValueEnum, Debug, PartialEq, Clone, Copy)]
enum Status {
    InProgress,
    NotStarted,
    Done,
}

/// `snake_case` via `rename_all`.
#[derive(ValueEnum, Debug, PartialEq, Clone, Copy)]
#[argy(rename_all = "snake_case")]
enum AgentStatus {
    InProgress,
    NotStarted,
}

/// name and alias overrides on variants.
#[derive(ValueEnum, Debug, PartialEq, Clone, Copy)]
enum StoryType {
    #[argy(name = "feat", alias = "feature", alias = "f")]
    Feature,
    #[argy(name = "bugfix", alias = "fix")]
    BugFix,
}

#[test]
fn kebab_case_default_parse_and_display() {
    assert_eq!(Status::from_str("in-progress"), Ok(Status::InProgress));
    assert_eq!(Status::from_str("not-started"), Ok(Status::NotStarted));
    assert_eq!(Status::from_str("done"), Ok(Status::Done));

    assert_eq!(Status::InProgress.to_string(), "in-progress");
    assert_eq!(Status::NotStarted.to_string(), "not-started");
    assert_eq!(Status::Done.to_string(), "done");
}

#[test]
fn snake_case_rename_all() {
    assert_eq!(AgentStatus::from_str("in_progress"), Ok(AgentStatus::InProgress));
    assert_eq!(AgentStatus::from_str("not_started"), Ok(AgentStatus::NotStarted));
    assert_eq!(AgentStatus::InProgress.to_string(), "in_progress");
    assert_eq!(AgentStatus::NotStarted.to_string(), "not_started");
}

#[test]
fn name_and_alias_overrides() {
    // Canonical overridden names parse.
    assert_eq!(StoryType::from_str("feat"), Ok(StoryType::Feature));
    assert_eq!(StoryType::from_str("bugfix"), Ok(StoryType::BugFix));
    // Aliases parse too.
    assert_eq!(StoryType::from_str("feature"), Ok(StoryType::Feature));
    assert_eq!(StoryType::from_str("f"), Ok(StoryType::Feature));
    assert_eq!(StoryType::from_str("fix"), Ok(StoryType::BugFix));
    // Display renders the canonical overridden name, not the alias.
    assert_eq!(StoryType::Feature.to_string(), "feat");
    assert_eq!(StoryType::BugFix.to_string(), "bugfix");
}

#[test]
fn from_str_display_round_trip() {
    for variant in Status::value_variants() {
        let rendered = variant.to_string();
        assert_eq!(Status::from_str(&rendered), Ok(*variant));
    }
    for variant in AgentStatus::value_variants() {
        let rendered = variant.to_string();
        assert_eq!(AgentStatus::from_str(&rendered), Ok(*variant));
    }
    for variant in StoryType::value_variants() {
        let rendered = variant.to_string();
        assert_eq!(StoryType::from_str(&rendered), Ok(*variant));
    }
}

#[test]
fn value_variants_lists_all_variants() {
    assert_eq!(Status::value_variants(), &[Status::InProgress, Status::NotStarted, Status::Done]);
    assert_eq!(AgentStatus::value_variants(), &[AgentStatus::InProgress, AgentStatus::NotStarted]);
    assert_eq!(StoryType::value_variants(), &[StoryType::Feature, StoryType::BugFix]);
}

#[test]
fn to_possible_value_introspection() {
    let in_progress = Status::InProgress.to_possible_value().expect("has a possible value");
    assert_eq!(in_progress.name(), "in-progress");
    assert_eq!(in_progress.aliases(), &[] as &[&str]);

    let feature = StoryType::Feature.to_possible_value().expect("has a possible value");
    assert_eq!(feature.name(), "feat");
    assert_eq!(feature.aliases(), &["feature", "f"] as &[&str]);
}

#[test]
fn parse_error_lists_expected_values() {
    let err = Status::from_str("bogus").unwrap_err();
    assert_eq!(err, "expected \"in-progress\", \"not-started\" or \"done\"");
}

#[test]
fn value_enum_implements_from_arg_value() {
    // Direct FromArgValue call (delegates to the generated FromStr).
    assert_eq!(FromArgValue::from_arg_value("in-progress"), Ok(Status::InProgress));
    assert!(<Status as FromArgValue>::from_arg_value("bogus").is_err());
}

#[test]
fn value_enum_works_as_from_args_option() {
    #[derive(FromArgs)]
    /// Do the thing.
    struct DoIt {
        /// how to do it.
        #[argy(option)]
        mode: Status,
    }

    let parsed = DoIt::from_args(&["do-it"], &["--mode", "in-progress"]).unwrap();
    assert_eq!(parsed.mode, Status::InProgress);

    // Invalid value fails parsing.
    assert!(DoIt::from_args(&["do-it"], &["--mode", "bogus"]).is_err());
}
