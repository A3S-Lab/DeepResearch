use super::super::{ReaderLabels, ReportDocument, ReportSource, StructuralCoverage};
use std::collections::BTreeMap;

pub(super) struct RenderContext<'a> {
    pub(super) document: &'a ReportDocument,
    pub(super) labels: &'a ReaderLabels,
    claim_numbers: BTreeMap<&'a str, usize>,
}

impl<'a> RenderContext<'a> {
    pub(super) fn new(document: &'a ReportDocument) -> Self {
        let claim_numbers = document
            .direct_answer_claims
            .iter()
            .chain(
                document
                    .dimensions
                    .iter()
                    .flat_map(|dimension| dimension.claims.iter()),
            )
            .enumerate()
            .map(|(index, claim)| (claim.id.as_str(), index + 1))
            .collect();
        Self {
            document,
            labels: &document.reader_labels,
            claim_numbers,
        }
    }

    pub(super) fn claim_number(&self, claim_id: &str) -> Option<usize> {
        self.claim_numbers.get(claim_id).copied()
    }

    pub(super) fn source(&self, source_id: &str) -> Option<&'a ReportSource> {
        self.document
            .source_ledger
            .iter()
            .find(|source| source.id == source_id)
    }

    pub(super) fn coverage_label(&self, coverage: StructuralCoverage) -> &str {
        match coverage {
            StructuralCoverage::ClaimsOnly => &self.labels.coverage_claims,
            StructuralCoverage::ClaimsAndGap => &self.labels.coverage_partial,
            StructuralCoverage::GapOnly => &self.labels.coverage_bounded,
            StructuralCoverage::Missing => &self.labels.coverage_missing,
        }
    }
}
