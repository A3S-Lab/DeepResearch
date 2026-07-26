//! Host-owned output-language contracts for reader-facing research artifacts.

const MAX_LANGUAGE_TAG_CHARS: usize = 32;

/// Infer the language used by the user's request.
///
/// Callers may override this value explicitly on `DeepResearchRequest`. The
/// inference exists so CLI and TUI surfaces can preserve the language of a
/// plain-text request without adding a separate UI control.
pub fn infer_deep_research_output_language(text: &str) -> String {
    if text.chars().any(is_japanese_kana) {
        return "ja".to_string();
    }
    if text.chars().any(is_hangul) {
        return "ko".to_string();
    }
    if text.contains('¿') || text.contains('¡') {
        return "es".to_string();
    }
    let han_count = text.chars().filter(|character| is_han(*character)).count();
    let ascii_letter_count = text
        .chars()
        .filter(|character| character.is_ascii_alphabetic())
        .count();
    let uses_cjk_sentence_punctuation = text
        .chars()
        .any(|character| matches!(character, '。' | '，' | '；' | '：' | '、' | '？' | '！'));
    if han_count >= 4
        && (han_count.saturating_mul(2) >= ascii_letter_count || uses_cjk_sentence_punctuation)
    {
        return "zh".to_string();
    }
    if looks_like_english(text) {
        return "en".to_string();
    }

    if let Some(info) = whatlang::detect(text) {
        if info.is_reliable()
            || text
                .chars()
                .filter(|character| character.is_alphabetic())
                .count()
                >= 24
        {
            return canonical_language_code(info.lang().code()).to_string();
        }
    }

    if text.chars().any(is_han) {
        "zh".to_string()
    } else if text.chars().any(is_arabic) {
        "ar".to_string()
    } else if text.chars().any(is_hebrew) {
        "he".to_string()
    } else if text.chars().any(is_devanagari) {
        "hi".to_string()
    } else if text.chars().any(is_thai) {
        "th".to_string()
    } else if text.chars().any(is_greek) {
        "el".to_string()
    } else if text.chars().any(is_cyrillic) {
        "ru".to_string()
    } else {
        "en".to_string()
    }
}

fn looks_like_english(text: &str) -> bool {
    let words = text
        .split(|character: char| !character.is_ascii_alphabetic())
        .filter(|word| !word.is_empty())
        .map(str::to_ascii_lowercase)
        .collect::<Vec<_>>();
    let strong = [
        "analyze",
        "assess",
        "compare",
        "does",
        "evaluate",
        "explain",
        "how",
        "investigate",
        "research",
        "review",
        "should",
        "what",
        "which",
        "why",
    ];
    if words.iter().any(|word| strong.contains(&word.as_str())) {
        return true;
    }
    let common = [
        "a", "an", "and", "are", "for", "from", "in", "is", "it", "of", "on", "the", "to", "with",
    ];
    words
        .iter()
        .filter(|word| common.contains(&word.as_str()))
        .count()
        >= 2
}

pub(crate) fn validate_deep_research_output_language(language: &str) -> Result<(), String> {
    let language = language.trim();
    if language.is_empty()
        || language.chars().count() > MAX_LANGUAGE_TAG_CHARS
        || language != language.trim()
    {
        return Err(
            "DeepResearch output language must be a bounded BCP 47 language tag".to_string(),
        );
    }
    let mut subtags = language.split('-');
    let Some(primary) = subtags.next() else {
        return Err(
            "DeepResearch output language must be a bounded BCP 47 language tag".to_string(),
        );
    };
    if !(2..=8).contains(&primary.len()) || !primary.bytes().all(|byte| byte.is_ascii_alphabetic())
    {
        return Err(
            "DeepResearch output language must be a bounded BCP 47 language tag".to_string(),
        );
    }
    if subtags.any(|subtag| {
        subtag.is_empty()
            || subtag.len() > 8
            || !subtag.bytes().all(|byte| byte.is_ascii_alphanumeric())
    }) {
        return Err(
            "DeepResearch output language must be a bounded BCP 47 language tag".to_string(),
        );
    }
    Ok(())
}

pub(crate) fn output_language_matches(expected: &str, observed: &str) -> bool {
    expected.eq_ignore_ascii_case(observed)
}

pub(crate) fn primary_output_language(language: &str) -> &str {
    let primary = language.split('-').next().unwrap_or(language);
    match primary.to_ascii_lowercase().as_str() {
        "cmn" | "zho" => "zh",
        "eng" => "en",
        "fra" | "fre" => "fr",
        "deu" | "ger" => "de",
        "spa" => "es",
        "por" => "pt",
        "jpn" => "ja",
        "kor" => "ko",
        "rus" => "ru",
        "ukr" => "uk",
        "ara" => "ar",
        "heb" => "he",
        "hin" => "hi",
        "tha" => "th",
        "ell" | "gre" => "el",
        _ => primary,
    }
}

/// Reject obvious script or language mismatches in model-authored prose.
///
/// This is deliberately an aggregate check. Product names, quotations, and
/// source-defined identifiers may remain in their original language, while
/// the report's surrounding prose must still use the requested language.
pub(crate) fn reader_text_matches_output_language(text: &str, expected: &str) -> bool {
    let primary = primary_output_language(expected);
    let alphabetic_count = text
        .chars()
        .filter(|character| character.is_alphabetic())
        .count();
    if alphabetic_count < 12 || primary == "und" {
        return true;
    }

    let script_count = match primary {
        "zh" => text.chars().filter(|character| is_han(*character)).count(),
        "ja" => text
            .chars()
            .filter(|character| is_han(*character) || is_japanese_kana(*character))
            .count(),
        "ko" => text
            .chars()
            .filter(|character| is_hangul(*character) || is_han(*character))
            .count(),
        "ar" | "fa" | "ur" => text
            .chars()
            .filter(|character| is_arabic(*character))
            .count(),
        "he" => text
            .chars()
            .filter(|character| is_hebrew(*character))
            .count(),
        "hi" | "mr" | "ne" => text
            .chars()
            .filter(|character| is_devanagari(*character))
            .count(),
        "th" => text.chars().filter(|character| is_thai(*character)).count(),
        "el" => text
            .chars()
            .filter(|character| is_greek(*character))
            .count(),
        "ru" | "uk" | "bg" | "sr" | "mk" => text
            .chars()
            .filter(|character| is_cyrillic(*character))
            .count(),
        _ => 0,
    };
    if script_count > 0
        || matches!(
            primary,
            "zh" | "ja"
                | "ko"
                | "ar"
                | "fa"
                | "ur"
                | "he"
                | "hi"
                | "mr"
                | "ne"
                | "th"
                | "el"
                | "ru"
                | "uk"
                | "bg"
                | "sr"
                | "mk"
        )
    {
        return script_count.saturating_mul(5) >= alphabetic_count;
    }

    let Some(info) = whatlang::detect(text) else {
        return true;
    };
    if !info.is_reliable() {
        return true;
    }
    primary_output_language(canonical_language_code(info.lang().code()))
        .eq_ignore_ascii_case(primary)
}

fn canonical_language_code(code: &str) -> &str {
    match code {
        "afr" => "af",
        "ara" => "ar",
        "aze" => "az",
        "bel" => "be",
        "ben" => "bn",
        "bul" => "bg",
        "cat" => "ca",
        "ces" => "cs",
        "cmn" | "zho" => "zh",
        "dan" => "da",
        "deu" => "de",
        "ell" => "el",
        "eng" => "en",
        "epo" => "eo",
        "est" => "et",
        "fas" => "fa",
        "fin" => "fi",
        "fra" => "fr",
        "guj" => "gu",
        "heb" => "he",
        "hin" => "hi",
        "hrv" => "hr",
        "hun" => "hu",
        "ind" => "id",
        "ita" => "it",
        "jpn" => "ja",
        "kan" => "kn",
        "kor" => "ko",
        "lav" => "lv",
        "lit" => "lt",
        "mal" => "ml",
        "mar" => "mr",
        "mkd" => "mk",
        "nld" => "nl",
        "nob" => "nb",
        "pan" => "pa",
        "pol" => "pl",
        "por" => "pt",
        "ron" => "ro",
        "rus" => "ru",
        "slk" => "sk",
        "slv" => "sl",
        "spa" => "es",
        "srp" => "sr",
        "swe" => "sv",
        "swa" => "sw",
        "tam" => "ta",
        "tel" => "te",
        "tha" => "th",
        "tur" => "tr",
        "ukr" => "uk",
        "urd" => "ur",
        "vie" => "vi",
        "zul" => "zu",
        _ => code,
    }
}

fn is_han(character: char) -> bool {
    matches!(
        character as u32,
        0x3400..=0x4DBF
            | 0x4E00..=0x9FFF
            | 0xF900..=0xFAFF
            | 0x20000..=0x2FA1F
    )
}

fn is_japanese_kana(character: char) -> bool {
    matches!(character as u32, 0x3040..=0x30FF | 0x31F0..=0x31FF)
}

fn is_hangul(character: char) -> bool {
    matches!(character as u32, 0x1100..=0x11FF | 0x3130..=0x318F | 0xAC00..=0xD7AF)
}

fn is_arabic(character: char) -> bool {
    matches!(character as u32, 0x0600..=0x06FF | 0x0750..=0x077F | 0x08A0..=0x08FF)
}

fn is_hebrew(character: char) -> bool {
    matches!(character as u32, 0x0590..=0x05FF)
}

fn is_devanagari(character: char) -> bool {
    matches!(character as u32, 0x0900..=0x097F)
}

fn is_thai(character: char) -> bool {
    matches!(character as u32, 0x0E00..=0x0E7F)
}

fn is_greek(character: char) -> bool {
    matches!(character as u32, 0x0370..=0x03FF | 0x1F00..=0x1FFF)
}

fn is_cyrillic(character: char) -> bool {
    matches!(character as u32, 0x0400..=0x052F | 0x2DE0..=0x2DFF | 0xA640..=0xA69F)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infers_common_user_languages_without_using_source_text() {
        assert_eq!(
            infer_deep_research_output_language("比较两个深度研究实现"),
            "zh"
        );
        assert_eq!(
            infer_deep_research_output_language(
                "基于一手资料，比较 Stanford STORM、OpenAI deep research 与 A3S DeepResearch 的方法与局限。"
            ),
            "zh"
        );
        assert_eq!(
            infer_deep_research_output_language(
                "Compare the terms 中文模型 and English model in this document"
            ),
            "en"
        );
        assert_eq!(
            infer_deep_research_output_language("Compare two research implementations"),
            "en"
        );
        assert_eq!(
            infer_deep_research_output_language("Comparer deux implémentations de recherche"),
            "fr"
        );
        assert_eq!(
            infer_deep_research_output_language("¿Qué demuestra la evidencia?"),
            "es"
        );
    }

    #[test]
    fn detects_obvious_reader_language_mismatches() {
        assert!(reader_text_matches_output_language(
            "结论说明了证据、分析过程以及仍未解决的边界。",
            "zh"
        ));
        assert!(!reader_text_matches_output_language(
            "This conclusion and its surrounding analysis are written in English.",
            "zh"
        ));
    }
}
