fn catalog_source_claim_eligible(
    anchor: &str,
    semantic_source_admission: bool,
) -> bool {
    semantic_source_admission || deterministic_fallback_claim_anchor(anchor)
}

/// A failed semantic admission cannot turn arbitrary search rank into source
/// authority. Web fallback sources remain visible for audit, while workspace
/// evidence may support a claim without web-source admission. The Host never
/// infers authority from source prose, query words, publisher names, TLDs, or
/// a maintained domain list.
fn deterministic_fallback_claim_anchor(anchor: &str) -> bool {
    !anchor.starts_with("http://") && !anchor.starts_with("https://")
}

fn catalog_source_latest_observed_date(
    source: &DeepResearchCatalogSource,
) -> Option<chrono::NaiveDate> {
    source
        .chunks
        .iter()
        .flat_map(|chunk| catalog_observed_dates(chunk))
        .max()
}

fn catalog_source_is_temporal_snapshot(source: &DeepResearchCatalogSource) -> bool {
    let text = source.chunks.join(" ").to_ascii_lowercase();
    [
        "截至目前",
        "截至当时",
        "截至",
        "当时",
        "as of",
        "at the time of writing",
        "at that point",
    ]
    .iter()
    .any(|marker| text.contains(marker))
}

fn catalog_observed_dates(value: &str) -> Vec<chrono::NaiveDate> {
    static NUMERIC_DATE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    static CHINESE_DATE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    static CHINESE_RANGE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    static COMPACT_PATH_DATE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    static NAMED_MONTH_DATE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let numeric_date = NUMERIC_DATE.get_or_init(|| {
        regex::Regex::new(
            r"(?:^|[^0-9])(?P<year>20[0-9]{2})[-/](?P<month>[0-9]{1,2})[-/](?P<day>[0-9]{1,2})(?:$|[^0-9])",
        )
        .expect("valid numeric date regex")
    });
    let chinese_date = CHINESE_DATE.get_or_init(|| {
        regex::Regex::new(
            r"(?P<year>20[0-9]{2})\s*年\s*(?P<month>[0-9]{1,2})\s*月\s*(?P<day>[0-9]{1,2})\s*日",
        )
        .expect("valid Chinese date regex")
    });
    let chinese_range = CHINESE_RANGE.get_or_init(|| {
        regex::Regex::new(
            r"(?P<year>20[0-9]{2})\s*年\s*[0-9]{1,2}\s*月\s*[0-9]{1,2}\s*日\s*(?:至|到|—|–|-)\s*(?P<month>[0-9]{1,2})\s*月\s*(?P<day>[0-9]{1,2})\s*日",
        )
        .expect("valid Chinese date-range regex")
    });
    let compact_path_date = COMPACT_PATH_DATE.get_or_init(|| {
        regex::Regex::new(
            r"(?:^|/)(?P<year>20[0-9]{2})/(?P<month>[01][0-9])(?P<day>[0-3][0-9])(?:/|$)",
        )
        .expect("valid compact path date regex")
    });
    let named_month_date = NAMED_MONTH_DATE.get_or_init(|| {
        regex::Regex::new(
            r"(?i)\b(?P<month>January|February|March|April|May|June|July|August|September|October|November|December)\s+(?P<day>[0-9]{1,2})(?:st|nd|rd|th)?[,]?\s+(?P<year>20[0-9]{2})\b",
        )
        .expect("valid named-month date regex")
    });
    let mut dates = Vec::new();
    for captures in numeric_date.captures_iter(value) {
        push_catalog_numeric_date(&mut dates, &captures);
    }
    for captures in chinese_date.captures_iter(value) {
        push_catalog_numeric_date(&mut dates, &captures);
    }
    for captures in chinese_range.captures_iter(value) {
        push_catalog_numeric_date(&mut dates, &captures);
    }
    for captures in compact_path_date.captures_iter(value) {
        push_catalog_numeric_date(&mut dates, &captures);
    }
    for captures in named_month_date.captures_iter(value) {
        let month = match captures
            .name("month")
            .map(|value| value.as_str().to_ascii_lowercase())
            .as_deref()
        {
            Some("january") => 1,
            Some("february") => 2,
            Some("march") => 3,
            Some("april") => 4,
            Some("may") => 5,
            Some("june") => 6,
            Some("july") => 7,
            Some("august") => 8,
            Some("september") => 9,
            Some("october") => 10,
            Some("november") => 11,
            Some("december") => 12,
            _ => continue,
        };
        push_catalog_date_parts(&mut dates, &captures, month);
    }
    dates.sort_unstable();
    dates.dedup();
    dates
}

fn push_catalog_numeric_date(dates: &mut Vec<chrono::NaiveDate>, captures: &regex::Captures<'_>) {
    let Some(month) = captures
        .name("month")
        .and_then(|value| value.as_str().parse::<u32>().ok())
    else {
        return;
    };
    push_catalog_date_parts(dates, captures, month);
}

fn push_catalog_date_parts(
    dates: &mut Vec<chrono::NaiveDate>,
    captures: &regex::Captures<'_>,
    month: u32,
) {
    let Some(year) = captures
        .name("year")
        .and_then(|value| value.as_str().parse::<i32>().ok())
    else {
        return;
    };
    let Some(day) = captures
        .name("day")
        .and_then(|value| value.as_str().parse::<u32>().ok())
    else {
        return;
    };
    if let Some(date) = chrono::NaiveDate::from_ymd_opt(year, month, day) {
        dates.push(date);
    }
}
