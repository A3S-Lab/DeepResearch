const RESEARCH_REPORT_PAIR_TRANSACTION_FILE: &str = ".a3s-report-pair-transaction.json";
const RESEARCH_REPORT_PAIR_TRANSACTION_VERSION: u8 = 1;
const MAX_RESEARCH_REPORT_TRANSACTION_BYTES: u64 = 4 * 1024 * 1024;
const MAX_RESEARCH_REPORT_TRANSACTION_JOURNAL_BYTES: u64 = 64 * 1024;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
struct ResearchReportPairTransaction {
    version: u8,
    markdown_name: String,
    html_name: String,
    staged_markdown_name: String,
    staged_html_name: String,
    previous_markdown_name: Option<String>,
    previous_html_name: Option<String>,
    new_markdown_sha256: String,
    new_html_sha256: String,
    previous_markdown_sha256: Option<String>,
    previous_html_sha256: Option<String>,
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ResearchReportPairInterruption {
    AfterMarkdownReplacement,
    AfterHtmlReplacement,
}

fn research_report_file_name(path: &Path) -> Result<String, String> {
    path.file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("invalid DeepResearch artifact name: {}", path.display()))
}

fn research_report_pair_parent(
    markdown_path: &Path,
    html_path: &Path,
) -> Result<PathBuf, String> {
    let markdown_parent = markdown_path.parent().ok_or_else(|| {
        format!(
            "DeepResearch artifact has no parent: {}",
            markdown_path.display()
        )
    })?;
    let html_parent = html_path.parent().ok_or_else(|| {
        format!(
            "DeepResearch artifact has no parent: {}",
            html_path.display()
        )
    })?;
    if markdown_parent != html_parent || markdown_path == html_path {
        return Err(
            "DeepResearch Markdown and HTML artifacts must be distinct files in one directory"
                .to_string(),
        );
    }
    Ok(markdown_parent.to_path_buf())
}

fn research_report_sha256(contents: &[u8]) -> String {
    use sha2::{Digest, Sha256};

    format!("{:x}", Sha256::digest(contents))
}

fn existing_research_report_file(path: &Path) -> Result<Option<Vec<u8>>, String> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) => {
            validate_research_report_file_target(path)?;
            if metadata.len() > MAX_RESEARCH_REPORT_TRANSACTION_BYTES {
                return Err(format!(
                    "DeepResearch artifact exceeds the transaction byte limit: {}",
                    path.display()
                ));
            }
            std::fs::read(path)
                .map(Some)
                .map_err(|error| format!("could not preserve {}: {error}", path.display()))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("could not inspect {}: {error}", path.display())),
    }
}

fn research_report_file_sha256(path: &Path) -> Result<Option<String>, String> {
    existing_research_report_file(path)
        .map(|contents| contents.map(|contents| research_report_sha256(&contents)))
}

fn sync_research_report_directory(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        std::fs::File::open(path)
            .and_then(|directory| directory.sync_all())
            .map_err(|error| {
                format!(
                    "could not sync DeepResearch artifact directory {}: {error}",
                    path.display()
                )
            })?;
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

fn remove_plain_research_report_file(path: &Path) -> Result<(), String> {
    match std::fs::symlink_metadata(path) {
        Ok(_) => {
            validate_research_report_file_target(path)?;
            std::fs::remove_file(path)
                .map_err(|error| format!("could not remove {}: {error}", path.display()))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("could not inspect {}: {error}", path.display())),
    }
}

fn transaction_auxiliary_path(
    parent: &Path,
    name: &str,
    target_name: &str,
) -> Result<PathBuf, String> {
    let prefix = format!(".{target_name}.");
    if name.is_empty()
        || name.contains('/')
        || name.contains('\\')
        || !name.starts_with(&prefix)
        || !name.ends_with(".tmp")
        || Path::new(name).file_name().and_then(|value| value.to_str()) != Some(name)
    {
        return Err("invalid DeepResearch report transaction file name".to_string());
    }
    Ok(parent.join(name))
}

fn valid_research_report_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn validate_research_report_transaction(
    transaction: &ResearchReportPairTransaction,
    markdown_path: &Path,
    html_path: &Path,
) -> Result<(), String> {
    if transaction.version != RESEARCH_REPORT_PAIR_TRANSACTION_VERSION
        || transaction.markdown_name != research_report_file_name(markdown_path)?
        || transaction.html_name != research_report_file_name(html_path)?
        || transaction.markdown_name == transaction.html_name
    {
        return Err("invalid DeepResearch report transaction identity".to_string());
    }
    if !valid_research_report_sha256(&transaction.new_markdown_sha256)
        || !valid_research_report_sha256(&transaction.new_html_sha256)
        || transaction
            .previous_markdown_sha256
            .as_deref()
            .is_some_and(|value| !valid_research_report_sha256(value))
        || transaction
            .previous_html_sha256
            .as_deref()
            .is_some_and(|value| !valid_research_report_sha256(value))
        || (transaction.previous_markdown_name.is_some()
            != transaction.previous_markdown_sha256.is_some())
        || (transaction.previous_html_name.is_some()
            != transaction.previous_html_sha256.is_some())
    {
        return Err("invalid DeepResearch report transaction digest".to_string());
    }

    let parent = research_report_pair_parent(markdown_path, html_path)?;
    let mut names = HashSet::new();
    for (name, target_name) in [
        (
            Some(transaction.staged_markdown_name.as_str()),
            transaction.markdown_name.as_str(),
        ),
        (
            Some(transaction.staged_html_name.as_str()),
            transaction.html_name.as_str(),
        ),
        (
            transaction.previous_markdown_name.as_deref(),
            transaction.markdown_name.as_str(),
        ),
        (
            transaction.previous_html_name.as_deref(),
            transaction.html_name.as_str(),
        ),
    ] {
        let Some(name) = name else {
            continue;
        };
        transaction_auxiliary_path(&parent, name, target_name)?;
        if !names.insert(name) {
            return Err("duplicate DeepResearch report transaction file".to_string());
        }
    }
    Ok(())
}

fn research_report_pair_transaction_path(parent: &Path) -> PathBuf {
    parent.join(RESEARCH_REPORT_PAIR_TRANSACTION_FILE)
}

fn read_research_report_pair_transaction(
    markdown_path: &Path,
    html_path: &Path,
) -> Result<Option<ResearchReportPairTransaction>, String> {
    let parent = research_report_pair_parent(markdown_path, html_path)?;
    let transaction_path = research_report_pair_transaction_path(&parent);
    let metadata = match std::fs::symlink_metadata(&transaction_path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(format!(
                "could not inspect DeepResearch report transaction {}: {error}",
                transaction_path.display()
            ));
        }
    };
    validate_research_report_file_target(&transaction_path)?;
    if metadata.len() == 0 || metadata.len() > MAX_RESEARCH_REPORT_TRANSACTION_JOURNAL_BYTES {
        return Err("invalid DeepResearch report transaction size".to_string());
    }
    let bytes = std::fs::read(&transaction_path).map_err(|error| {
        format!(
            "could not read DeepResearch report transaction {}: {error}",
            transaction_path.display()
        )
    })?;
    let transaction =
        serde_json::from_slice::<ResearchReportPairTransaction>(&bytes).map_err(|error| {
            format!(
                "could not decode DeepResearch report transaction {}: {error}",
                transaction_path.display()
            )
        })?;
    validate_research_report_transaction(&transaction, markdown_path, html_path)?;
    Ok(Some(transaction))
}

fn remove_research_report_transaction_files(
    transaction: &ResearchReportPairTransaction,
    markdown_path: &Path,
    html_path: &Path,
) -> Result<(), String> {
    validate_research_report_transaction(transaction, markdown_path, html_path)?;
    let parent = research_report_pair_parent(markdown_path, html_path)?;
    for (name, target_name) in [
        (
            Some(transaction.staged_markdown_name.as_str()),
            transaction.markdown_name.as_str(),
        ),
        (
            Some(transaction.staged_html_name.as_str()),
            transaction.html_name.as_str(),
        ),
        (
            transaction.previous_markdown_name.as_deref(),
            transaction.markdown_name.as_str(),
        ),
        (
            transaction.previous_html_name.as_deref(),
            transaction.html_name.as_str(),
        ),
    ] {
        if let Some(name) = name {
            remove_plain_research_report_file(&transaction_auxiliary_path(
                &parent,
                name,
                target_name,
            )?)?;
        }
    }
    remove_plain_research_report_file(&research_report_pair_transaction_path(&parent))?;
    sync_research_report_directory(&parent)
}

fn restore_research_report_transaction_target(
    path: &Path,
    previous_name: Option<&str>,
    previous_sha256: Option<&str>,
    parent: &Path,
) -> Result<(), String> {
    let current_sha256 = research_report_file_sha256(path)?;
    match previous_sha256 {
        None => {
            if current_sha256.is_some() {
                remove_plain_research_report_file(path)?;
            }
        }
        Some(expected) if current_sha256.as_deref() == Some(expected) => {}
        Some(expected) => {
            let previous_name = previous_name.ok_or_else(|| {
                "DeepResearch report transaction omitted its previous file".to_string()
            })?;
            let target_name = research_report_file_name(path)?;
            let previous =
                transaction_auxiliary_path(parent, previous_name, target_name.as_str())?;
            if research_report_file_sha256(&previous)?.as_deref() != Some(expected) {
                return Err(format!(
                    "DeepResearch report transaction cannot restore {}",
                    path.display()
                ));
            }
            validate_research_report_file_target(path)?;
            replace_staged_research_report_file(&previous, path)?;
        }
    }
    Ok(())
}

fn recover_research_report_pair(markdown_path: &Path, html_path: &Path) -> Result<(), String> {
    validate_research_report_file_target(markdown_path)?;
    validate_research_report_file_target(html_path)?;
    let Some(transaction) =
        read_research_report_pair_transaction(markdown_path, html_path)?
    else {
        return Ok(());
    };
    let parent = research_report_pair_parent(markdown_path, html_path)?;
    let markdown_sha256 = research_report_file_sha256(markdown_path)?;
    let html_sha256 = research_report_file_sha256(html_path)?;
    let fully_replaced = markdown_sha256.as_deref()
        == Some(transaction.new_markdown_sha256.as_str())
        && html_sha256.as_deref() == Some(transaction.new_html_sha256.as_str());

    if !fully_replaced {
        restore_research_report_transaction_target(
            markdown_path,
            transaction.previous_markdown_name.as_deref(),
            transaction.previous_markdown_sha256.as_deref(),
            &parent,
        )?;
        restore_research_report_transaction_target(
            html_path,
            transaction.previous_html_name.as_deref(),
            transaction.previous_html_sha256.as_deref(),
            &parent,
        )?;
        if research_report_file_sha256(markdown_path)?
            != transaction.previous_markdown_sha256
            || research_report_file_sha256(html_path)? != transaction.previous_html_sha256
        {
            return Err("DeepResearch report transaction rollback did not converge".to_string());
        }
    }

    sync_research_report_directory(&parent)?;
    remove_research_report_transaction_files(&transaction, markdown_path, html_path)
}

fn remove_uncommitted_research_report_files(paths: &[PathBuf]) {
    for path in paths {
        let _ = remove_plain_research_report_file(path);
    }
}

fn stage_tracked_research_report_file(
    path: &Path,
    contents: &[u8],
    allocated: &mut Vec<PathBuf>,
) -> Result<PathBuf, String> {
    let staged = stage_research_report_file(path, contents)?;
    allocated.push(staged.clone());
    Ok(staged)
}

fn stage_research_report_pair_transaction(
    markdown_path: &Path,
    markdown: &[u8],
    html_path: &Path,
    html: &[u8],
) -> Result<ResearchReportPairTransaction, String> {
    if markdown.len() as u64 > MAX_RESEARCH_REPORT_TRANSACTION_BYTES
        || html.len() as u64 > MAX_RESEARCH_REPORT_TRANSACTION_BYTES
    {
        return Err("DeepResearch report pair exceeds the transaction byte limit".to_string());
    }
    let parent = research_report_pair_parent(markdown_path, html_path)?;
    recover_research_report_pair(markdown_path, html_path)?;
    validate_research_report_file_target(markdown_path)?;
    validate_research_report_file_target(html_path)?;
    let markdown_name = research_report_file_name(markdown_path)?;
    let html_name = research_report_file_name(html_path)?;
    let previous_markdown = existing_research_report_file(markdown_path)?;
    let previous_html = existing_research_report_file(html_path)?;
    let mut allocated = Vec::new();

    let transaction_result = (|| -> Result<ResearchReportPairTransaction, String> {
        let staged_markdown =
            stage_tracked_research_report_file(markdown_path, markdown, &mut allocated)?;
        let staged_html =
            stage_tracked_research_report_file(html_path, html, &mut allocated)?;
        let previous_markdown_path = previous_markdown
            .as_deref()
            .map(|contents| {
                stage_tracked_research_report_file(markdown_path, contents, &mut allocated)
            })
            .transpose()?;
        let previous_html_path = previous_html
            .as_deref()
            .map(|contents| {
                stage_tracked_research_report_file(html_path, contents, &mut allocated)
            })
            .transpose()?;
        let transaction = ResearchReportPairTransaction {
            version: RESEARCH_REPORT_PAIR_TRANSACTION_VERSION,
            markdown_name,
            html_name,
            staged_markdown_name: research_report_file_name(&staged_markdown)?,
            staged_html_name: research_report_file_name(&staged_html)?,
            previous_markdown_name: previous_markdown_path
                .as_deref()
                .map(research_report_file_name)
                .transpose()?,
            previous_html_name: previous_html_path
                .as_deref()
                .map(research_report_file_name)
                .transpose()?,
            new_markdown_sha256: research_report_sha256(markdown),
            new_html_sha256: research_report_sha256(html),
            previous_markdown_sha256: previous_markdown
                .as_deref()
                .map(research_report_sha256),
            previous_html_sha256: previous_html.as_deref().map(research_report_sha256),
        };
        validate_research_report_transaction(&transaction, markdown_path, html_path)?;
        Ok(transaction)
    })();
    let transaction = match transaction_result {
        Ok(transaction) => transaction,
        Err(error) => {
            remove_uncommitted_research_report_files(&allocated);
            return Err(error);
        }
    };
    let transaction_bytes = match serde_json::to_vec(&transaction) {
        Ok(bytes) => bytes,
        Err(error) => {
            remove_uncommitted_research_report_files(&allocated);
            return Err(format!(
                "could not encode DeepResearch report transaction: {error}"
            ));
        }
    };
    let transaction_path = research_report_pair_transaction_path(&parent);
    if let Err(error) = write_research_report_file(&transaction_path, &transaction_bytes) {
        remove_uncommitted_research_report_files(&allocated);
        return Err(error);
    }
    if let Err(error) = sync_research_report_directory(&parent) {
        let recovery = recover_research_report_pair(markdown_path, html_path);
        return match recovery {
            Ok(()) => Err(error),
            Err(recovery) => Err(format!(
                "{error}; DeepResearch report transaction recovery failed: {recovery}"
            )),
        };
    }
    Ok(transaction)
}

fn replace_research_report_transaction_file(
    parent: &Path,
    staged_name: &str,
    target_name: &str,
    target: &Path,
) -> Result<(), String> {
    let staged = transaction_auxiliary_path(parent, staged_name, target_name)?;
    replace_staged_research_report_file(&staged, target)
}

fn research_report_transaction_error(
    error: String,
    markdown_path: &Path,
    html_path: &Path,
) -> Result<(), String> {
    match recover_research_report_pair(markdown_path, html_path) {
        Ok(()) => Err(error),
        Err(recovery) => Err(format!(
            "{error}; DeepResearch report transaction recovery failed: {recovery}"
        )),
    }
}

fn write_research_report_pair(
    markdown_path: &Path,
    markdown: impl AsRef<[u8]>,
    html_path: &Path,
    html: impl AsRef<[u8]>,
) -> Result<(), String> {
    let transaction = stage_research_report_pair_transaction(
        markdown_path,
        markdown.as_ref(),
        html_path,
        html.as_ref(),
    )?;
    let parent = research_report_pair_parent(markdown_path, html_path)?;

    if let Err(error) = replace_research_report_transaction_file(
        &parent,
        &transaction.staged_markdown_name,
        &transaction.markdown_name,
        markdown_path,
    ) {
        return research_report_transaction_error(error, markdown_path, html_path);
    }
    if let Err(error) = replace_research_report_transaction_file(
        &parent,
        &transaction.staged_html_name,
        &transaction.html_name,
        html_path,
    ) {
        return research_report_transaction_error(error, markdown_path, html_path);
    }
    if let Err(error) = sync_research_report_directory(&parent) {
        return research_report_transaction_error(error, markdown_path, html_path);
    }
    recover_research_report_pair(markdown_path, html_path)
}

#[cfg(test)]
fn simulate_research_report_pair_interruption_for_test(
    markdown_path: &Path,
    markdown: impl AsRef<[u8]>,
    html_path: &Path,
    html: impl AsRef<[u8]>,
    interruption: ResearchReportPairInterruption,
) -> Result<(), String> {
    let transaction = stage_research_report_pair_transaction(
        markdown_path,
        markdown.as_ref(),
        html_path,
        html.as_ref(),
    )?;
    let parent = research_report_pair_parent(markdown_path, html_path)?;
    replace_research_report_transaction_file(
        &parent,
        &transaction.staged_markdown_name,
        &transaction.markdown_name,
        markdown_path,
    )?;
    if interruption == ResearchReportPairInterruption::AfterHtmlReplacement {
        replace_research_report_transaction_file(
            &parent,
            &transaction.staged_html_name,
            &transaction.html_name,
            html_path,
        )?;
    }
    sync_research_report_directory(&parent)
}
