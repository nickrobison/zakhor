use gio::File;
use tracker::{SparqlConnection, SparqlConnectionFlags};

pub fn init_db(path: &str) -> SparqlConnection {
    let store = File::for_path(path);
    let ontology =
        tracker::functions::sparql_get_ontology_nepomuk().expect("Nepomuk ontology missing");

    SparqlConnection::new(
        SparqlConnectionFlags::empty(),
        Some(&store),
        Some(&ontology),
        None::<&gio::Cancellable>,
    )
    .expect("Failed to init Tracker DB")
}
