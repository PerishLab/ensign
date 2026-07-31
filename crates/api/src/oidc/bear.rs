pub(crate) struct Bearing {
    pub(crate) actor: i64,
    pub(crate) client: String,
    pub(crate) scope: String,
    pub(crate) audience: String,
}

pub(crate) struct Code {
    pub(crate) bearing: Bearing,
    pub(crate) redirect: String,
    pub(crate) challenge: String,
    pub(crate) nonce: Option<String>,
    pub(crate) dies: i64,
}

pub(crate) struct Ward {
    pub(crate) row: i64,
    pub(crate) bearing: Bearing,
}
