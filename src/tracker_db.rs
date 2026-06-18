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

    let escaped_text = tracker::functions::sparql_escape_string(text)
        .ok_or_else(|| "Failed to escape text literal".to_string())?;

    let sparql = format!(
        "PREFIX nie: <http://www.semanticdesktop.org/ontologies/2007/01/19/nie#>\n\
         PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>\n\
         INSERT DATA {{\n\
             <{}> rdf:type nie:InformationElement ;\n\
                  nie:identifier \"{}\" ;\n\
                  nie:plainTextContent \"{}\" .\n\
         }}",
        uuid, uuid, escaped_text,
    );

    conn.update(&sparql, None::<&gio::Cancellable>)
        .map_err(|e| format!("Failed to store memory: {}", e))?;

    Ok(uuid.to_string())
}

pub fn read_memory(conn: &SparqlConnection, id: &str) -> Result<String, String> {
    let escaped_id = tracker::functions::sparql_escape_string(id)
        .ok_or_else(|| "Failed to escape ID".to_string())?;

    let sparql = format!(
        "PREFIX nie: <http://www.semanticdesktop.org/ontologies/2007/01/19/nie#>\n\
         PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>\n\
         SELECT ?text WHERE {{\n\
             ?id rdf:type nie:InformationElement ;\n\
                 nie:identifier \"{}\" ;\n\
                 nie:plainTextContent ?text .\n\
         }}",
        escaped_id,
    );

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
    let escaped_id = tracker::functions::sparql_escape_string(id)
        .ok_or_else(|| "Failed to escape ID".to_string())?;

    let escaped_text = tracker::functions::sparql_escape_string(text)
        .ok_or_else(|| "Failed to escape text".to_string())?;

    let sparql = format!(
        "PREFIX nie: <http://www.semanticdesktop.org/ontologies/2007/01/19/nie#>\n\
         PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>\n\
         DELETE {{\n\
             ?id nie:plainTextContent ?oldText .\n\
         }}\n\
         INSERT {{\n\
             ?id nie:plainTextContent \"{}\" .\n\
         }}\n\
         WHERE {{\n\
             ?id rdf:type nie:InformationElement ;\n\
                 nie:identifier \"{}\" ;\n\
             OPTIONAL {{ ?id nie:plainTextContent ?oldText . }}\n\
         }}",
        escaped_text, escaped_id,
    );

    conn.update(&sparql, None::<&gio::Cancellable>)
        .map_err(|e| format!("Failed to update memory: {}", e))?;

    Ok("updated".to_string())
}

pub fn delete_memory(conn: &SparqlConnection, id: &str) -> Result<(), String> {
    let escaped_id = tracker::functions::sparql_escape_string(id)
        .ok_or_else(|| "Failed to escape ID".to_string())?;

    let sparql = format!(
        "PREFIX nie: <http://www.semanticdesktop.org/ontologies/2007/01/19/nie#>\n\
         PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>\n\
         DELETE {{\n\
             ?id rdf:type nie:InformationElement .\n\
             ?id nie:identifier ?oldId .\n\
             ?id nie:plainTextContent ?oldText .\n\
         }}\n\
         WHERE {{\n\
             ?id rdf:type nie:InformationElement ;\n\
                 nie:identifier \"{}\" .\n\
             ?id nie:identifier ?oldId .\n\
             OPTIONAL {{ ?id nie:plainTextContent ?oldText . }}\n\
         }}",
        escaped_id,
    );

    conn.update(&sparql, None::<&gio::Cancellable>)
        .map_err(|e| format!("Failed to delete memory: {}", e))?;

    Ok(())
}
