//! Explicit native acceptance for a normally loaded, CLI-backed C# workspace.
mod comparison;
mod config;
mod driver;
mod input;
mod io;
mod mode;
mod observation;
mod report;

use crate::app::State;
use anyhow::Result;
use config::Configuration;
use mode::Driver;
use rune::{App, Compositor, Frame};
use std::{cell::RefCell, path::PathBuf, rc::Rc, time::Instant};

/// Ordinary launches install no probe, clocks, files or extra callbacks.
pub fn attach(app: App<State>) -> Result<App<State>> {
    let Some(directory) = std::env::var_os("CODECALLNAVIGATOR_FLOW_ACCEPTANCE") else {
        return Ok(app);
    };
    config::validate_launch(
        rune::tooling::is_headless(),
        &std::env::args().collect::<Vec<_>>(),
        std::env::var_os("CODECALLNAVIGATOR_PAN_PROFILE").is_some(),
    )?;
    let directory = PathBuf::from(directory);
    let configuration = Configuration::load(&directory)?;
    let worker = io::Worker::start(directory, configuration.timeout())?;
    let origin = Instant::now();
    let driver = Rc::new(RefCell::new(Driver::new(configuration, worker, origin)));
    let encoded_driver = driver.clone();
    let ready_driver = driver.clone();
    let drawn_driver = driver.clone();
    Ok(app
        .compositor(Probe(driver))
        .on_ready(move |window| ready_driver.borrow_mut().window_ready(window))
        .on_frame_drawn(move |runtime| drawn_driver.borrow_mut().observed(runtime))
        .on_frame_encoded(move |frame| encoded_driver.borrow_mut().encoded(frame)))
}

struct Probe(Rc<RefCell<Driver>>);

impl Compositor<State> for Probe {
    fn render(&mut self, frame: &mut Frame<'_, State>) {
        frame.draw_ui();
        self.0.borrow_mut().advance(frame.rt);
    }

    fn animating(&self) -> bool {
        !self.0.borrow().finished()
    }

    fn draws_own(&self) -> bool {
        false
    }
}
