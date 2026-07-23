use std::collections::HashSet;

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
