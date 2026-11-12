//! Global diagnostics are summaries; explicit snapshots retain the complete data.
use serde_json::{Value, json};

pub(super) const COUNT: usize = 50;
pub(super) const CHARACTERS: usize = 8192;
const MESSAGE_CHARACTERS: usize = 1024;

struct Message {
    text: String,
    shortened: bool,
}

fn message(source: &str, budget: &mut usize) -> Message {
    let mut text: String = source
        .chars()
        .take((*budget).min(MESSAGE_CHARACTERS))
        .collect();
    let shortened = text.len() < source.len();
    if shortened {
        text.pop();
        text.push('…');
    }
    *budget -= text.chars().count();
    Message { text, shortened }
}

pub(super) fn append(result: &mut Value, source: &[String]) {
    let mut budget = CHARACTERS;
    let mut messages = Vec::new();
    for original in source.iter().take(COUNT) {
        if budget == 0 {
            break;
        }
        messages.push(message(original, &mut budget));
    }
    let shortened = messages.iter().filter(|message| message.shortened).count();
    result["diagnostics_total"] = json!(source.len());
    result["omitted_diagnostics"] = json!(source.len() - messages.len());
    result["shortened_diagnostics"] = json!(shortened);
    result["diagnostics"] = json!(
        messages
            .into_iter()
            .map(|message| message.text)
            .collect::<Vec<_>>()
    );
}
