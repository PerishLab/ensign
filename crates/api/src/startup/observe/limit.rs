use axum::http::HeaderMap;
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Mutex;
use std::time::{Duration, Instant};

const KEYS: usize = 4096;

struct Bucket {
    tokens: f64,
    seen: Instant,
}

pub(crate) struct Meter {
    rate: f64,
    cap: f64,
    stale: Duration,
    state: Mutex<HashMap<String, Bucket>>,
}

impl Meter {
    pub(crate) fn new(rpm: u32, burst: u32) -> Result<Self, String> {
        if rpm == 0 || burst == 0 {
            return Err("locus rpm and burst must be positive".into());
        }
        let rate = f64::from(rpm) / 60.0;
        let cap = f64::from(burst);
        let fill = (cap / rate).ceil().max(60.0);
        Ok(Self {
            rate,
            cap,
            stale: Duration::from_secs(fill as u64),
            state: Mutex::new(HashMap::new()),
        })
    }

    pub(crate) fn take(&self, key: String) -> Result<(), u64> {
        let now = Instant::now();
        let mut map = self
            .state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        if !map.contains_key(&key) && map.len() >= KEYS {
            evict(&mut map, now, self.stale);
        }
        let bucket = map.entry(key).or_insert(Bucket {
            tokens: self.cap,
            seen: now,
        });
        let elapsed = now.saturating_duration_since(bucket.seen).as_secs_f64();
        bucket.tokens = (bucket.tokens + elapsed * self.rate).min(self.cap);
        bucket.seen = now;
        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            return Ok(());
        }
        let wait = ((1.0 - bucket.tokens) / self.rate).ceil().max(1.0) as u64;
        Err(wait)
    }
}

fn evict(map: &mut HashMap<String, Bucket>, now: Instant, stale: Duration) {
    map.retain(|_, bucket| now.saturating_duration_since(bucket.seen) < stale);
    if map.len() < KEYS {
        return;
    }
    let oldest = map
        .iter()
        .min_by_key(|(_, bucket)| bucket.seen)
        .map(|(key, _)| key.clone());
    if let Some(victim) = oldest {
        map.remove(&victim);
    }
}

pub(crate) struct Source<'a>(pub(crate) &'a HeaderMap);

impl Source<'_> {
    pub(crate) fn caller(&self, peer: Option<SocketAddr>, forwarded: bool) -> String {
        if forwarded && let Some(ip) = self.real().or_else(|| self.forwarded()) {
            return ip.to_string();
        }
        peer.map(|addr| addr.ip().to_string())
            .unwrap_or_else(|| "default".to_string())
    }

    fn real(&self) -> Option<IpAddr> {
        self.0
            .get_all("x-real-ip")
            .iter()
            .next_back()?
            .to_str()
            .ok()?
            .trim()
            .parse()
            .ok()
    }

    fn forwarded(&self) -> Option<IpAddr> {
        let raw = self
            .0
            .get_all("x-forwarded-for")
            .iter()
            .next_back()?
            .to_str()
            .ok()?;
        raw.rsplit(',').next()?.trim().parse().ok()
    }
}
