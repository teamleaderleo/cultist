use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use crate::compact_ir::MAX_C1_BYTES;

const ALIAS_HEADER: &str = "C1A";
const CANONICAL_HEADER: &str = "C1";

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AliasError {
    message: String,
}

impl AliasError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for AliasError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "C1 local aliases: {}", self.message)
    }
}

impl Error for AliasError {}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AliasEntry {
    pub id: usize,
    pub value: String,
    pub occurrences: usize,
    pub net_bytes_saved: usize,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AliasTransform {
    pub encoded: String,
    pub aliases: Vec<AliasEntry>,
    pub canonical_bytes: usize,
    pub encoded_bytes: usize,
}

impl AliasTransform {
    pub fn used_aliases(&self) -> bool {
        !self.aliases.is_empty()
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct Candidate {
    raw_literal: String,
    occurrences: usize,
    preliminary_savings: usize,
}

pub fn alias_canonical_c1(input: &str) -> Result<AliasTransform, AliasError> {
    if !input.starts_with("C1\n") {
        return Err(AliasError::new(
            "input must begin with the canonical `C1` header",
        ));
    }

    let spans = json_string_spans(input)?;
    let mut counts = BTreeMap::<String, usize>::new();
    for (start, end) in &spans {
        *counts.entry(input[*start..*end].to_string()).or_default() += 1;
    }

    let mut candidates = counts
        .into_iter()
        .filter_map(|(raw_literal, occurrences)| {
            if occurrences < 2 {
                return None;
            }
            let preliminary_savings = alias_net_savings(raw_literal.len(), occurrences, 1)?;
            Some(Candidate {
                raw_literal,
                occurrences,
                preliminary_savings,
            })
        })
        .collect::<Vec<_>>();

    candidates.sort_by(|left, right| {
        right
            .preliminary_savings
            .cmp(&left.preliminary_savings)
            .then_with(|| left.raw_literal.cmp(&right.raw_literal))
    });

    let mut selected = Vec::<(String, usize, usize)>::new();
    for candidate in candidates {
        let id = selected.len() + 1;
        let Some(net_bytes_saved) =
            alias_net_savings(candidate.raw_literal.len(), candidate.occurrences, id)
        else {
            continue;
        };
        selected.push((
            candidate.raw_literal,
            candidate.occurrences,
            net_bytes_saved,
        ));
    }

    if selected.is_empty() {
        return Ok(plain_transform(input));
    }

    let lookup = selected
        .iter()
        .enumerate()
        .map(|(index, (raw_literal, _, _))| (raw_literal.as_str(), index + 1))
        .collect::<BTreeMap<_, _>>();

    let mut body = String::with_capacity(input.len());
    let mut cursor = 0usize;
    for (start, end) in spans {
        body.push_str(&input[cursor..start]);
        let raw_literal = &input[start..end];
        if let Some(id) = lookup.get(raw_literal) {
            body.push('@');
            body.push_str(&id.to_string());
        } else {
            body.push_str(raw_literal);
        }
        cursor = end;
    }
    body.push_str(&input[cursor..]);

    let mut encoded = String::new();
    encoded.push_str(ALIAS_HEADER);
    encoded.push('\n');
    for (index, (raw_literal, _, _)) in selected.iter().enumerate() {
        encoded.push('A');
        encoded.push_str(&(index + 1).to_string());
        encoded.push(' ');
        encoded.push_str(raw_literal);
        encoded.push('\n');
    }
    encoded.push_str(&body);

    if encoded.len() >= input.len() {
        return Ok(plain_transform(input));
    }

    let aliases = selected
        .into_iter()
        .enumerate()
        .map(|(index, (raw_literal, occurrences, net_bytes_saved))| {
            let value = serde_json::from_str::<String>(&raw_literal).map_err(|error| {
                AliasError::new(format!("invalid canonical C1 string literal: {error}"))
            })?;
            Ok(AliasEntry {
                id: index + 1,
                value,
                occurrences,
                net_bytes_saved,
            })
        })
        .collect::<Result<Vec<_>, AliasError>>()?;

    Ok(AliasTransform {
        canonical_bytes: input.len(),
        encoded_bytes: encoded.len(),
        encoded,
        aliases,
    })
}

pub fn expand_c1_aliases(input: &str) -> Result<String, AliasError> {
    if input.starts_with("C1\n") {
        return Ok(input.to_string());
    }
    if !input.starts_with("C1A\n") {
        return Err(AliasError::new(
            "input must begin with `C1` or the research `C1A` header",
        ));
    }
    if input.len() > MAX_C1_BYTES {
        return Err(AliasError::new(format!(
            "C1A input exceeds the {MAX_C1_BYTES}-byte transport limit"
        )));
    }

    let mut offset = "C1A\n".len();
    let mut aliases = BTreeMap::<usize, String>::new();
    let mut values = BTreeSet::<String>::new();
    let body_offset = loop {
        let remainder = &input[offset..];
        let Some(relative_end) = remainder.find('\n') else {
            return Err(AliasError::new("alias packet is missing canonical C1 body"));
        };
        let line_end = offset + relative_end;
        let line = &input[offset..line_end];
        let next_offset = line_end + 1;

        if line == CANONICAL_HEADER {
            break offset;
        }

        let (id, raw_literal) = parse_alias_definition(line)?;
        let expected_id = aliases.len() + 1;
        if id != expected_id {
            return Err(AliasError::new(format!(
                "alias A{id} is out of sequence; expected A{expected_id}"
            )));
        }
        if aliases.insert(id, raw_literal.clone()).is_some() {
            return Err(AliasError::new(format!("duplicate alias id A{id}")));
        }
        if !values.insert(raw_literal) {
            return Err(AliasError::new("duplicate alias literal"));
        }
        offset = next_offset;
    };

    if aliases.is_empty() {
        return Err(AliasError::new(
            "C1A packet must declare at least one alias",
        ));
    }

    expand_body(&input[body_offset..], &aliases)
}

fn plain_transform(input: &str) -> AliasTransform {
    AliasTransform {
        encoded: input.to_string(),
        aliases: Vec::new(),
        canonical_bytes: input.len(),
        encoded_bytes: input.len(),
    }
}

fn alias_net_savings(literal_bytes: usize, occurrences: usize, id: usize) -> Option<usize> {
    let digits = id.to_string().len();
    let token_bytes = 1 + digits;
    if literal_bytes <= token_bytes {
        return None;
    }
    let replacement_savings = occurrences.checked_mul(literal_bytes - token_bytes)?;
    let declaration_bytes = literal_bytes.checked_add(digits + 3)?;
    let net = replacement_savings.checked_sub(declaration_bytes)?;
    (net > 0).then_some(net)
}

fn parse_alias_definition(line: &str) -> Result<(usize, String), AliasError> {
    let Some(rest) = line.strip_prefix('A') else {
        return Err(AliasError::new(format!(
            "malformed alias definition `{line}`"
        )));
    };
    let Some((id_text, raw_literal)) = rest.split_once(' ') else {
        return Err(AliasError::new(format!(
            "malformed alias definition `{line}`"
        )));
    };
    if id_text.is_empty() || !id_text.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(AliasError::new(format!("invalid alias id `{id_text}`")));
    }
    let id = id_text
        .parse::<usize>()
        .map_err(|_| AliasError::new(format!("invalid alias id `{id_text}`")))?;
    if id == 0 {
        return Err(AliasError::new("alias ids start at A1"));
    }

    let value = serde_json::from_str::<String>(raw_literal)
        .map_err(|error| AliasError::new(format!("invalid alias JSON string: {error}")))?;
    let canonical = serde_json::to_string(&value).map_err(|error| {
        AliasError::new(format!("could not canonicalize alias string: {error}"))
    })?;
    if canonical != raw_literal {
        return Err(AliasError::new(
            "alias literal must use canonical JSON encoding",
        ));
    }

    Ok((id, raw_literal.to_string()))
}

fn json_string_spans(input: &str) -> Result<Vec<(usize, usize)>, AliasError> {
    let bytes = input.as_bytes();
    let mut spans = Vec::new();
    let mut index = 0usize;

    while index < bytes.len() {
        if bytes[index] != b'"' {
            index += 1;
            continue;
        }
        let start = index;
        index += 1;
        let mut escaped = false;
        let mut closed = false;
        while index < bytes.len() {
            let byte = bytes[index];
            if escaped {
                escaped = false;
                index += 1;
                continue;
            }
            if byte == b'\\' {
                escaped = true;
                index += 1;
                continue;
            }
            if byte == b'"' {
                index += 1;
                spans.push((start, index));
                closed = true;
                break;
            }
            index += 1;
        }
        if !closed {
            return Err(AliasError::new("unterminated JSON string in canonical C1"));
        }
    }

    Ok(spans)
}

fn checked_expanded_len(current: usize, additional: usize) -> Result<usize, AliasError> {
    current
        .checked_add(additional)
        .ok_or_else(|| AliasError::new("expanded canonical C1 byte count overflow"))
}

fn append_bounded(output: &mut String, value: &str) -> Result<(), AliasError> {
    let next_len = checked_expanded_len(output.len(), value.len())?;
    if next_len > MAX_C1_BYTES {
        return Err(AliasError::new(format!(
            "expanded canonical C1 exceeds the {MAX_C1_BYTES}-byte transport limit"
        )));
    }
    output.push_str(value);
    Ok(())
}

fn expand_body(body: &str, aliases: &BTreeMap<usize, String>) -> Result<String, AliasError> {
    if !body.starts_with("C1\n") {
        return Err(AliasError::new(
            "alias body must begin with canonical `C1` header",
        ));
    }

    let bytes = body.as_bytes();
    let mut output = String::with_capacity(body.len());
    let mut cursor = 0usize;
    let mut index = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    while index < bytes.len() {
        let byte = bytes[index];
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
            index += 1;
            continue;
        }

        if byte == b'"' {
            in_string = true;
            index += 1;
            continue;
        }

        if byte != b'@' {
            index += 1;
            continue;
        }

        append_bounded(&mut output, &body[cursor..index])?;
        let token_start = index;
        index += 1;
        let digits_start = index;
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            index += 1;
        }
        if digits_start == index {
            return Err(AliasError::new(format!(
                "malformed alias reference at byte {token_start}"
            )));
        }
        let id_text = &body[digits_start..index];
        let id = id_text
            .parse::<usize>()
            .map_err(|_| AliasError::new(format!("invalid alias reference `@{id_text}`")))?;
        let raw_literal = aliases
            .get(&id)
            .ok_or_else(|| AliasError::new(format!("unknown alias reference `@{id}`")))?;
        append_bounded(&mut output, raw_literal)?;
        cursor = index;
    }

    if in_string {
        return Err(AliasError::new("unterminated JSON string in alias body"));
    }

    append_bounded(&mut output, &body[cursor..])?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expanded_length_arithmetic_fails_closed_on_overflow() {
        assert!(checked_expanded_len(usize::MAX, 1).is_err());
    }
}
