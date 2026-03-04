//! Word Tools — an example Omni extension
//!
//! Demonstrates: multiple tools, storage, HTTP requests, config access,
//! logging, error handling, and JSON parameter parsing.

use omni_sdk::prelude::*;
use std::collections::HashMap;

/// Stop words filtered out during keyword extraction.
const STOP_WORDS: &[&str] = &[
    "a", "an", "the", "and", "or", "but", "is", "are", "was", "were", "be", "been", "being",
    "have", "has", "had", "do", "does", "did", "will", "would", "could", "should", "may",
    "might", "shall", "can", "need", "dare", "ought", "used", "to", "of", "in", "for", "on",
    "with", "at", "by", "from", "as", "into", "through", "during", "before", "after", "above",
    "below", "between", "out", "off", "over", "under", "again", "further", "then", "once",
    "here", "there", "when", "where", "why", "how", "all", "each", "every", "both", "few",
    "more", "most", "other", "some", "such", "no", "nor", "not", "only", "own", "same", "so",
    "than", "too", "very", "just", "because", "if", "while", "about", "up", "it", "its",
    "he", "she", "they", "them", "his", "her", "this", "that", "these", "those", "i", "me",
    "my", "we", "our", "you", "your",
];

#[derive(Default)]
struct WordToolsExtension;

impl Extension for WordToolsExtension {
    fn handle_tool(
        &mut self,
        ctx: &Context,
        tool_name: &str,
        params: serde_json::Value,
    ) -> ToolResult {
        ctx.debug(&format!("Handling tool: {tool_name}"));

        match tool_name {
            "word_count" => self.word_count(ctx, &params),
            "readability" => self.readability(ctx, &params),
            "transform_case" => self.transform_case(ctx, &params),
            "find_replace" => self.find_replace(ctx, &params),
            "extract_keywords" => self.extract_keywords(ctx, &params),
            "define_word" => self.define_word(ctx, &params),
            "text_stats_history" => self.text_stats_history(ctx, &params),
            _ => Err(SdkError::UnknownTool(tool_name.to_string())),
        }
    }
}

impl WordToolsExtension {
    // ── Helpers ──────────────────────────────────────────────────────

    fn require_text<'a>(&self, params: &'a serde_json::Value) -> Result<&'a str, SdkError> {
        params["text"]
            .as_str()
            .ok_or_else(|| SdkError::Other("Missing required parameter: 'text'".into()))
    }

    fn max_text_length(&self, ctx: &Context) -> usize {
        ctx.config()
            .get("max_text_length")
            .ok()
            .flatten()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(100_000)
    }

    fn check_length(&self, ctx: &Context, text: &str) -> Result<(), SdkError> {
        let max = self.max_text_length(ctx);
        if text.len() > max {
            return Err(SdkError::Other(format!(
                "Text exceeds maximum length ({} > {max})",
                text.len()
            )));
        }
        Ok(())
    }

    /// Record a tool invocation in persistent storage for history tracking.
    fn record_history(&self, ctx: &Context, tool: &str, summary: &str) {
        let storage = ctx.storage();
        let key = "analysis_history";
        let mut history: Vec<String> = storage
            .get(key)
            .ok()
            .flatten()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        history.push(format!("{tool}: {summary}"));

        // Keep only the last 50 entries
        if history.len() > 50 {
            history.drain(..history.len() - 50);
        }

        if let Ok(json) = serde_json::to_string(&history) {
            let _ = storage.set(key, &json);
        }
    }

    fn count_syllables(word: &str) -> u32 {
        let word = word.to_lowercase();
        if word.is_empty() {
            return 0;
        }
        let vowels = ['a', 'e', 'i', 'o', 'u'];
        let mut count = 0u32;
        let mut prev_vowel = false;
        for ch in word.chars() {
            let is_vowel = vowels.contains(&ch);
            if is_vowel && !prev_vowel {
                count += 1;
            }
            prev_vowel = is_vowel;
        }
        // Words ending in silent 'e'
        if word.ends_with('e') && count > 1 {
            count -= 1;
        }
        count.max(1)
    }

    // ── Tool Implementations ────────────────────────────────────────

    /// Count words, sentences, paragraphs, and characters.
    fn word_count(&self, ctx: &Context, params: &serde_json::Value) -> ToolResult {
        let text = self.require_text(params)?;
        self.check_length(ctx, text)?;

        let chars = text.len();
        let chars_no_spaces = text.chars().filter(|c| !c.is_whitespace()).count();
        let words: Vec<&str> = text.split_whitespace().collect();
        let word_count = words.len();
        let sentences = text
            .chars()
            .filter(|&c| c == '.' || c == '!' || c == '?')
            .count()
            .max(if word_count > 0 { 1 } else { 0 });
        let paragraphs = text
            .split("\n\n")
            .filter(|p| !p.trim().is_empty())
            .count()
            .max(if word_count > 0 { 1 } else { 0 });

        let summary = format!("{word_count} words, {sentences} sentences");
        self.record_history(ctx, "word_count", &summary);
        ctx.info(&format!("Word count complete: {summary}"));

        Ok(serde_json::json!({
            "words": word_count,
            "characters": chars,
            "characters_no_spaces": chars_no_spaces,
            "sentences": sentences,
            "paragraphs": paragraphs,
            "avg_word_length": if word_count > 0 {
                words.iter().map(|w| w.len()).sum::<usize>() as f64 / word_count as f64
            } else {
                0.0
            }
        }))
    }

    /// Compute readability metrics (Flesch-Kincaid grade level).
    fn readability(&self, ctx: &Context, params: &serde_json::Value) -> ToolResult {
        let text = self.require_text(params)?;
        self.check_length(ctx, text)?;

        let words: Vec<&str> = text.split_whitespace().collect();
        let word_count = words.len();
        if word_count == 0 {
            return Ok(serde_json::json!({
                "error": "Text is empty — cannot compute readability."
            }));
        }

        let sentence_count = text
            .chars()
            .filter(|&c| c == '.' || c == '!' || c == '?')
            .count()
            .max(1) as f64;

        let total_syllables: u32 = words.iter().map(|w| Self::count_syllables(w)).sum();
        let avg_sentence_len = word_count as f64 / sentence_count;
        let avg_syllables = total_syllables as f64 / word_count as f64;

        // Flesch-Kincaid Grade Level
        let fk_grade = 0.39 * avg_sentence_len + 11.8 * avg_syllables - 15.59;
        // Flesch Reading Ease
        let fk_ease = 206.835 - 1.015 * avg_sentence_len - 84.6 * avg_syllables;

        let grade_level = format!("{:.1}", fk_grade);
        self.record_history(ctx, "readability", &format!("grade {grade_level}"));

        Ok(serde_json::json!({
            "flesch_kincaid_grade": (fk_grade * 10.0).round() / 10.0,
            "flesch_reading_ease": (fk_ease * 10.0).round() / 10.0,
            "avg_sentence_length": (avg_sentence_len * 10.0).round() / 10.0,
            "avg_syllables_per_word": (avg_syllables * 100.0).round() / 100.0,
            "total_words": word_count,
            "total_sentences": sentence_count as u64,
            "total_syllables": total_syllables,
            "difficulty": match fk_grade {
                g if g < 6.0 => "Easy (elementary school)",
                g if g < 10.0 => "Medium (middle/high school)",
                g if g < 14.0 => "Difficult (college level)",
                _ => "Very difficult (graduate level)",
            }
        }))
    }

    /// Transform text case.
    fn transform_case(&self, ctx: &Context, params: &serde_json::Value) -> ToolResult {
        let text = self.require_text(params)?;
        self.check_length(ctx, text)?;

        let mode = params["mode"]
            .as_str()
            .ok_or_else(|| SdkError::Other("Missing required parameter: 'mode'".into()))?;

        let result = match mode {
            "uppercase" => text.to_uppercase(),
            "lowercase" => text.to_lowercase(),
            "title_case" => text
                .split_whitespace()
                .map(|word| {
                    let mut chars = word.chars();
                    match chars.next() {
                        Some(c) => {
                            let upper: String = c.to_uppercase().collect();
                            let rest: String = chars.as_str().to_lowercase();
                            format!("{upper}{rest}")
                        }
                        None => String::new(),
                    }
                })
                .collect::<Vec<_>>()
                .join(" "),
            "sentence_case" => {
                let mut result = String::with_capacity(text.len());
                let mut capitalize_next = true;
                for ch in text.chars() {
                    if capitalize_next && ch.is_alphabetic() {
                        result.extend(ch.to_uppercase());
                        capitalize_next = false;
                    } else {
                        result.push(if ch.is_alphabetic() {
                            ch.to_lowercase().next().unwrap_or(ch)
                        } else {
                            ch
                        });
                        if ch == '.' || ch == '!' || ch == '?' {
                            capitalize_next = true;
                        }
                    }
                }
                result
            }
            "snake_case" => to_delimiter_case(text, '_'),
            "kebab_case" => to_delimiter_case(text, '-'),
            "camel_case" => {
                let pascal = to_pascal_case(text);
                let mut chars = pascal.chars();
                match chars.next() {
                    Some(c) => {
                        let lower: String = c.to_lowercase().collect();
                        format!("{lower}{}", chars.as_str())
                    }
                    None => String::new(),
                }
            }
            "pascal_case" => to_pascal_case(text),
            _ => {
                return Err(SdkError::Other(format!("Unknown case mode: '{mode}'")));
            }
        };

        self.record_history(ctx, "transform_case", mode);

        Ok(serde_json::json!({
            "original": text,
            "transformed": result,
            "mode": mode
        }))
    }

    /// Find and replace text.
    fn find_replace(&self, ctx: &Context, params: &serde_json::Value) -> ToolResult {
        let text = self.require_text(params)?;
        self.check_length(ctx, text)?;

        let find = params["find"]
            .as_str()
            .ok_or_else(|| SdkError::Other("Missing required parameter: 'find'".into()))?;
        let replace = params["replace"]
            .as_str()
            .ok_or_else(|| SdkError::Other("Missing required parameter: 'replace'".into()))?;

        if find.is_empty() {
            return Err(SdkError::Other("'find' cannot be empty".into()));
        }

        let count = text.matches(find).count();
        let result = text.replace(find, replace);

        self.record_history(ctx, "find_replace", &format!("{count} replacements"));

        Ok(serde_json::json!({
            "result": result,
            "replacements": count,
            "find": find,
            "replace": replace
        }))
    }

    /// Extract top keywords by frequency.
    fn extract_keywords(&self, ctx: &Context, params: &serde_json::Value) -> ToolResult {
        let text = self.require_text(params)?;
        self.check_length(ctx, text)?;

        let top_n = params["top_n"].as_u64().unwrap_or(10) as usize;

        // Tokenize and normalize
        let mut freq: HashMap<String, u32> = HashMap::new();
        for word in text.split(|c: char| !c.is_alphanumeric() && c != '\'') {
            let lower = word.to_lowercase();
            if lower.len() < 2 {
                continue;
            }
            if STOP_WORDS.contains(&lower.as_str()) {
                continue;
            }
            *freq.entry(lower).or_insert(0) += 1;
        }

        // Sort by frequency descending, then alphabetically
        let mut keywords: Vec<(String, u32)> = freq.into_iter().collect();
        keywords.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        keywords.truncate(top_n);

        let summary = format!("{} keywords extracted", keywords.len());
        self.record_history(ctx, "extract_keywords", &summary);

        Ok(serde_json::json!({
            "keywords": keywords.iter().map(|(word, count)| {
                serde_json::json!({ "word": word, "count": count })
            }).collect::<Vec<_>>(),
            "total_unique_words": keywords.len()
        }))
    }

    /// Look up a word definition via the free Dictionary API.
    /// Demonstrates HTTP requests and JSON response parsing.
    fn define_word(&self, ctx: &Context, params: &serde_json::Value) -> ToolResult {
        let word = params["word"]
            .as_str()
            .ok_or_else(|| SdkError::Other("Missing required parameter: 'word'".into()))?;

        if word.is_empty() || word.len() > 100 {
            return Err(SdkError::Other(
                "Word must be between 1 and 100 characters".into(),
            ));
        }

        ctx.info(&format!("Looking up definition for: {word}"));

        let url = format!("https://api.dictionaryapi.dev/api/v2/entries/en/{word}");
        let response = ctx.http().get(&url)?;

        if response.status == 404 {
            return Ok(serde_json::json!({
                "word": word,
                "found": false,
                "message": format!("No definition found for '{word}'.")
            }));
        }

        if response.status != 200 {
            return Err(SdkError::HttpError(format!(
                "Dictionary API returned status {}",
                response.status
            )));
        }

        let body: serde_json::Value = response.json()?;

        // Extract the first entry's meanings
        let mut definitions = Vec::new();
        if let Some(entries) = body.as_array() {
            if let Some(entry) = entries.first() {
                if let Some(meanings) = entry["meanings"].as_array() {
                    for meaning in meanings {
                        let part_of_speech =
                            meaning["partOfSpeech"].as_str().unwrap_or("unknown");
                        if let Some(defs) = meaning["definitions"].as_array() {
                            for def in defs.iter().take(2) {
                                definitions.push(serde_json::json!({
                                    "part_of_speech": part_of_speech,
                                    "definition": def["definition"].as_str().unwrap_or(""),
                                    "example": def.get("example").and_then(|e| e.as_str())
                                }));
                            }
                        }
                    }
                }
            }
        }

        self.record_history(ctx, "define_word", word);

        Ok(serde_json::json!({
            "word": word,
            "found": true,
            "definitions": definitions
        }))
    }

    /// View or clear the analysis history stored in persistent storage.
    /// Demonstrates the StorageClient.
    fn text_stats_history(&self, ctx: &Context, params: &serde_json::Value) -> ToolResult {
        let action = params["action"].as_str().unwrap_or("view");

        match action {
            "view" => {
                let history: Vec<String> = ctx
                    .storage()
                    .get("analysis_history")
                    .ok()
                    .flatten()
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or_default();

                ctx.info(&format!("History has {} entries", history.len()));

                Ok(serde_json::json!({
                    "entries": history,
                    "count": history.len()
                }))
            }
            "clear" => {
                ctx.storage().delete("analysis_history")?;
                ctx.info("History cleared");

                Ok(serde_json::json!({
                    "message": "History cleared.",
                    "count": 0
                }))
            }
            _ => Err(SdkError::Other(format!(
                "Unknown action: '{action}'. Use 'view' or 'clear'."
            ))),
        }
    }
}

// ── Free-standing helpers ───────────────────────────────────────────

fn split_words(text: &str) -> Vec<&str> {
    let mut words = Vec::new();
    let mut start = None;
    for (i, ch) in text.char_indices() {
        if ch.is_alphanumeric() {
            if start.is_none() {
                start = Some(i);
            }
        } else if let Some(s) = start {
            words.push(&text[s..i]);
            start = None;
        }
    }
    if let Some(s) = start {
        words.push(&text[s..]);
    }
    words
}

fn to_delimiter_case(text: &str, delimiter: char) -> String {
    split_words(text)
        .iter()
        .map(|w| w.to_lowercase())
        .collect::<Vec<_>>()
        .join(&delimiter.to_string())
}

fn to_pascal_case(text: &str) -> String {
    split_words(text)
        .iter()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(c) => {
                    let upper: String = c.to_uppercase().collect();
                    let rest: String = chars.as_str().to_lowercase();
                    format!("{upper}{rest}")
                }
                None => String::new(),
            }
        })
        .collect()
}

// Register the extension with the Omni runtime.
omni_sdk::omni_main!(WordToolsExtension);
