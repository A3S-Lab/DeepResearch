fn catalog_source_matches_query(query: &str, title: &str, chunks: &[String]) -> bool {
    let features = source_backed_query_features(query);
    if features.is_empty() {
        return false;
    }
    let haystack = format!("{title} {}", chunks.join(" ")).to_lowercase();
    features
        .iter()
        .any(|feature| haystack.contains(feature.as_str()))
}

fn source_backed_query_features(query: &str) -> Vec<String> {
    let mut features = HashSet::new();
    let mut token = String::new();
    let mut han = String::new();
    let flush_token = |token: &mut String, features: &mut HashSet<String>| {
        let normalized = token.to_lowercase();
        if normalized.chars().count() >= 3
            && !matches!(
                normalized.as_str(),
                "the"
                    | "and"
                    | "for"
                    | "with"
                    | "from"
                    | "into"
                    | "which"
                    | "what"
                    | "when"
                    | "where"
                    | "who"
                    | "why"
                    | "how"
                    | "are"
                    | "was"
                    | "were"
                    | "this"
                    | "that"
            )
        {
            features.insert(normalized);
        }
        token.clear();
    };
    let flush_han = |han: &mut String, features: &mut HashSet<String>| {
        let characters = han.chars().collect::<Vec<_>>();
        for pair in characters.windows(2) {
            features.insert(pair.iter().collect());
        }
        han.clear();
    };
    for character in query.chars() {
        if source_backed_han_character(character) {
            flush_token(&mut token, &mut features);
            han.push(character);
        } else if character.is_alphanumeric() {
            flush_han(&mut han, &mut features);
            token.push(character);
        } else {
            flush_token(&mut token, &mut features);
            flush_han(&mut han, &mut features);
        }
    }
    flush_token(&mut token, &mut features);
    flush_han(&mut han, &mut features);
    let mut features = features.into_iter().collect::<Vec<_>>();
    features.sort();
    features
}

fn source_backed_han_character(character: char) -> bool {
    matches!(
        character as u32,
        0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF | 0x20000..=0x2FA1F
    )
}
