use crate::{
    arguments::{self, RefreshArgs},
    configuration::Configuration,
    process, wire,
};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::time::Duration;
use tokio_util::sync::CancellationToken;

const QUERY_DEADLINE: Duration = Duration::from_secs(120);
const REFRESH_DEADLINE: Duration = Duration::from_secs(1800);
#[derive(Clone)]
pub struct Backend {
    config: Configuration,
    processes: process::Processes,
    capabilities: wire::Capabilities,
}
pub struct Reply {
    pub value: Value,
    pub is_error: bool,
}

impl Backend {
    pub async fn connect(config: Configuration) -> Result<Self> {
        let processes = process::Processes::default();
        let output = processes
            .run(
                &config.engine,
                process::Invocation {
                    arguments: vec!["capabilities".into()],
                    input: None,
                    mode: process::StdinMode::Close,
                    deadline: Duration::from_secs(15),
                },
                &CancellationToken::new(),
            )
            .await?;
        process::require_success(&output)?;
        let capabilities = wire::capabilities(
            &serde_json::from_slice(&output.stdout).context("Invalid CLI capabilities JSON")?,
        )?;
        Ok(Self {
            config,
            processes,
            capabilities,
        })
    }
    pub async fn shutdown(&self) {
        self.processes.shutdown().await;
    }
    pub async fn query(
        &self,
        query: arguments::Query,
        cancel: &CancellationToken,
    ) -> Result<Reply> {
        let operation = query.query["kind"]
            .as_str()
            .context("Missing query kind")?
            .to_owned();
        if let Some(reply) = self.unsupported_observation(&operation) {
            return Ok(reply);
        }
        let request = json!({"version":1,"project":self.config.project,"roots":self.config.roots,"expected_generation":query.generation,"query":query.query});
        let output = self
            .processes
            .run(
                &self.config.engine,
                process::Invocation {
                    arguments: vec!["query".into()],
                    input: Some(serde_json::to_vec(&request)?),
                    mode: process::StdinMode::Close,
                    deadline: QUERY_DEADLINE,
                },
                cancel,
            )
            .await?;
        let value: Value = serde_json::from_slice(&output.stdout).with_context(|| {
            format!(
                "Invalid CLI query JSON (successful exit: {}): {}",
                output.success,
                output.stderr.trim()
            )
        })?;
        let is_error = wire::response(&value, query.generation.as_deref())?;
        if !is_error {
            process::require_success(&output)?;
            wire::metadata::validate(&value, &operation, query.generation.is_some())?;
            wire::results::validate(&operation, &value["result"])?;
            wire::results::binding(&query.query, &value["result"])?;
        }
        Ok(Reply { value, is_error })
    }
    fn unsupported_observation(&self, operation: &str) -> Option<Reply> {
        if operation != "freshness" {
            return None;
        }
        let wire::FreshnessSupport::Unsupported(message) = self.capabilities.freshness else {
            return None;
        };
        Some(Reply {
            value: json!({"version":1,"generation":null,"freshness":null,"result":null,"error":{"code":"unsupported_capability","message":message}}),
            is_error: true,
        })
    }
    pub async fn discovery(&self, cancel: &CancellationToken) -> Result<Value> {
        let output = self
            .processes
            .run(
                &self.config.engine,
                process::Invocation {
                    arguments: vec![
                        "discover".into(),
                        self.config.project.to_string_lossy().into_owned(),
                    ],
                    input: None,
                    mode: process::StdinMode::Close,
                    deadline: QUERY_DEADLINE,
                },
                cancel,
            )
            .await?;
        process::require_success(&output)?;
        let value: Value =
            serde_json::from_slice(&output.stdout).context("Invalid CLI discovery JSON")?;
        serde_json::from_value::<wire::Discovery>(value.clone())
            .context("Malformed CLI discovery response")?;
        Ok(value)
    }
    pub async fn refresh(&self, args: RefreshArgs, cancel: &CancellationToken) -> Result<Reply> {
        let discovered: wire::Discovery = serde_json::from_value(self.discovery(cancel).await?)?;
        let targets = choose_targets(&discovered, args)?;
        let request = json!({"command":"analyze","version":1,"project":self.config.project,"roots":self.config.roots,"targets":targets,"infer_linked_roots":false});
        let mut bytes = serde_json::to_vec(&request)?;
        bytes.push(b'\n');
        let output = self
            .processes
            .run(
                &self.config.engine,
                process::Invocation {
                    arguments: vec!["serve-once".into()],
                    input: Some(bytes),
                    mode: process::StdinMode::Analysis,
                    deadline: REFRESH_DEADLINE,
                },
                cancel,
            )
            .await?;
        let diagnostic = output.cleanup_diagnostic.clone();
        analysis_reply(output).map_err(|error| match diagnostic {
            Some(diagnostic) => error.context(diagnostic),
            None => error,
        })
    }
}

fn choose_targets(discovered: &wire::Discovery, args: RefreshArgs) -> Result<Vec<wire::Target>> {
    let Some(identities) = args.targets else {
        return discovered.selected();
    };
    ensure!(
        !identities.is_empty() && identities.len() <= 1000,
        "Refresh requires 1..1000 build identities"
    );
    let mut targets: Vec<wire::Target> = Vec::new();
    for identity in identities {
        let matches: Vec<_> = discovered
            .targets
            .iter()
            .filter(|t| t.path == identity.path && t.kind == identity.kind)
            .collect();
        ensure!(
            matches.len() == 1,
            "Refresh target identity was not uniquely returned by CLI discovery"
        );
        ensure!(
            !targets
                .iter()
                .any(|t| t.path == identity.path && t.kind == identity.kind),
            "Duplicate refresh target"
        );
        targets.push(matches[0].clone());
    }
    Ok(targets)
}

fn analysis_reply(output: process::Output) -> Result<Reply> {
    let text = std::str::from_utf8(&output.stdout).context("CLI analysis output is not UTF-8")?;
    let mut events = text.lines();
    let hello: Value = serde_json::from_str(events.next().context("CLI omitted protocol hello")?)?;
    ensure!(
        hello == json!({"event":"hello","version":1}),
        "Incompatible CLI analysis protocol hello"
    );
    let mut terminal = None;
    for line in events {
        ensure!(
            terminal.is_none(),
            "CLI emitted data after its terminal event"
        );
        let event: Value = serde_json::from_str(line).context("Invalid CLI analysis event JSON")?;
        match event["event"].as_str() {
            Some("progress") => {
                ensure!(event["message"].is_string(), "Malformed CLI progress event");
            }
            Some("complete") | Some("failed") => terminal = Some(event),
            _ => anyhow::bail!("Unexpected CLI analysis event"),
        }
    }
    let terminal = terminal.context("CLI omitted terminal analysis event")?;
    if terminal["event"] == "failed" {
        let message = terminal["message"]
            .as_str()
            .context("Malformed CLI failure event")?;
        return Ok(Reply {
            value: json!({"version":1,"generation":null,"freshness":null,"result":null,"error":{"code":"analysis_failed","message":message},"cli_event":terminal,"cleanup_diagnostic":output.cleanup_diagnostic}),
            is_error: true,
        });
    }
    validate_complete(&terminal)?;
    process::require_success(&output)?;
    Ok(Reply {
        value: json!({"version":1,"generation":terminal["digest"],"freshness":null,"result":terminal,"error":null}),
        is_error: false,
    })
}
fn validate_complete(event: &Value) -> Result<()> {
    let digest = event["digest"]
        .as_str()
        .context("CLI completion omitted digest")?;
    ensure!(
        wire::generation(digest),
        "CLI completion returned an invalid digest"
    );
    ensure!(
        event["roots"].is_array()
            && event["fingerprints"].is_array()
            && event["metrics"].is_object()
            && event["engine_versions"].is_string(),
        "Malformed CLI completion metadata"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn output(text: &str) -> process::Output {
        process::Output {
            stdout: text.as_bytes().to_vec(),
            stderr: String::new(),
            success: true,
            cleanup_diagnostic: None,
        }
    }
    #[test]
    fn analysis_requires_compatible_hello_terminal_and_successful_exit() {
        for text in [
            "",
            "{\"event\":\"hello\",\"version\":2}\n",
            "{\"event\":\"hello\",\"version\":1}\n",
            "{\"event\":\"hello\",\"version\":1}\n{\"event\":\"complete\",\"digest\":\"bad\"}\n",
        ] {
            assert!(analysis_reply(output(text)).is_err());
        }
    }
    #[test]
    fn unknown_refresh_identity_is_not_invented_by_adapter() {
        let discovered: wire::Discovery =
            serde_json::from_value(json!({"targets":[],"default_selection":[]})).unwrap();
        let args = serde_json::from_value(json!({"targets":[{"path":"arbitrary","kind":"maven"}]}))
            .unwrap();
        assert!(choose_targets(&discovered, args).is_err());
    }
}
