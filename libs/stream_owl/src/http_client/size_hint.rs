use http::{self, header, HeaderValue};

#[derive(Default, Clone, Copy, Debug)]
pub(crate) struct SizeHint {
    estimate_bounds: Option<(u64, u64)>,
}

impl SizeHint {
    fn add(&mut self, estimate: u64) {
        match self.estimate_bounds.as_mut() {
            Some((lower, higer)) if estimate < *lower => *lower = estimate,
            Some((lower, higer)) if estimate > *higer => *higer = estimate,
            None => self.estimate_bounds = Some((estimate, estimate)),
            _ => (),
        }
    }

    pub(crate) fn update_from_headers(&mut self, headers: &http::HeaderMap) {
        if let Some(content_length) = headers
            .get(header::CONTENT_LENGTH)
            .map(HeaderValue::to_str)
            .and_then(Result::ok)
            .map(|len| u64::from_str_radix(len, 10))
            .and_then(Result::ok)
        {
            self.add(content_length);
        }

        if let Some(range_total) = headers
            .get(header::CONTENT_RANGE)
            .map(HeaderValue::to_str)
            .and_then(Result::ok)
            .filter(|range| range.starts_with("bytes"))
            .and_then(|range| range.rsplit_once("/"))
            .map(|(_, total)| u64::from_str_radix(total, 10))
            .and_then(Result::ok)
        {
            self.add(range_total);
        }
    }

    /// Data size could still be more
    pub(crate) fn highest_estimate(&self) -> Option<u64> {
        self.estimate_bounds.clone().map(|(_lower, higer)| higer)
    }

    /// Data size can still be less
    pub(crate) fn lowest_estimate(&self) -> Option<u64> {
        self.estimate_bounds.clone().map(|(lower, _higer)| lower)
    }
}
