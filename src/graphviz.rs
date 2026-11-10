//! Shared bounded Graphviz process execution; layout policy belongs to each caller.
use anyhow::{Context, Result, bail};
use serde_json::Value;
use std::{
    io::Write,
    path::Path,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

pub fn run(dot: &Path, source: &str) -> Result<Vec<u8>> {
    let mut command = Command::new(dot);
    let plugins = dot
        .parent()
        .and_then(Path::parent)
        .map(|path| path.join("lib/graphviz"));
    if let Some(path) = plugins.filter(|path| path.is_dir()) {
        command.env("GVBINDIR", path);
    }
    let mut child = command
        .arg("-Tjson")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Cannot start Graphviz; install or bundle dot")?;
    child
        .stdin
        .take()
        .context("Graphviz input unavailable")?
        .write_all(source.as_bytes())?;
    // Drain pipes while dot works so a large JSON result cannot fill stdout and deadlock.
    let output = child.stdout.take().expect("piped stdout");
    let errors = child.stderr.take().expect("piped stderr");
    let reader = thread::spawn(move || {
        use std::io::Read;
        let mut data = Vec::new();
        let mut stream = output;
        stream.read_to_end(&mut data).map(|_| data)
    });
    let error_reader = thread::spawn(move || {
        use std::io::Read;
        let mut data = String::new();
        let mut stream = errors;
        stream.read_to_string(&mut data).map(|_| data)
    });
    let deadline = Instant::now() + Duration::from_secs(15);
    while child.try_wait()?.is_none() {
        if Instant::now() > deadline {
            child.kill()?;
            child.wait()?;
            bail!("Graphviz layout timed out; narrow the neighborhood or collapse groups")
        }
        thread::sleep(Duration::from_millis(20));
    }
    let data = reader
        .join()
        .map_err(|_| anyhow::anyhow!("Graphviz output worker failed"))??;
    let errors = error_reader
        .join()
        .map_err(|_| anyhow::anyhow!("Graphviz error worker failed"))??;
    if !child.wait()?.success() {
        bail!("Graphviz layout failed: {errors}")
    }
    Ok(data)
}

pub fn coordinates(value: &str) -> Result<Vec<f32>> {
    value
        .split(',')
        .map(|part| part.parse().context("Invalid Graphviz coordinate"))
        .collect()
}

pub fn spline_points(edge: &Value, height: f32) -> Vec<[f32; 2]> {
    edge["_draw_"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|op| op["op"] == "b" || op["op"] == "B")
        .flat_map(|op| op["points"].as_array().into_iter().flatten())
        .filter_map(|point| {
            Some([
                point[0].as_f64()? as f32 + 16.0,
                height - point[1].as_f64()? as f32 + 16.0,
            ])
        })
        .collect()
}
