use std::collections::HashSet;
use std::path::{Component, Path};

use a3s_acl::{Block, Value};
use sha2::{Digest, Sha256};

#[test]
fn evaluation_manifests_are_valid_acl_with_unique_cases() {
    for (name, source, expected_cases) in [
        (
            "frozen",
            include_str!("fixtures/deep_research_eval/frozen.acl"),
            8,
        ),
        (
            "live",
            include_str!("fixtures/deep_research_eval/live.acl"),
            9,
        ),
    ] {
        let document = a3s_acl::parse_acl(source)
            .unwrap_or_else(|error| panic!("parse {name} evaluation manifest: {error}"));
        let corpus = document
            .blocks
            .first()
            .unwrap_or_else(|| panic!("{name} evaluation manifest omitted its corpus"));
        assert_eq!(corpus.name, "corpus");

        let case_ids = corpus
            .blocks
            .iter()
            .filter(|block| block.name == "case")
            .map(|block| {
                block
                    .labels
                    .first()
                    .cloned()
                    .unwrap_or_else(|| panic!("{name} evaluation case omitted its ID"))
            })
            .collect::<Vec<_>>();
        assert_eq!(case_ids.len(), expected_cases);
        assert_eq!(
            case_ids.iter().collect::<HashSet<_>>().len(),
            case_ids.len(),
            "{name} evaluation case IDs must be unique"
        );
    }
}

#[test]
fn frozen_corpus_source_digests_match_the_declared_immutable_snapshots() {
    let fixture_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("deep_research_eval");
    let source = include_str!("fixtures/deep_research_eval/frozen.acl");
    let document = a3s_acl::parse_acl(source).expect("parse frozen evaluation manifest");
    let corpus = document.blocks.first().expect("frozen corpus block");
    let mut observed_paths = HashSet::new();

    for case in corpus.blocks.iter().filter(|block| block.name == "case") {
        let case_id = case.labels.first().expect("frozen case ID");
        for source in case.blocks.iter().filter(|block| block.name == "source") {
            let source_id = source.labels.first().expect("frozen source ID");
            let relative = required_string(source, "path");
            let path = Path::new(relative);
            assert!(
                !path.is_absolute()
                    && path
                        .components()
                        .all(|component| matches!(component, Component::Normal(_))),
                "{case_id}/{source_id}: source path must be a closed relative path"
            );
            assert!(
                observed_paths.insert(relative),
                "{case_id}/{source_id}: duplicate frozen source path `{relative}`"
            );

            let bytes = std::fs::read(fixture_root.join(path)).unwrap_or_else(|error| {
                panic!("{case_id}/{source_id}: read `{relative}`: {error}")
            });
            let observed = format!("{:x}", Sha256::digest(bytes));
            assert_eq!(
                observed,
                required_string(source, "sha256"),
                "{case_id}/{source_id}: frozen source digest changed"
            );
        }
    }
}

fn required_string<'a>(block: &'a Block, key: &str) -> &'a str {
    block
        .attributes
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("{} {:?} omitted string `{key}`", block.name, block.labels))
}
