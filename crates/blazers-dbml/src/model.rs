//! Domain model for a parsed DBML document.
//!
//! Mirrors the Go `dbml` port: a [`Project`] holds tables, enums, top-level
//! `Ref:` relationships and `Note:` comments. Generators live in
//! [`crate::generate`] and [`crate::svg`].

/// One column inside a [`Table`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Column {
    pub name: String,
    pub typ: String,
    /// Raw settings that did not map to a known flag (`increment`, …).
    pub settings: Vec<String>,
    pub comment: String,
    pub is_primary: bool,
    pub is_unique: bool,
    pub is_not_null: bool,
    pub default: String,
    pub ref_table: String,
    pub ref_column: String,
}

/// A `Table name { … }` block.
#[derive(Debug, Clone, Default)]
pub struct Table {
    pub name: String,
    pub columns: Vec<Column>,
    pub indexes: Vec<Index>,
    pub comment: String,
}

/// An `Index`/`indexes { … }` entry.
#[derive(Debug, Clone, Default)]
pub struct Index {
    pub name: String,
    pub columns: Vec<String>,
    pub unique: bool,
    pub kind: String,
}

/// A top-level `Ref: name a.b < c.d` relationship.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct Reference {
    pub name: String,
    pub table1: String,
    pub column1: String,
    pub table2: String,
    pub column2: String,
}

/// An `Enum name { … }` block.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct Enum {
    pub name: String,
    pub values: Vec<String>,
}

/// The whole parsed document.
#[derive(Debug, Clone, Default)]
pub struct Project {
    pub name: String,
    pub tables: Vec<Table>,
    pub enums: Vec<Enum>,
    pub references: Vec<Reference>,
    pub comments: std::collections::BTreeMap<String, String>,
}

impl Project {
    pub fn get_table(&self, name: &str) -> Option<&Table> {
        self.tables.iter().find(|t| t.name == name)
    }

    pub fn get_column(&self, table: &str, column: &str) -> Option<&Column> {
        self.get_table(table)?
            .columns
            .iter()
            .find(|c| c.name == column)
    }
}
