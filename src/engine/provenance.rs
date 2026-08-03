//! Adapter-asserted, audit-only retrieval receipt bindings.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Conventionally reserved top-level
/// [`WorkflowOutput`](super::WorkflowOutput) metadata key.
///
/// This is not a cryptographically separate channel. Product adapters must
/// prevent untrusted retrieval tools and workflow-generated JSON from being
/// copied here without source-receipt validation.
pub const RETRIEVAL_RUN_PROVENANCE_METADATA_KEY: &str = "retrieval_run_provenance";

/// Frozen schema for a bounded set of retrieval receipt identities.
pub const RETRIEVAL_RUN_PROVENANCE_V1_SCHEMA: &str =
    "a3s/deep-research-retrieval-run-provenance/v1";

const RETRIEVAL_RUN_PROVENANCE_V1_DOMAIN: &[u8] =
    b"a3s/deep-research-retrieval-run-provenance/v1\0";
const MAX_RECEIPT_SCHEMA_BYTES: usize = 160;
const MAX_PROVENANCE_BINDINGS: usize = 64;

/// Audit identity for one already validated retrieval receipt.
///
/// `request_sha256` and `output_sha256` are generic producer-owned identities.
/// Their exact coverage is defined by `receipt_schema`. A Search cascade
/// adapter maps its complete typed `SearchQuery` binding to the former and its
/// complete ordered `SearchResults` binding to the latter; its quality floor
/// and tier plan remain covered by `receipt_sha256`. DeepResearch never
/// interprets these digests as source validity, evidence coverage, or report
/// quality.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct RetrievalRunProvenanceBindingV1 {
    /// Producer-declared frozen receipt schema.
    pub receipt_schema: String,
    /// Producer-defined canonical SHA-256 identity of the complete receipt.
    pub receipt_sha256: String,
    /// Producer-defined SHA-256 identity of the request or typed input.
    pub request_sha256: String,
    /// SHA-256 identity of the complete ordered retrieval output.
    pub output_sha256: String,
}

impl RetrievalRunProvenanceBindingV1 {
    /// Creates and validates one producer-neutral receipt binding.
    ///
    /// This validates the binding shape only. The product adapter must first
    /// validate the source receipt against its returned output and retain the
    /// complete receipt plus the producer-specific verification material.
    pub fn new(
        receipt_schema: impl Into<String>,
        receipt_sha256: impl Into<String>,
        request_sha256: impl Into<String>,
        output_sha256: impl Into<String>,
    ) -> Result<Self, RetrievalRunProvenanceError> {
        let binding = Self {
            receipt_schema: receipt_schema.into(),
            receipt_sha256: receipt_sha256.into(),
            request_sha256: request_sha256.into(),
            output_sha256: output_sha256.into(),
        };
        binding.validate(0)?;
        Ok(binding)
    }

    fn validate(&self, index: usize) -> Result<(), RetrievalRunProvenanceError> {
        if !valid_receipt_schema(&self.receipt_schema) {
            return Err(RetrievalRunProvenanceError::InvalidReceiptSchema { index });
        }
        for (field, digest) in [
            ("receipt_sha256", self.receipt_sha256.as_str()),
            ("request_sha256", self.request_sha256.as_str()),
            ("output_sha256", self.output_sha256.as_str()),
        ] {
            if !is_canonical_sha256(digest) {
                return Err(RetrievalRunProvenanceError::InvalidDigest { index, field });
            }
        }
        Ok(())
    }
}

/// Bounded, ordered retrieval receipt identities asserted by an adapter.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct RetrievalRunProvenanceEnvelopeV1 {
    /// Exact DeepResearch envelope schema.
    pub schema: String,
    /// Receipt identities in retrieval execution order.
    pub bindings: Vec<RetrievalRunProvenanceBindingV1>,
}

impl RetrievalRunProvenanceEnvelopeV1 {
    /// Creates a non-empty bounded provenance envelope.
    pub fn new(
        bindings: Vec<RetrievalRunProvenanceBindingV1>,
    ) -> Result<Self, RetrievalRunProvenanceError> {
        let envelope = Self {
            schema: RETRIEVAL_RUN_PROVENANCE_V1_SCHEMA.to_string(),
            bindings,
        };
        envelope.validate()?;
        Ok(envelope)
    }

    /// Validates the frozen schema, bounds, identities, and uniqueness.
    pub fn validate(&self) -> Result<(), RetrievalRunProvenanceError> {
        if self.schema != RETRIEVAL_RUN_PROVENANCE_V1_SCHEMA {
            return Err(RetrievalRunProvenanceError::UnsupportedSchema);
        }
        if self.bindings.is_empty() {
            return Err(RetrievalRunProvenanceError::EmptyBindings);
        }
        if self.bindings.len() > MAX_PROVENANCE_BINDINGS {
            return Err(RetrievalRunProvenanceError::TooManyBindings {
                actual: self.bindings.len(),
                maximum: MAX_PROVENANCE_BINDINGS,
            });
        }
        let mut receipts = HashSet::new();
        for (index, binding) in self.bindings.iter().enumerate() {
            binding.validate(index)?;
            if !receipts.insert(binding.receipt_sha256.as_str()) {
                return Err(RetrievalRunProvenanceError::DuplicateReceipt { index });
            }
        }
        Ok(())
    }

    /// Returns a deterministic identity of this exact ordered envelope.
    pub fn identity_sha256(&self) -> Result<String, RetrievalRunProvenanceError> {
        self.validate()?;
        let mut hasher = Sha256::new();
        hasher.update(RETRIEVAL_RUN_PROVENANCE_V1_DOMAIN);
        update_bytes(&mut hasher, self.schema.as_bytes());
        update_length(&mut hasher, self.bindings.len());
        for binding in &self.bindings {
            update_bytes(&mut hasher, binding.receipt_schema.as_bytes());
            update_bytes(&mut hasher, binding.receipt_sha256.as_bytes());
            update_bytes(&mut hasher, binding.request_sha256.as_bytes());
            update_bytes(&mut hasher, binding.output_sha256.as_bytes());
        }
        Ok(format!("{:x}", hasher.finalize()))
    }

    /// Inserts this envelope under the reserved host-owned metadata key.
    pub fn insert_into_metadata(
        &self,
        metadata: &mut Value,
    ) -> Result<(), RetrievalRunProvenanceError> {
        self.validate()?;
        let object = metadata
            .as_object_mut()
            .ok_or(RetrievalRunProvenanceError::MetadataNotObject)?;
        object.insert(
            RETRIEVAL_RUN_PROVENANCE_METADATA_KEY.to_string(),
            serde_json::to_value(self).map_err(|_| RetrievalRunProvenanceError::EncodingFailure)?,
        );
        Ok(())
    }
}

/// Invalid adapter-provided retrieval provenance metadata.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
#[non_exhaustive]
pub enum RetrievalRunProvenanceError {
    #[error("retrieval provenance has an unsupported schema")]
    UnsupportedSchema,
    #[error("retrieval provenance must contain at least one binding")]
    EmptyBindings,
    #[error("retrieval provenance contains {actual} bindings; maximum is {maximum}")]
    TooManyBindings { actual: usize, maximum: usize },
    #[error("retrieval provenance binding {index} has an invalid receipt schema")]
    InvalidReceiptSchema { index: usize },
    #[error("retrieval provenance binding {index} has an invalid {field}")]
    InvalidDigest { index: usize, field: &'static str },
    #[error("retrieval provenance binding {index} repeats a receipt identity")]
    DuplicateReceipt { index: usize },
    #[error("retrieval provenance metadata must be a JSON object")]
    MetadataNotObject,
    #[error("retrieval provenance could not be encoded")]
    EncodingFailure,
}

pub(super) fn workflow_retrieval_provenance_audit(
    metadata: Option<&Value>,
    stage: &'static str,
) -> Option<Value> {
    let raw = metadata?.get(RETRIEVAL_RUN_PROVENANCE_METADATA_KEY)?;
    if !raw_provenance_envelope_is_bounded(raw) {
        return Some(rejected_audit(stage));
    }
    let envelope = match serde_json::from_value::<RetrievalRunProvenanceEnvelopeV1>(raw.clone()) {
        Ok(envelope) => envelope,
        Err(_) => return Some(rejected_audit(stage)),
    };
    let identity_sha256 = match envelope.identity_sha256() {
        Ok(identity) => identity,
        Err(_) => return Some(rejected_audit(stage)),
    };
    Some(serde_json::json!({
        "stage": stage,
        "status": "shape_validated",
        "identity_sha256": identity_sha256,
        "envelope": envelope,
    }))
}

fn raw_provenance_envelope_is_bounded(raw: &Value) -> bool {
    let Some(object) = raw.as_object() else {
        return false;
    };
    if object.len() != 2 || !object.contains_key("schema") || !object.contains_key("bindings") {
        return false;
    }
    let Some(schema) = object.get("schema").and_then(Value::as_str) else {
        return false;
    };
    if schema.len() > MAX_RECEIPT_SCHEMA_BYTES {
        return false;
    }
    let Some(bindings) = object.get("bindings").and_then(Value::as_array) else {
        return false;
    };
    !bindings.is_empty()
        && bindings.len() <= MAX_PROVENANCE_BINDINGS
        && bindings.iter().all(raw_provenance_binding_is_bounded)
}

fn raw_provenance_binding_is_bounded(raw: &Value) -> bool {
    let Some(object) = raw.as_object() else {
        return false;
    };
    if object.len() != 4
        || !object.contains_key("receipt_schema")
        || !object.contains_key("receipt_sha256")
        || !object.contains_key("request_sha256")
        || !object.contains_key("output_sha256")
    {
        return false;
    }
    let Some(receipt_schema) = object.get("receipt_schema").and_then(Value::as_str) else {
        return false;
    };
    if receipt_schema.len() > MAX_RECEIPT_SCHEMA_BYTES {
        return false;
    }
    ["receipt_sha256", "request_sha256", "output_sha256"]
        .into_iter()
        .all(|field| {
            object
                .get(field)
                .and_then(Value::as_str)
                .is_some_and(|digest| digest.len() == 64)
        })
}

fn rejected_audit(stage: &'static str) -> Value {
    serde_json::json!({
        "stage": stage,
        "status": "rejected",
        "reason": "invalid_adapter_provenance_envelope",
    })
}

fn valid_receipt_schema(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_RECEIPT_SCHEMA_BYTES
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'.' | b'_' | b':' | b'-')
        })
}

fn is_canonical_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn update_bytes(hasher: &mut Sha256, value: &[u8]) {
    update_length(hasher, value.len());
    hasher.update(value);
}

fn update_length(hasher: &mut Sha256, value: usize) {
    hasher.update((value as u128).to_be_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(character: char) -> String {
        std::iter::repeat_n(character, 64).collect()
    }

    fn binding(receipt: char) -> RetrievalRunProvenanceBindingV1 {
        RetrievalRunProvenanceBindingV1::new(
            "a3s/retrieval-receipt/v1",
            digest(receipt),
            digest('b'),
            digest('c'),
        )
        .expect("valid binding")
    }

    #[test]
    fn envelope_has_a_frozen_ordered_identity_and_metadata_key() {
        let envelope = RetrievalRunProvenanceEnvelopeV1::new(vec![binding('a'), binding('d')])
            .expect("valid envelope");

        assert_eq!(
            envelope.identity_sha256().expect("envelope identity"),
            "b2e990d9f603790397f577130835465f4e943cbf8e365a701b5fd64a29348cd5"
        );
        let reversed = RetrievalRunProvenanceEnvelopeV1::new(vec![binding('d'), binding('a')])
            .expect("valid reversed envelope");
        assert_ne!(
            envelope.identity_sha256().expect("ordered identity"),
            reversed.identity_sha256().expect("reversed identity")
        );
        let mut metadata = serde_json::json!({"dynamic_workflow": {}});
        envelope
            .insert_into_metadata(&mut metadata)
            .expect("insert provenance");
        assert_eq!(
            metadata[RETRIEVAL_RUN_PROVENANCE_METADATA_KEY]["schema"],
            RETRIEVAL_RUN_PROVENANCE_V1_SCHEMA
        );

        let mut unknown = serde_json::to_value(&envelope).expect("serialize envelope");
        unknown["unexpected"] = Value::Bool(true);
        assert!(serde_json::from_value::<RetrievalRunProvenanceEnvelopeV1>(unknown).is_err());
    }

    #[test]
    fn validation_rejects_ambiguous_or_unbounded_bindings() {
        assert!(RetrievalRunProvenanceBindingV1::new(
            "a3s receipt v1",
            digest('a'),
            digest('b'),
            digest('c'),
        )
        .is_err());
        assert!(RetrievalRunProvenanceBindingV1::new(
            "a3s/receipt/v1",
            digest('A'),
            digest('b'),
            digest('c'),
        )
        .is_err());

        let duplicate = RetrievalRunProvenanceEnvelopeV1::new(vec![binding('a'), binding('a')]);
        assert!(matches!(
            duplicate,
            Err(RetrievalRunProvenanceError::DuplicateReceipt { index: 1 })
        ));

        let too_many = (0..=MAX_PROVENANCE_BINDINGS)
            .map(|index| {
                RetrievalRunProvenanceBindingV1::new(
                    format!("a3s/receipt/{index}"),
                    format!("{index:064x}"),
                    digest('b'),
                    digest('c'),
                )
                .expect("unique valid binding")
            })
            .collect();
        assert!(matches!(
            RetrievalRunProvenanceEnvelopeV1::new(too_many),
            Err(RetrievalRunProvenanceError::TooManyBindings { .. })
        ));
    }

    #[test]
    fn audit_accepts_only_the_reserved_top_level_host_envelope() {
        let envelope =
            RetrievalRunProvenanceEnvelopeV1::new(vec![binding('a')]).expect("valid envelope");
        let nested = serde_json::json!({
            "dynamic_workflow": {
                RETRIEVAL_RUN_PROVENANCE_METADATA_KEY: envelope,
            }
        });
        assert!(workflow_retrieval_provenance_audit(Some(&nested), "bootstrap").is_none());

        let invalid = serde_json::json!({
            RETRIEVAL_RUN_PROVENANCE_METADATA_KEY: {
                "schema": RETRIEVAL_RUN_PROVENANCE_V1_SCHEMA,
                "bindings": [{"receipt_sha256": digest('a')}],
            }
        });
        assert_eq!(
            workflow_retrieval_provenance_audit(Some(&invalid), "bootstrap")
                .expect("rejection audit")["status"],
            "rejected"
        );
    }

    #[test]
    fn audit_rejects_unbounded_raw_envelopes_before_deserialization() {
        let oversized_schema = serde_json::json!({
            RETRIEVAL_RUN_PROVENANCE_METADATA_KEY: {
                "schema": RETRIEVAL_RUN_PROVENANCE_V1_SCHEMA,
                "bindings": [{
                    "receipt_schema": "x".repeat(MAX_RECEIPT_SCHEMA_BYTES + 1),
                    "receipt_sha256": digest('a'),
                    "request_sha256": digest('b'),
                    "output_sha256": digest('c'),
                }],
            }
        });
        assert_eq!(
            workflow_retrieval_provenance_audit(Some(&oversized_schema), "bootstrap")
                .expect("oversized rejection audit")["status"],
            "rejected"
        );

        let oversized_bindings = serde_json::json!({
            RETRIEVAL_RUN_PROVENANCE_METADATA_KEY: {
                "schema": RETRIEVAL_RUN_PROVENANCE_V1_SCHEMA,
                "bindings": (0..=MAX_PROVENANCE_BINDINGS)
                    .map(|index| serde_json::json!({
                        "receipt_schema": "a3s/retrieval-receipt/v1",
                        "receipt_sha256": format!("{index:064x}"),
                        "request_sha256": digest('b'),
                        "output_sha256": digest('c'),
                    }))
                    .collect::<Vec<_>>(),
            }
        });
        assert_eq!(
            workflow_retrieval_provenance_audit(Some(&oversized_bindings), "bootstrap")
                .expect("binding-count rejection audit")["status"],
            "rejected"
        );

        let oversized_unknown = serde_json::json!({
            RETRIEVAL_RUN_PROVENANCE_METADATA_KEY: {
                "schema": RETRIEVAL_RUN_PROVENANCE_V1_SCHEMA,
                "bindings": [binding('a')],
                "untrusted": "x".repeat(1_000_000),
            }
        });
        assert_eq!(
            workflow_retrieval_provenance_audit(Some(&oversized_unknown), "bootstrap")
                .expect("unknown-field rejection audit")["status"],
            "rejected"
        );
    }
}
