use std::path::Path;

use ed25519_dalek::{Signature, VerifyingKey};
use sha2::{Digest, Sha256};

pub(crate) fn verify_ed25519(
    payload: &[u8],
    signature_hex: &str,
    public_key_hex: &str,
    role: &str,
) -> Result<String, String> {
    let signature_bytes = decode_hex::<64>(signature_hex, &format!("{role} signature"))?;
    let public_key_bytes = decode_hex::<32>(public_key_hex, &format!("{role} public key"))?;
    let signature = Signature::from_bytes(&signature_bytes);
    let public_key = VerifyingKey::from_bytes(&public_key_bytes)
        .map_err(|error| format!("invalid {role} public key: {error}"))?;
    public_key
        .verify_strict(payload, &signature)
        .map_err(|error| format!("invalid {role} signature: {error}"))?;
    Ok(sha256_bytes(&public_key_bytes))
}

pub(crate) fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(crate) fn sorted_commitment_root<'a>(values: impl IntoIterator<Item = &'a str>) -> String {
    sorted_root(b"a3s/deep-research-case-set/v1\n", values)
}

pub(crate) fn sorted_attempt_commitment_root<'a>(
    values: impl IntoIterator<Item = &'a str>,
) -> String {
    sorted_root(b"a3s/deep-research-attempt-set/v1\n", values)
}

pub(crate) fn sorted_attempt_log_root<'a>(values: impl IntoIterator<Item = &'a str>) -> String {
    sorted_root(b"a3s/deep-research-attempt-log/v1\n", values)
}

pub(crate) fn sorted_artifact_subtree_root<'a>(
    values: impl IntoIterator<Item = &'a str>,
) -> String {
    sorted_root(b"a3s/deep-research-artifact-subtrees/v1\n", values)
}

pub(crate) fn sorted_ballot_receipt_root<'a>(values: impl IntoIterator<Item = &'a str>) -> String {
    sorted_root(b"a3s/deep-research-review-ballots/v1\n", values)
}

pub(crate) fn case_descriptor_commitment(
    sealed_payload_sha256: &str,
    fields: &[&str],
    material_dimension_count: usize,
) -> String {
    let mut payload = b"a3s/deep-research-case-descriptor/v1\n".to_vec();
    append_field(&mut payload, sealed_payload_sha256.as_bytes());
    for field in fields {
        append_field(&mut payload, field.as_bytes());
    }
    payload.extend_from_slice(&(material_dimension_count as u64).to_be_bytes());
    sha256_bytes(&payload)
}

pub(crate) fn attempt_slot_commitment(
    case_commitment_sha256: &str,
    slot_index: usize,
    slot_nonce_sha256: &str,
) -> String {
    let mut payload = b"a3s/deep-research-attempt-slot/v1\n".to_vec();
    append_field(&mut payload, case_commitment_sha256.as_bytes());
    payload.extend_from_slice(&(slot_index as u64).to_be_bytes());
    append_field(&mut payload, slot_nonce_sha256.as_bytes());
    sha256_bytes(&payload)
}

pub(crate) fn attempt_start_receipt(slot_commitment_sha256: &str, started_at: &str) -> String {
    let mut payload = b"a3s/deep-research-attempt-start/v1\n".to_vec();
    append_field(&mut payload, slot_commitment_sha256.as_bytes());
    append_field(&mut payload, started_at.as_bytes());
    sha256_bytes(&payload)
}

pub(crate) fn attempt_terminal_receipt(
    start_receipt_sha256: &str,
    terminal: &str,
    finished_at: &str,
    artifact_subtree_sha256: &str,
) -> String {
    let mut payload = b"a3s/deep-research-attempt-terminal/v1\n".to_vec();
    append_field(&mut payload, start_receipt_sha256.as_bytes());
    append_field(&mut payload, terminal.as_bytes());
    append_field(&mut payload, finished_at.as_bytes());
    append_field(&mut payload, artifact_subtree_sha256.as_bytes());
    sha256_bytes(&payload)
}

pub(crate) fn sha256_file(path: impl AsRef<Path>) -> Result<String, String> {
    std::fs::read(path.as_ref())
        .map(|bytes| sha256_bytes(&bytes))
        .map_err(|error| format!("read {}: {error}", path.as_ref().display()))
}

pub(crate) fn validate_sha256(value: &str, field: &str) -> Result<(), String> {
    if value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(())
    } else {
        Err(format!("{field} must be a lowercase SHA-256 digest"))
    }
}

fn decode_hex<const LENGTH: usize>(value: &str, field: &str) -> Result<[u8; LENGTH], String> {
    if value.len() != LENGTH * 2 {
        return Err(format!("{field} has the wrong length"));
    }
    let mut decoded = [0_u8; LENGTH];
    for (index, output) in decoded.iter_mut().enumerate() {
        *output = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)
            .map_err(|_| format!("{field} is not hexadecimal"))?;
    }
    Ok(decoded)
}

fn append_field(payload: &mut Vec<u8>, field: &[u8]) {
    payload.extend_from_slice(&(field.len() as u64).to_be_bytes());
    payload.extend_from_slice(field);
}

fn sorted_root<'a>(domain: &[u8], values: impl IntoIterator<Item = &'a str>) -> String {
    let mut values = values.into_iter().collect::<Vec<_>>();
    values.sort_unstable();
    let mut payload = domain.to_vec();
    for value in values {
        append_field(&mut payload, value.as_bytes());
    }
    sha256_bytes(&payload)
}
