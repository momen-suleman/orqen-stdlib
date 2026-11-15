//! The published micro-workflows of this package, as Orqen Library files
//! (`workflows/*.json`). Orqen imports them like any Library repository;
//! the engine's tests run them from here so the two never drift apart.

/// `(file name, JSON)`, dependencies before the workflows that use them.
pub const ALL: [(&str, &str); 3] = [
    (
        "normalize-input.json",
        include_str!("../workflows/normalize-input.json"),
    ),
    (
        "normalize-search-query.json",
        include_str!("../workflows/normalize-search-query.json"),
    ),
    (
        "calculate-percentage.json",
        include_str!("../workflows/calculate-percentage.json"),
    ),
];

#[cfg(test)]
mod tests {
    #[test]
    fn workflows_are_valid_json_with_a_contract_and_only_known_std_ops() {
        for (file, text) in super::ALL {
            let value: serde_json::Value =
                serde_json::from_str(text).unwrap_or_else(|e| panic!("{file}: {e}"));
            let graph = &value["graph"];
            assert!(
                graph["contract"]["capabilityId"]
                    .as_str()
                    .is_some_and(|id| id.starts_with("wf.std.")),
                "{file}"
            );
            for node in graph["nodes"].as_array().expect("nodes") {
                let cap = node["capability"].as_str().expect("capability");
                if crate::is_std_id(cap) {
                    assert!(crate::find(cap).is_some(), "{file}: unknown {cap}");
                }
            }
        }
    }
}
