use super::state::OutputHighlightPreset;
use crate::config::OutputHighlightRule;
use crate::terminal::CompiledOutputRule;

pub(crate) fn compile_output_rules(rules: &[OutputHighlightRule]) -> Vec<CompiledOutputRule> {
    rules
        .iter()
        .filter(|rule| rule.enabled && !rule.pattern.trim().is_empty())
        .filter_map(|rule| {
            let pattern = if rule.regex {
                rule.pattern.clone()
            } else {
                regex::escape(&rule.pattern)
            };
            let matcher = regex::RegexBuilder::new(&pattern)
                .case_insensitive(!rule.case_sensitive)
                .build()
                .ok()?;
            let is_bg = !rule.bg_color.is_empty();
            let ansi_index = if is_bg {
                highlight_bg_color_index(&rule.bg_color)
            } else {
                highlight_color_index(&rule.color)
            };
            Some(CompiledOutputRule {
                matcher,
                whole_line: rule.whole_line,
                ansi_index,
                is_bg,
            })
        })
        .collect()
}

fn highlight_color_index(color: &str) -> u8 {
    match color {
        "yellow" => 11,
        "green" => 10,
        "cyan" => 14,
        "magenta" => 13,
        "gray" => 8,
        _ => 9,
    }
}

/// Soft 256-colour background palette for highlighter-pen style bg rules.
fn highlight_bg_color_index(color: &str) -> u8 {
    match color {
        "red" => 52,
        "yellow" => 58,
        "green" => 22,
        "cyan" => 24,
        "magenta" => 54,
        "orange" => 94,
        _ => 58,
    }
}

impl OutputHighlightPreset {
    pub(crate) fn from_settings(enabled: bool, preset: &str) -> Self {
        if !enabled {
            Self::Off
        } else if preset == "devops" {
            Self::DevOps
        } else if preset == "windterm" {
            Self::WindTerm
        } else {
            Self::Log
        }
    }
}
