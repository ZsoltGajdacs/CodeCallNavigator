//! Dispatch keeps the ordinary probe and report unchanged.
use super::{comparison, config::Configuration, driver, io::Worker, report::Report};
use crate::app::State;
use rune::{FrameEncoded, Runtime, WindowHandle};
use std::{cell::RefCell, rc::Rc, time::Instant};
pub(super) enum Driver {
    Single(Box<driver::Driver>),
    Comparison(Box<comparison::Driver>),
    Expansion(Box<super::expansion::Driver>),
}
impl Driver {
    pub fn new(configuration: Configuration, worker: Worker, origin: Instant) -> Self {
        match configuration {
            Configuration::Single(configuration) => {
                let report = Rc::new(RefCell::new(Report::new(&configuration)));
                Self::Single(Box::new(driver::Driver::new(
                    configuration,
                    worker,
                    report,
                    origin,
                )))
            }
            Configuration::Comparison(configuration) => Self::Comparison(Box::new(
                comparison::Driver::new(*configuration, worker, origin),
            )),
            Configuration::Expansion(configuration) => Self::Expansion(Box::new(
                super::expansion::Driver::new(*configuration, worker, origin),
            )),
        }
    }
    pub fn window_ready(&mut self, window: &WindowHandle) {
        match self {
            Self::Single(driver) => driver.window_ready(window),
            Self::Comparison(driver) => driver.window_ready(window),
            Self::Expansion(driver) => driver.window_ready(window),
        }
    }
    pub fn advance(&mut self, runtime: &mut Runtime<State>) {
        match self {
            Self::Single(driver) => driver.advance(runtime),
            Self::Comparison(driver) => driver.advance(runtime),
            Self::Expansion(driver) => driver.advance(runtime),
        }
    }
    pub fn observed(&mut self, runtime: &Runtime<State>) {
        match self {
            Self::Single(driver) => driver.observed(runtime),
            Self::Comparison(driver) => driver.observed(runtime),
            Self::Expansion(driver) => driver.observed(runtime),
        }
    }
    pub fn encoded(&mut self, frame: &FrameEncoded) {
        match self {
            Self::Single(driver) => driver.encoded(frame),
            Self::Comparison(driver) => driver.encoded(frame),
            Self::Expansion(driver) => driver.encoded(frame),
        }
    }
    pub fn finished(&self) -> bool {
        match self {
            Self::Single(driver) => driver.finished(),
            Self::Comparison(driver) => driver.finished(),
            Self::Expansion(driver) => driver.finished(),
        }
    }
}
