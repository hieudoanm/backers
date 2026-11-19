//! JSON summary of a parsed project — backs the `dbml_parse` MCP tool and
//! the `/api/dbml/parse` surface.

use serde::Serialize;

use crate::model::{Project, Table};

#[derive(Serialize)]
struct TableSummary<'a> {
    name: &'a str,
    columns: Vec<&'a str>,
}

#[derive(Serialize)]
struct Summary<'a> {
    name: &'a str,
    tables: Vec<TableSummary<'a>>,
    enums: &'a [crate::model::Enum],
    references: &'a [crate::model::Reference],
}

/// Serializes a project as indented JSON: name, tables (name + column names),
/// enums and references.
pub fn summarize(project: &Project) -> Result<String, serde_json::Error> {
    let tables: Vec<TableSummary> = project
        .tables
        .iter()
        .map(|table: &Table| TableSummary {
            name: &table.name,
            columns: table.columns.iter().map(|c| c.name.as_str()).collect(),
        })
        .collect();
    serde_json::to_string_pretty(&Summary {
        name: &project.name,
        tables,
        enums: &project.enums,
        references: &project.references,
    })
}
