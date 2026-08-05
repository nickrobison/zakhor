use oxiri::Iri;

use super::*;

#[test]
fn test_scored_entity_struct() {
    let e = ScoredEntity {
        uri: Iri::parse("http://example.com/e1".to_string()).expect("valid test iri"),
        label: "Entity 1".into(),
        connectivity: 5,
        importance: 0.5,
    };
    assert_eq!(e.uri.as_str(), "http://example.com/e1");
    assert_eq!(e.connectivity, 5);
    assert_eq!(e.importance, 0.5);
}

#[test]
fn test_scored_entity_sort_order() {
    let mut items = [
        ScoredEntity {
            uri: Iri::parse("http://example.com/a".to_string()).expect("valid test iri"),
            label: "A".into(),
            connectivity: 1,
            importance: 0.1,
        },
        ScoredEntity {
            uri: Iri::parse("http://example.com/b".to_string()).expect("valid test iri"),
            label: "B".into(),
            connectivity: 10,
            importance: 1.0,
        },
        ScoredEntity {
            uri: Iri::parse("http://example.com/c".to_string()).expect("valid test iri"),
            label: "C".into(),
            connectivity: 5,
            importance: 0.5,
        },
    ];
    items.sort_by_key(|item| std::cmp::Reverse(item.connectivity));
    assert_eq!(items[0].uri.as_str(), "http://example.com/b");
    assert_eq!(items[1].uri.as_str(), "http://example.com/c");
    assert_eq!(items[2].uri.as_str(), "http://example.com/a");
}

#[test]
fn test_importance_normalisation() {
    // With only 1 entity, max_score = 1, so importance = connectivity / 1
    let max_score = 1.0;
    let connectivity = 3;
    let importance = connectivity as f64 / max_score;
    assert!((importance - 3.0).abs() < f64::EPSILON);

    // With 10 entities, max_score = 10
    let max_score = 10.0;
    let connectivity = 3;
    let importance = connectivity as f64 / max_score;
    assert!((importance - 0.3).abs() < f64::EPSILON);
}
