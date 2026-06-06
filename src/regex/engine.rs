// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Runtime regex execution and match conversion helpers.

use super::support::*;
use super::*;

impl RuntimeRegex {
    /// Returns the rendered regex text used to compile this value.
    pub fn rendered(&self) -> &str {
        &self.rendered
    }

    /// Tests whether the regex matches anywhere in the input text.
    pub fn is_match(&self, text: &str) -> NodiaResult<bool> {
        self.engine.is_match(text).map_err(regex_engine_error)
    }

    /// Tests whether the regex matches the entire input text.
    pub fn is_full_match(&self, text: &str) -> NodiaResult<bool> {
        let captures = self.engine.captures(text).map_err(regex_engine_error)?;
        Ok(captures
            .and_then(|captures| captures.get(0))
            .is_some_and(|matched| matched.start() == 0 && matched.end() == text.len()))
    }

    /// Returns the first match, if any.
    pub fn find(&self, text: &str) -> NodiaResult<Option<RegexMatch>> {
        let captures = self.engine.captures(text).map_err(regex_engine_error)?;
        captures
            .map(|captures| self.capture_to_match(text, &captures))
            .transpose()
    }

    /// Returns every non-overlapping match in the input text.
    pub fn find_all(&self, text: &str) -> NodiaResult<Vec<RegexMatch>> {
        let mut matches = Vec::new();
        for captures in self.engine.captures_iter(text) {
            let captures = captures.map_err(regex_engine_error)?;
            matches.push(self.capture_to_match(text, &captures)?);
        }
        Ok(matches)
    }

    /// Replaces every match using Nodia replacement placeholders.
    pub fn replace_all(&self, text: &str, replacement: &str) -> NodiaResult<String> {
        let translated = self.translate_replacement(replacement)?;
        self.engine
            .try_replacen(text, 0, translated.as_str())
            .map(|value| value.into_owned())
            .map_err(regex_engine_error)
    }

    /// Splits input text on regex matches.
    pub fn split(&self, text: &str) -> NodiaResult<Vec<String>> {
        self.engine
            .split(text)
            .map(|part| {
                part.map(|value| value.to_string())
                    .map_err(regex_engine_error)
            })
            .collect()
    }

    fn capture_to_match(&self, text: &str, captures: &Captures<'_>) -> NodiaResult<RegexMatch> {
        let matched = captures
            .get(0)
            .ok_or_else(|| NodiaError::runtime("regex engine returned a match without group 0"))?;
        let groups = (1..captures.len())
            .map(|index| captures.get(index).map(|value| value.as_str().to_string()))
            .collect();
        let mut named = BTreeMap::new();
        for name in self.engine.capture_names().flatten() {
            named
                .entry(name.to_string())
                .or_insert_with(|| captures.name(name).map(|value| value.as_str().to_string()));
        }
        Ok(RegexMatch {
            text: matched.as_str().to_string(),
            start: scalar_offset(text, matched.start()),
            end: scalar_offset(text, matched.end()),
            groups,
            named,
        })
    }

    fn translate_replacement(&self, replacement: &str) -> NodiaResult<String> {
        let mut out = String::new();
        let chunks = parse_replacement_chunks(replacement).map_err(NodiaError::runtime)?;
        let names = self
            .engine
            .capture_names()
            .flatten()
            .map(str::to_string)
            .collect::<HashSet<_>>();
        let capture_len = self.engine.captures_len();

        for chunk in chunks {
            match chunk {
                ReplacementChunk::Literal(value) => out.push_str(&value),
                ReplacementChunk::Dollar => out.push_str("$$"),
                ReplacementChunk::CaptureIndex { raw, index } => {
                    if index >= capture_len {
                        return Err(NodiaError::runtime(format!(
                            "regex replacement refers to missing capture group {index}"
                        )));
                    }
                    out.push('$');
                    out.push_str(&raw);
                }
                ReplacementChunk::CaptureName(name) => {
                    if !names.contains(&name) {
                        return Err(NodiaError::runtime(format!(
                            "regex replacement refers to missing named capture '{name}'"
                        )));
                    }
                    out.push_str("${");
                    out.push_str(&name);
                    out.push('}');
                }
            }
        }

        Ok(out)
    }
}

impl PartialEq for RuntimeRegex {
    fn eq(&self, other: &Self) -> bool {
        self.rendered == other.rendered
    }
}

impl fmt::Debug for RuntimeRegex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RuntimeRegex")
            .field("rendered", &self.rendered)
            .finish()
    }
}
