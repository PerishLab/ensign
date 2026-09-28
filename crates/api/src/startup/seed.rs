pub(crate) struct Seed<'a> {
    pub who: &'a str,
    pub verb: &'a str,
    pub unit: &'a str,
    pub scope: &'a str,
}

pub(crate) fn wire<'a>(seeds: &[Seed<'a>]) -> Vec<(&'a str, &'a str, &'a str, &'a str)> {
    seeds
        .iter()
        .map(|seed| (seed.who, seed.verb, seed.unit, seed.scope))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{Seed, wire};

    #[test]
    fn order() {
        let seeds = [
            Seed {
                who: "all",
                verb: "see",
                unit: "Actor",
                scope: "all",
            },
            Seed {
                who: "7",
                verb: "put",
                unit: "Profile",
                scope: "own",
            },
        ];
        assert_eq!(
            wire(&seeds),
            vec![
                ("all", "see", "Actor", "all"),
                ("7", "put", "Profile", "own")
            ]
        );
    }
}
