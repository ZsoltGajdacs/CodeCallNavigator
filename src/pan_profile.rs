//! Opt-in native pan probe. Synthetic pointer events use the production drag handlers.
mod driver;
mod gpu;
#[cfg(test)]
mod tests;

use crate::app::State;
use rune::{App, Compositor, Frame, FrameEncoded, FrameOutcome};
use std::{
    cell::{Cell, RefCell},
    fs::File,
    io::{BufWriter, Write},
    path::PathBuf,
    rc::Rc,
    time::Instant,
};

struct Reports {
    origin: Instant,
    input: Cell<Option<Instant>>,
    index: Cell<u64>,
    output: RefCell<BufWriter<File>>,
}

/// Neither a compositor nor clocks/log files are installed for ordinary launches.
pub fn attach(app: App<State>) -> anyhow::Result<App<State>> {
    let Some(directory) = std::env::var_os("CODECALLNAVIGATOR_PAN_PROFILE") else {
        return Ok(app);
    };
    anyhow::ensure!(
        !rune::tooling::is_headless()
            && std::env::args().any(|arg| arg == "--native")
            && std::env::args().any(|arg| arg == "--trace"),
        "CODECALLNAVIGATOR_PAN_PROFILE requires --native --trace; headless frames cannot verify FPS"
    );
    let directory = PathBuf::from(directory);
    std::fs::create_dir_all(&directory)?;
    let reports = Rc::new(Reports {
        origin: Instant::now(),
        input: Cell::new(None),
        index: Cell::new(0),
        output: RefCell::new(BufWriter::new(File::create(
            directory.join("encoded.jsonl"),
        )?)),
    });
    let probe = Probe {
        driver: driver::Driver::default(),
        gpu: gpu::Meter::default(),
        reports: reports.clone(),
    };
    Ok(app
        .compositor(probe)
        .on_frame_encoded(move |frame| reports.encoded(frame)))
}

impl Reports {
    fn elapsed_ms(&self, at: Instant) -> f64 {
        at.saturating_duration_since(self.origin).as_secs_f64() * 1000.0
    }

    fn encoded(&self, frame: &FrameEncoded) {
        if frame.outcome() != FrameOutcome::Drawn {
            return;
        }
        let row = serde_json::json!({
            "sample": self.index.get(),
            "update_ms": frame.update_ms(),
            "encode_ms": frame.encode_ms(),
            "refresh_hz": frame.refresh_hz(),
            "synthetic_request_ms": self.input.get().map(|at| self.elapsed_ms(at)),
            "frame_started_ms": frame.started().map(|at| self.elapsed_ms(at)),
            "cpu_submitted_ms": self.elapsed_ms(frame.at()),
        });
        let mut output = self.output.borrow_mut();
        if let Err(error) = writeln!(output, "{row}") {
            eprintln!("pan profile: cannot write frame timing: {error}");
        }
    }
}

struct Probe {
    driver: driver::Driver,
    gpu: gpu::Meter,
    reports: Rc<Reports>,
}

impl Compositor<State> for Probe {
    fn features(&self) -> wgpu::Features {
        wgpu::Features::TIMESTAMP_QUERY
    }

    fn render(&mut self, frame: &mut Frame<'_, State>) {
        let index = self.reports.index.get() + 1;
        self.reports.index.set(index);
        self.reports.input.set(self.driver.requested());
        self.driver.sample(frame.rt);
        frame
            .rt
            .trace_channel("codecallnavigator.sample", "level", index as f32);
        frame
            .rt
            .trace_channel("codecallnavigator.update_ms", "level", frame.update_ms as f32);
        frame
            .rt
            .trace_channel("codecallnavigator.target.width", "level", frame.size.0 as f32);
        frame
            .rt
            .trace_channel("codecallnavigator.target.height", "level", frame.size.1 as f32);
        self.gpu.draw(frame, index);
        // Input queued here reaches the next Runtime::frame through the same handlers as a mouse.
        self.driver.next(frame.rt);
    }

    fn animating(&self) -> bool {
        !self.driver.finished()
    }
}
