use core::fmt;
use utils::prom::{Counter, Metric};

pub static METRICS: Metrics = Metrics::new();

#[derive(Debug)]
pub struct Metrics {
    pub http_get_requests: Counter,
    pub http_post_requests: Counter,
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

impl Metrics {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            http_get_requests: Counter::new("http_request_count", "method=\"GET\""),
            http_post_requests: Counter::new("http_request_count", "method=\"POST\""),
        }
    }

    pub fn write_http_body<T: fmt::Write>(&self, writer: &mut T) -> fmt::Result {
        self.http_get_requests.write(writer)?;
        self.http_post_requests.write(writer)
    }
}
