//! Live data over HTTP (currency rates, etc.).

/// Minimal percent-encoder for query strings.
pub fn urlencode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            b' ' => "%20".to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// USD based exchange rates -> Vec<(code_lowercase, rate)>. Empty on failure.
pub fn currency_rates() -> Vec<(String, f64)> {
    let body = match ureq::get("https://open.er-api.com/v6/latest/USD")
        .call()
        .ok()
        .and_then(|r| r.into_string().ok())
    {
        Some(b) => b,
        None => return Vec::new(),
    };
    let v: serde_json::Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let mut out = Vec::new();
    if let Some(rates) = v["rates"].as_object() {
        for (k, val) in rates {
            if let Some(r) = val.as_f64() {
                out.push((k.to_lowercase(), r));
            }
        }
    }
    out
}
