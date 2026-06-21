use libm::sqrtf;

#[must_use]
pub fn mean_and_stddev(values: &[f32]) -> (f32, f32) {
    #[allow(clippy::cast_precision_loss)]
    let samples = values.len() as f32;
    let sum: f32 = values.iter().sum();
    let mean = sum / samples;
    let diffs: f32 = values
        .iter()
        .fold(0.0, |acc, v| acc + (v - mean) * (v - mean));

    (mean, sqrtf(diffs / samples))
}

pub type Range<T> = (T, T);
pub type Domain<T> = (T, T);

#[derive(Debug)]
pub struct RangeDomainMapper<T> {
    range: Range<T>,
    domain: Domain<T>,
}

impl<T> RangeDomainMapper<T>
where
    T: libm::support::Int<Unsigned = T>,
{
    pub fn new(range: Range<T>, domain: Domain<T>) -> Self {
        Self { range, domain }
    }

    #[must_use]
    pub fn value(&self, value: &T) -> T {
        let clamped = (*value).clamp(self.range.0, self.range.1);

        // deref the range/domain
        let (range_start, range_end) = self.range;
        let (domain_start, domain_end) = self.domain;

        // Get the span of both range and domain
        let range_span = range_start.abs_diff(range_end);
        let domain_span = domain_start.abs_diff(domain_end);
        // And whether the domain is inverted
        let inverse_domain = domain_start > domain_end;

        // how far is the value from the range minimum
        let value_offset = clamped - range_start;

        // do the math. Takes value_offset/range_span (e.g. position in range), and multiples by
        // domain_span for position in the domain. Is the offset by the domain_start to get the
        // true value in the domain. Math is done here to preserve integers etc.

        if inverse_domain {
            let offset = value_offset * domain_span / range_span;
            domain_start - offset
        } else {
            let offset = value_offset * domain_span / range_span;
            domain_start + offset
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::maths::mean_and_stddev;
    use libm::sqrtf;

    #[test]
    fn mean_and_stddev_success() {
        assert_eq!(mean_and_stddev(&[0.0, 4.0]), (2.0, 2.0));
        assert_eq!(mean_and_stddev(&[0.0, 2.0]), (1.0, 1.0));
        assert_eq!(mean_and_stddev(&[0.0, 2.0, 2.0, 4.0]), (2.0, sqrtf(2.0)));
    }
}
