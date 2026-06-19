use rdf_types::dataset::{BTreeDataset, TraversableDataset};
use rdf_types::Quad;

const GRAPH_PREFIX: &str = "http://zakhor/ns/graph/";

/// Tracks provenance of observations using named graphs.
///
/// Each observation is stored as a set of quads in a named graph
/// identified by `zakhor:graph/{observation-uuid}`.
///
/// The underlying [`BTreeDataset<String>`] stores all four quad
/// components (subject, predicate, object, graph name) as plain
/// strings.  The graph name is stored as `Option<String>` inside
/// each [`Quad`].
pub struct ProvenanceTracker {
    dataset: BTreeDataset<String>,
}

impl ProvenanceTracker {
    /// Creates an empty provenance tracker.
    pub fn new() -> Self {
        Self {
            dataset: BTreeDataset::new(),
        }
    }

    /// Creates a named graph `zakhor:graph/{uuid}` and inserts the given
    /// triples as quads.
    ///
    /// Each triple `(s, p, o)` is stored as a quad in the named graph.
    /// Existing quads with the same values are silently ignored (the
    /// underlying `BTreeDataset` is a set).
    pub fn add_observation(&mut self, uuid: &str, triples: Vec<(String, String, String)>) {
        let graph_name = format!("{}{}", GRAPH_PREFIX, uuid);
        for (s, p, o) in triples {
            let quad = Quad::new(s, p, o, Some(graph_name.clone()));
            self.dataset.insert(quad);
        }
    }

    /// Queries all triples `(s, p, o)` stored in the named graph
    /// `zakhor:graph/{uuid}`.
    ///
    /// Returns an empty `Vec` if the graph does not exist.
    pub fn get_observation_graph(&self, uuid: &str) -> Vec<(String, String, String)> {
        let graph_name = format!("{}{}", GRAPH_PREFIX, uuid);
        self.dataset
            .quads()
            .filter(|q| q.graph() == Some(&&graph_name))
            .map(|q| {
                (
                    q.subject().to_string(),
                    q.predicate().to_string(),
                    q.object().to_string(),
                )
            })
            .collect()
    }

    /// Returns the UUIDs of all observed (non-empty) named graphs.
    ///
    /// The order is **not** guaranteed.
    pub fn all_observations(&self) -> Vec<String> {
        let mut uuids: Vec<String> = Vec::new();
        for quad in self.dataset.quads() {
            if let Some(gn) = quad.graph() {
                if let Some(uuid) = gn.strip_prefix(GRAPH_PREFIX) {
                    let uuid = uuid.to_string();
                    if !uuids.contains(&uuid) {
                        uuids.push(uuid);
                    }
                }
            }
        }
        uuids
    }

    /// Returns `true` if the named graph `zakhor:graph/{uuid}` contains at
    /// least one quad.
    pub fn contains_observation(&self, uuid: &str) -> bool {
        let graph_name = format!("{}{}", GRAPH_PREFIX, uuid);
        self.dataset
            .quads()
            .any(|q| q.graph() == Some(&&graph_name))
    }

    /// Removes all quads from the tracker.
    pub fn clear(&mut self) {
        let quads: Vec<Quad<String>> = self.dataset.quads().map(|q| q.cloned()).collect();
        for quad in quads {
            self.dataset.remove(quad.as_ref());
        }
    }
}

impl Default for ProvenanceTracker {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_tracker_empty() {
        let tracker = ProvenanceTracker::new();
        assert!(tracker.all_observations().is_empty());
        assert!(!tracker.contains_observation("any-uuid"));
        assert!(tracker.get_observation_graph("any-uuid").is_empty());
    }

    #[test]
    fn test_add_observation_creates_graph() {
        let mut tracker = ProvenanceTracker::new();
        let triples = vec![
            (
                "http://example.com/s1".into(),
                "http://example.com/p1".into(),
                "http://example.com/o1".into(),
            ),
            (
                "http://example.com/s2".into(),
                "http://example.com/p2".into(),
                "http://example.com/o2".into(),
            ),
        ];
        tracker.add_observation("obs-1", triples);

        let result = tracker.get_observation_graph("obs-1");
        assert_eq!(result.len(), 2);
        assert!(result.contains(&(
            "http://example.com/s1".into(),
            "http://example.com/p1".into(),
            "http://example.com/o1".into(),
        )));
        assert!(result.contains(&(
            "http://example.com/s2".into(),
            "http://example.com/p2".into(),
            "http://example.com/o2".into(),
        )));
    }

    #[test]
    fn test_get_observation_graph_returns_correct_triples() {
        let mut tracker = ProvenanceTracker::new();
        let triples = vec![(
            "urn:subj:A".into(),
            "urn:pred:type".into(),
            "urn:obj:Class".into(),
        )];
        tracker.add_observation("obs-triple-1", triples);

        let result = tracker.get_observation_graph("obs-triple-1");
        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0],
            (
                "urn:subj:A".to_string(),
                "urn:pred:type".to_string(),
                "urn:obj:Class".to_string(),
            )
        );
    }

    #[test]
    fn test_contains_observation() {
        let mut tracker = ProvenanceTracker::new();
        assert!(!tracker.contains_observation("obs-abc"));

        let triples = vec![("urn:s".into(), "urn:p".into(), "urn:o".into())];
        tracker.add_observation("obs-abc", triples);

        assert!(tracker.contains_observation("obs-abc"));
        assert!(!tracker.contains_observation("obs-xyz"));
    }

    #[test]
    fn test_multiple_observations_isolated() {
        let mut tracker = ProvenanceTracker::new();

        tracker.add_observation(
            "alpha",
            vec![("urn:a".into(), "urn:pa".into(), "urn:oa".into())],
        );
        tracker.add_observation(
            "beta",
            vec![("urn:b".into(), "urn:pb".into(), "urn:ob".into())],
        );

        // Alpha should only contain the alpha triple
        let alpha = tracker.get_observation_graph("alpha");
        assert_eq!(alpha.len(), 1);
        assert_eq!(alpha[0].0, "urn:a");

        // Beta should only contain the beta triple
        let beta = tracker.get_observation_graph("beta");
        assert_eq!(beta.len(), 1);
        assert_eq!(beta[0].0, "urn:b");

        // Both should appear in all_observations
        let all = tracker.all_observations();
        assert!(all.contains(&"alpha".to_string()));
        assert!(all.contains(&"beta".to_string()));
    }

    #[test]
    fn test_clear_removes_all() {
        let mut tracker = ProvenanceTracker::new();

        tracker.add_observation(
            "obs-1",
            vec![("urn:s1".into(), "urn:p1".into(), "urn:o1".into())],
        );
        tracker.add_observation(
            "obs-2",
            vec![("urn:s2".into(), "urn:p2".into(), "urn:o2".into())],
        );
        assert!(tracker.contains_observation("obs-1"));
        assert!(tracker.contains_observation("obs-2"));

        tracker.clear();

        assert!(!tracker.contains_observation("obs-1"));
        assert!(!tracker.contains_observation("obs-2"));
        assert!(tracker.all_observations().is_empty());
        assert!(tracker.get_observation_graph("obs-1").is_empty());
    }
}
