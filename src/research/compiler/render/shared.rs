use super::super::{ReaderLabels, ReportClaim, ReportDocument, ReportSource, StructuralCoverage};
use std::collections::{BTreeMap, BTreeSet};

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

    pub(super) fn narrative_paragraphs<'b>(
        &self,
        claims: &'b [ReportClaim],
        paragraph_claim_ids: &[Vec<String>],
    ) -> Vec<Vec<&'b ReportClaim>> {
        let claims_by_id = claims
            .iter()
            .map(|claim| (claim.id.as_str(), claim))
            .collect::<BTreeMap<_, _>>();
        let mut seen = BTreeSet::<&str>::new();
        let mut paragraphs = paragraph_claim_ids
            .iter()
            .filter_map(|claim_ids| {
                let paragraph = claim_ids
                    .iter()
                    .filter_map(|claim_id| {
                        let claim = claims_by_id.get(claim_id.as_str()).copied()?;
                        seen.insert(claim.id.as_str()).then_some(claim)
                    })
                    .collect::<Vec<_>>();
                (!paragraph.is_empty()).then_some(paragraph)
            })
            .collect::<Vec<_>>();
        paragraphs.extend(
            claims
                .iter()
                .filter(|claim| seen.insert(claim.id.as_str()))
                .map(|claim| vec![claim]),
        );
        paragraphs
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
