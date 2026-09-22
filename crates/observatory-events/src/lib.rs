//! Soroban contract event normalization, filtering, and JSON representation.
//!
//! Events are supplied as a fixture document. Each event carries base64 XDR
//! `ScVal` topics and data, which are decoded with the real Stellar XDR types
//! and rendered to stable JSON.
//!
//! # Fixture format
//!
//! ```json
//! {
//!   "events": [
//!     {
//!       "contractId": "C...",
//!       "ledger": 123,
//!       "txHash": "abcd",
//!       "type": "contract",
//!       "topics": ["<base64 ScVal>", "<base64 ScVal>"],
//!       "data": "<base64 ScVal>"
//!     }
//!   ]
//! }
//! ```
//!
//! A bare array of event objects is also accepted. `topics` and `data` are
//! optional. `type` defaults to `contract`.
//!
//! # Limitation
//!
//! Declared events in the contract specification (`contractspecv0`) and
//! historical event data are different things: the specification declares the
//! shape of events, while a fixture contains concrete emitted events. This crate
//! handles the latter.

use observatory_core::limits::{MAX_EVENTS, MAX_EVENT_FILE_BYTES};
use observatory_core::{ObservatoryError, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use stellar_xdr::{Limits, ReadXdr, ScVal};

/// A normalized contract event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventRecord {
    /// Contract id (C... strkey), when known.
    pub contract_id: Option<String>,
    /// Ledger sequence, when known.
    pub ledger: Option<u32>,
    /// Transaction hash, when known.
    pub tx_hash: Option<String>,
    /// Event type: `system`, `contract`, or `diagnostic`.
    pub event_type: String,
    /// Topics rendered as JSON strings, in order.
    pub topics: Vec<String>,
    /// Symbol/string text extracted from topics, for filtering.
    pub topic_symbols: Vec<String>,
    /// Data rendered as JSON.
    pub data: String,
}

/// A filter over a set of events.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EventFilter {
    /// Only keep events for this contract id.
    pub contract: Option<String>,
    /// Only keep events whose topic text or rendered topic contains this value.
    pub topic: Option<String>,
    /// Only keep events of this type.
    pub event_type: Option<String>,
    /// Keep at most this many events (applied last).
    pub limit: Option<usize>,
}

/// Parse an event fixture from a JSON string.
pub fn parse_events_str(input: &str) -> Result<Vec<EventRecord>> {
    if input.len() > MAX_EVENT_FILE_BYTES {
        return Err(ObservatoryError::invalid(format!(
            "event fixture is {} bytes, exceeding the {} byte limit",
            input.len(),
            MAX_EVENT_FILE_BYTES
        )));
    }
    let value: Value = serde_json::from_str(input)
        .map_err(|error| ObservatoryError::decode(format!("invalid event JSON: {error}")))?;
    parse_events(&value)
}

/// Parse an event fixture from a JSON value.
pub fn parse_events(value: &Value) -> Result<Vec<EventRecord>> {
    let array = if let Some(events) = value.get("events").and_then(Value::as_array) {
        events
    } else if let Some(events) = value.as_array() {
        events
    } else {
        return Err(ObservatoryError::decode(
            "event fixture must be an array or an object with an `events` array",
        ));
    };
    if array.len() > MAX_EVENTS {
        return Err(ObservatoryError::invalid(format!(
            "event fixture has {} events, exceeding the {MAX_EVENTS} limit",
            array.len()
        )));
    }
    array.iter().map(parse_event).collect()
}

/// Apply a filter to a set of events.
#[must_use]
pub fn filter(events: &[EventRecord], filter: &EventFilter) -> Vec<EventRecord> {
    let mut out: Vec<EventRecord> = events
        .iter()
        .filter(|event| {
            filter
                .contract
                .as_ref()
                .is_none_or(|contract| event.contract_id.as_deref() == Some(contract.as_str()))
        })
        .filter(|event| {
            filter
                .event_type
                .as_ref()
                .is_none_or(|kind| event.event_type.eq_ignore_ascii_case(kind))
        })
        .filter(|event| {
            filter.topic.as_ref().is_none_or(|topic| {
                event
                    .topic_symbols
                    .iter()
                    .any(|symbol| symbol.contains(topic.as_str()))
                    || event
                        .topics
                        .iter()
                        .any(|rendered| rendered.contains(topic.as_str()))
            })
        })
        .cloned()
        .collect();
    if let Some(limit) = filter.limit {
        out.truncate(limit);
    }
    out
}

fn parse_event(value: &Value) -> Result<EventRecord> {
    let object = value
        .as_object()
        .ok_or_else(|| ObservatoryError::decode("event entry must be a JSON object"))?;
    let topics_value = object.get("topics");
    let topics = match topics_value {
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| {
                let encoded = item.as_str().ok_or_else(|| {
                    ObservatoryError::decode("event topic must be a base64 string")
                })?;
                decode_scval(encoded)
            })
            .collect::<Result<Vec<_>>>()?,
        Some(_) => return Err(ObservatoryError::decode("`topics` must be an array")),
        None => Vec::new(),
    };
    let data = match object.get("data") {
        Some(Value::String(encoded)) => decode_scval(encoded)?,
        Some(Value::Null) | None => ScVal::Void,
        Some(_) => return Err(ObservatoryError::decode("`data` must be a base64 string")),
    };

    let mut rendered_topics = Vec::with_capacity(topics.len());
    let mut topic_symbols = Vec::new();
    for topic in &topics {
        rendered_topics.push(render_scval(topic)?);
        if let Some(text) = scval_text(topic) {
            topic_symbols.push(text);
        }
    }

    Ok(EventRecord {
        contract_id: object
            .get("contractId")
            .and_then(Value::as_str)
            .map(str::to_string),
        ledger: object
            .get("ledger")
            .and_then(Value::as_u64)
            .map(|value| value as u32),
        tx_hash: object
            .get("txHash")
            .and_then(Value::as_str)
            .map(str::to_string),
        event_type: object
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("contract")
            .to_string(),
        topics: rendered_topics,
        topic_symbols,
        data: render_scval(&data)?,
    })
}

fn decode_scval(encoded: &str) -> Result<ScVal> {
    ScVal::from_xdr_base64(encoded.trim(), Limits::none())
        .map_err(|error| ObservatoryError::decode(format!("invalid ScVal XDR: {error}")))
}

fn render_scval(value: &ScVal) -> Result<String> {
    serde_json::to_string(value)
        .map_err(|error| ObservatoryError::internal(format!("failed to render ScVal: {error}")))
}

fn scval_text(value: &ScVal) -> Option<String> {
    match value {
        ScVal::Symbol(symbol) => Some(symbol.0.to_string()),
        ScVal::String(text) => Some(text.0.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use stellar_xdr::{ScSymbol, StringM, WriteXdr};

    fn symbol_base64(text: &str) -> String {
        ScVal::Symbol(ScSymbol(StringM::try_from(text).unwrap()))
            .to_xdr_base64(Limits::none())
            .unwrap()
    }

    fn fixture() -> Value {
        json!({
            "events": [
                {
                    "contractId": "CAAA",
                    "ledger": 10,
                    "type": "contract",
                    "topics": [symbol_base64("transfer"), symbol_base64("from")],
                    "data": symbol_base64("amount")
                },
                {
                    "contractId": "CBBB",
                    "ledger": 11,
                    "type": "system",
                    "topics": [symbol_base64("mint")],
                    "data": null
                }
            ]
        })
    }

    #[test]
    fn parses_fixture() {
        let events = parse_events(&fixture()).unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].topic_symbols, vec!["transfer", "from"]);
        assert!(events[0].topics[0].contains("transfer"));
        assert_eq!(events[1].event_type, "system");
    }

    #[test]
    fn accepts_bare_array() {
        let value = json!([{ "type": "contract", "topics": [], "data": null }]);
        assert_eq!(parse_events(&value).unwrap().len(), 1);
    }

    #[test]
    fn filters_by_contract_topic_and_type() {
        let events = parse_events(&fixture()).unwrap();

        let by_contract = filter(
            &events,
            &EventFilter {
                contract: Some("CAAA".to_string()),
                ..Default::default()
            },
        );
        assert_eq!(by_contract.len(), 1);

        let by_topic = filter(
            &events,
            &EventFilter {
                topic: Some("mint".to_string()),
                ..Default::default()
            },
        );
        assert_eq!(by_topic.len(), 1);
        assert_eq!(by_topic[0].contract_id.as_deref(), Some("CBBB"));

        let by_type = filter(
            &events,
            &EventFilter {
                event_type: Some("SYSTEM".to_string()),
                ..Default::default()
            },
        );
        assert_eq!(by_type.len(), 1);
    }

    #[test]
    fn applies_limit_last() {
        let events = parse_events(&fixture()).unwrap();
        let limited = filter(
            &events,
            &EventFilter {
                contract: Some("CAAA".to_string()),
                limit: Some(0),
                ..Default::default()
            },
        );
        assert!(limited.is_empty());
    }

    #[test]
    fn rejects_malformed_base64() {
        let value = json!([{ "topics": ["not base64!!!"], "data": null }]);
        assert!(parse_events(&value).is_err());
    }

    #[test]
    fn rejects_wrong_shape() {
        assert!(parse_events(&json!({ "foo": 1 })).is_err());
        assert!(parse_events(&json!([1, 2, 3])).is_err());
    }

    #[test]
    fn empty_events_are_allowed() {
        assert!(parse_events(&json!({ "events": [] })).unwrap().is_empty());
    }
}
