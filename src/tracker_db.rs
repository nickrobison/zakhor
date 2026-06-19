use gio::File;
use tracker::prelude::*;
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

pub fn store_memory(conn: &SparqlConnection, text: &str) -> Result<String, String> {
    let uuid = tracker::functions::sparql_get_uuid_urn()
        .ok_or_else(|| "Failed to generate UUID".to_string())?;

    let sparql = crate::sparql::SparqlBuilder::insert_data(&uuid, text);

    conn.update(&sparql, None::<&gio::Cancellable>)
        .map_err(|e| format!("Failed to store memory: {}", e))?;

    Ok(uuid.to_string())
}

pub fn read_memory(conn: &SparqlConnection, id: &str) -> Result<String, String> {
    let sparql = crate::sparql::SparqlBuilder::select(id);

    let cursor = conn
        .query(&sparql, None::<&gio::Cancellable>)
        .map_err(|e| format!("Failed to read memory: {}", e))?;

    if cursor
        .next(None::<&gio::Cancellable>)
        .map_err(|e| format!("Cursor error: {}", e))?
    {
        cursor
            .string(0)
            .map(|s| s.to_string())
            .ok_or_else(|| format!("Empty text content for: {}", id))
    } else {
        Err(format!("Memory not found: {}", id))
    }
}

pub fn update_memory(conn: &SparqlConnection, id: &str, text: &str) -> Result<String, String> {
    let sparql = crate::sparql::SparqlBuilder::delete_insert_where(id, text);

    conn.update(&sparql, None::<&gio::Cancellable>)
        .map_err(|e| format!("Failed to update memory: {}", e))?;

    Ok("updated".to_string())
}

pub fn delete_memory(conn: &SparqlConnection, id: &str) -> Result<(), String> {
    let sparql = crate::sparql::SparqlBuilder::delete_data(id);

    conn.update(&sparql, None::<&gio::Cancellable>)
        .map_err(|e| format!("Failed to delete memory: {}", e))?;

    Ok(())
}
