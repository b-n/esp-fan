use core::fmt;
use portable_atomic::{AtomicF64, AtomicUsize, Ordering};

// Traits
pub trait Metric {
    fn write<F: fmt::Write>(&self, writer: &mut F) -> fmt::Result;
}

pub struct GaugeFamily<'a, const METRICS: usize, M = Gauge> {
    name: &'a str,
    help: &'a str,
    unit: &'a str,
    metrics: [M; METRICS],
}

impl<'a, const METRICS: usize> GaugeFamily<'a, METRICS> {
    pub fn new(&'a name, &'a help, &'a unit) -> Self {
        Self {
            name,
            help,
            unit,
            metrics: [Gauge::new(); METRICS],
        }
    }
}

// [f64]Gauge
#[derive(Debug, Default)]
pub struct Gauge {
    val: AtomicF64,
}

impl Gauge {
    pub const fn new() -> Self {
        Self {
            val: AtomicF64::new(0.0),
        }
    }

    pub fn set(&mut self, val: f64) {
        self.val.store(val, Ordering::Relaxed)
    }

    pub fn value(&self) -> f64 {
        self.val.load(Ordering::Relaxed)
    }
}

impl Metric for Gauge {
    fn write<F: fmt::Write>(&self, writer: &mut F) -> fmt::Result {
        write!(writer, "{} {{", self.name)?;
        write!(writer, "{}", self.labels)?;
        write!(writer, "}} {}", self.value())
    }
}

// IntGauge
#[derive(Default, Debug)]
pub struct IntGauge {
    name: &'static str,
    labels: &'static str,
    val: AtomicUsize,
}

impl IntGauge {
    pub const fn new(name: &'static str, labels: &'static str) -> Self {
        Self {
            name,
            labels,
            val: AtomicUsize::new(0),
        }
    }

    pub fn set(&self, val: usize) {
        self.val.store(val, Ordering::Relaxed)
    }

    pub fn value(&self) -> usize {
        self.val.load(Ordering::Relaxed)
    }
}

impl Metric for IntGauge {
    fn write<F: fmt::Write>(&self, writer: &mut F) -> fmt::Result {
        write!(writer, "{} {{", self.name)?;
        write!(writer, "{}", self.labels)?;
        writeln!(writer, "}} {}", self.value())
    }
}

// Counter
#[derive(Default, Debug)]
pub struct Counter {
    name: &'static str,
    labels: &'static str,
    val: AtomicUsize,
}

impl Counter {
    pub const fn new(name: &'static str, labels: &'static str) -> Self {
        Self {
            name,
            labels,
            val: AtomicUsize::new(0),
        }
    }

    pub fn inc(&self, val: usize) -> usize {
        self.val.fetch_add(val, Ordering::Relaxed)
    }

    pub fn value(&self) -> usize {
        self.val.load(Ordering::Relaxed)
    }
}

impl Metric for Counter {
    fn write<F: fmt::Write>(&self, writer: &mut F) -> fmt::Result {
        write!(writer, "{} {{", self.name)?;
        write!(writer, "{}", self.labels)?;
        writeln!(writer, "}} {}", self.value())
    }
}
