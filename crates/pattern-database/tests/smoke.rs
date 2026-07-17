use pattern_database::DatabaseProperties;

#[test]
fn properties_round_trip() {
    let props = DatabaseProperties::default();
    assert_eq!(props.pattern_mode, "edge_ratio");
}
