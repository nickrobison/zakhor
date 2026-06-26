pub struct SpatialIndex {
    pub name: String,
}

pub fn create_index(name: &str) -> SpatialIndex {
    SpatialIndex {
        name: name.to_string(),
    }
}
