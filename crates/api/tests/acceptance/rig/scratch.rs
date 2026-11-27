use keel::Wire;
use keel::adapt::pg::Postgres;
use tokio::runtime::{Builder, Runtime};
use url::Url;

pub struct Scratch {
    maintenance: String,
    name: String,
    url: String,
    spin: Runtime,
}

impl Scratch {
    pub fn open() -> Option<Self> {
        let maintenance = store()?;
        let mut bytes = [0; 16];
        getrandom::fill(&mut bytes).expect("scratch database name");
        let name = format!("ensign_act_{:032x}", u128::from_ne_bytes(bytes));
        let url = retarget(&maintenance, &name);
        let spin = Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("scratch database runtime");
        spin.block_on(change(
            &maintenance,
            &format!("CREATE DATABASE {name} TEMPLATE template0"),
        ))
        .expect("create scratch database");
        Some(Self {
            maintenance,
            name,
            url,
            spin,
        })
    }

    pub fn url(&self) -> &str {
        &self.url
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let sql = format!("DROP DATABASE {} WITH (FORCE)", self.name);
        let result = self.spin.block_on(change(&self.maintenance, &sql));
        if let Err(err) = result {
            if std::thread::panicking() {
                eprintln!("scratch database cleanup failed for {}: {err}", self.name);
            } else {
                panic!("scratch database cleanup failed for {}: {err}", self.name);
            }
        }
    }
}

async fn change(url: &str, sql: &str) -> Result<(), keel::adapt::Error> {
    Postgres::at(url).await?.script(sql).await
}

fn store() -> Option<String> {
    ["ENSIGN_STORE_URL", "API_STORE_URL"]
        .into_iter()
        .filter_map(|key| std::env::var(key).ok())
        .find(|url| !url.is_empty())
}

fn retarget(url: &str, name: &str) -> String {
    if url.contains("://") {
        let mut url = Url::parse(url).expect("parse scratch database URL");
        url.set_path(name);
        if url.query_pairs().any(|(key, _)| key == "dbname") {
            let pairs: Vec<_> = url
                .query_pairs()
                .into_owned()
                .filter(|(key, _)| key != "dbname")
                .collect();
            url.set_query(None);
            if !pairs.is_empty() {
                url.query_pairs_mut().extend_pairs(pairs);
            }
        }
        return url.into();
    }
    let mut pairs: Vec<_> = url
        .split_whitespace()
        .filter(|pair| !pair.starts_with("dbname="))
        .map(str::to_owned)
        .collect();
    pairs.push(format!("dbname={name}"));
    pairs.join(" ")
}
