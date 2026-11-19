//! DBML parser — a faithful port of the Go `internal/dbml` line scanner.
//!
//! DBML is line-oriented: `Table name {`, `Enum name {`, `Ref: …`, `Note: …`
//! and `}` delimit blocks. The scanner is deliberately permissive, matching
//! the Go behaviour rather than enforcing the full dbml-lang spec.

use std::io::{BufRead, BufReader, Read};

use crate::model::*;

/// Parses a whole DBML document from a reader.
pub fn parse<R: Read>(reader: R) -> Result<Project, String> {
    let mut project = Project::default();
    let mut current_table: Option<Table> = None;
    let mut current_enum: Option<Enum> = None;
    let (mut in_table, mut in_enum) = (false, false);

    for line in BufReader::new(reader).lines() {
        let line = line.map_err(|err| format!("read error: {err}"))?;
        let line = line.trim();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }

        if let Some(name) = block_header(line, "Project") {
            project.name = name;
            continue;
        }
        if let Some(name) = block_header(line, "Table") {
            if in_table && let Some(t) = current_table.take() {
                project.tables.push(t);
            }
            current_table = Some(Table {
                name,
                ..Table::default()
            });
            in_table = true;
            in_enum = false;
            continue;
        }
        if let Some(name) = block_header(line, "Enum") {
            if in_enum && let Some(e) = current_enum.take() {
                project.enums.push(e);
            }
            current_enum = Some(Enum {
                name,
                ..Enum::default()
            });
            in_enum = true;
            in_table = false;
            continue;
        }
        if line == "}" {
            match (in_table, in_enum) {
                (true, _) => {
                    if let Some(t) = current_table.take() {
                        project.tables.push(t);
                    }
                    in_table = false;
                }
                (false, true) => {
                    if let Some(e) = current_enum.take() {
                        project.enums.push(e);
                    }
                    in_enum = false;
                }
                _ => {}
            }
            continue;
        }

        if in_table && let Some(table) = current_table.as_mut() {
            parse_table_line(line, table);
            continue;
        }
        if in_enum && let Some(enumeration) = current_enum.as_mut() {
            parse_enum_line(line, enumeration);
            continue;
        }
        if let Some(reference) = parse_reference(line) {
            project.references.push(reference);
            continue;
        }
        if let Some((key, value)) = note_parts(line) {
            project.comments.insert(key, value);
        }
    }

    Ok(project)
}

/// Parses a DBML file from disk.
pub fn parse_file(path: &str) -> Result<Project, String> {
    let file = std::fs::File::open(path).map_err(|err| format!("open {path}: {err}"))?;
    parse(file)
}

/// Extracts `NAME` from a `Name ident {` block header, or `None`.
fn block_header(line: &str, keyword: &str) -> Option<String> {
    let rest = line.strip_prefix(keyword)?.trim_start();
    let name: String = rest
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    (!name.is_empty() && rest[name.len()..].trim_start().starts_with('{')).then_some(name)
}

/// Splits `Note: 'text'` into (key, value); the Go port always uses an
/// empty key for top-level notes.
fn note_parts(line: &str) -> Option<(String, String)> {
    let rest = line.strip_prefix("Note:")?;
    let value = rest
        .trim()
        .trim_matches(|c| c == '\'' || c == '"')
        .to_string();
    Some((String::new(), value))
}

/// Handles one line inside a `Table { … }` block.
fn parse_table_line(line: &str, table: &mut Table) {
    if line.starts_with("Note:") {
        if let Some((_, value)) = note_parts(line) {
            table.comment = value;
        }
        return;
    }
    if line.starts_with("indexes") {
        return;
    }
    if line.starts_with("Index") {
        // Any "Index…" line is index syntax; malformed ones are dropped,
        // never treated as columns (matches the Go port).
        if let Some(index) = parse_index(line) {
            table.indexes.push(index);
        }
        return;
    }
    let column = parse_column(line);
    if !column.name.is_empty() {
        table.columns.push(column);
    }
}

/// Handles one line inside an `Enum { … }` block.
fn parse_enum_line(line: &str, enumeration: &mut Enum) {
    if line.is_empty() || line.starts_with("//") || line.starts_with("Note:") {
        return;
    }
    let value = line.trim_matches(|c| c == '\'' || c == '"');
    if !value.is_empty() {
        enumeration.values.push(value.to_string());
    }
}

/// Parses a top-level `Ref: name t1.c1 < t2.c2` line.
///
/// `<` means "many-to-one" (left references right); `>` reverses it, matching
/// the Go port's direction handling.
fn parse_reference(line: &str) -> Option<Reference> {
    let rest = line.strip_prefix("Ref:")?.trim();
    let mut tokens = rest.split_whitespace();
    let name = tokens.next()?.to_string();
    let left = tokens.next()?;
    let direction = tokens.next()?;
    let right = tokens.next()?;
    let (lt, rt) = (split_ref(left)?, split_ref(right)?);

    let (table1, column1, table2, column2) = if direction.contains('<') {
        (lt.0, lt.1, rt.0, rt.1)
    } else {
        (rt.0, rt.1, lt.0, lt.1)
    };
    Some(Reference {
        name,
        table1,
        column1,
        table2,
        column2,
    })
}

fn split_ref(token: &str) -> Option<(String, String)> {
    let (table, column) = token.split_once('.')?;
    if table.is_empty() || column.is_empty() {
        return None;
    }
    Some((
        table
            .trim_matches(|c| c == '>' || c == '<' || c == ' ')
            .to_string(),
        column
            .trim_matches(|c| c == '>' || c == '<' || c == ' ')
            .to_string(),
    ))
}

/// Parses one column line: `name type [settings, …]`.
fn parse_column(line: &str) -> Column {
    let mut settings_str = "";
    let mut head = line;
    if let (Some(open), Some(close)) = (line.find('['), line.rfind(']'))
        && close > open
    {
        settings_str = &line[open + 1..close];
        head = line[..open].trim_end();
    }

    let mut parts = head.split_whitespace();
    let (Some(name), Some(typ)) = (parts.next(), parts.next()) else {
        return Column::default();
    };
    let mut column = Column {
        name: name.to_string(),
        typ: typ.to_string(),
        ..Column::default()
    };

    if settings_str.is_empty() {
        return column;
    }
    for setting in settings_str.split(',') {
        let setting = setting.trim();
        let lower = setting.to_lowercase();
        if lower == "pk" || lower == "primary" {
            column.is_primary = true;
        } else if lower == "unique" {
            column.is_unique = true;
        } else if lower == "not_null" || lower == "notnull" {
            column.is_not_null = true;
        } else if lower == "null" {
            column.is_not_null = false;
        } else if lower.starts_with("default:") {
            column.default = setting["default:".len()..]
                .trim()
                .trim_matches(|c| c == '\'' || c == '"' || c == '`')
                .to_string();
        } else if lower.starts_with("ref:") {
            if let Some((table, col)) = setting["ref:".len()..].split_once('.') {
                column.ref_table = table
                    .trim()
                    .trim_matches(|c| c == '>' || c == '<' || c == ' ')
                    .to_string();
                column.ref_column = col
                    .trim()
                    .trim_matches(|c| c == '>' || c == '<' || c == ' ')
                    .to_string();
            }
        } else if lower.starts_with("note:") {
            column.comment = setting["note:".len()..]
                .trim()
                .trim_matches(|c| c == '\'' || c == '"')
                .to_string();
        } else if !setting.is_empty() {
            column.settings.push(setting.to_string());
        }
    }
    column
}

/// Parses an `Index … (cols)` line inside a table block.
fn parse_index(line: &str) -> Option<Index> {
    let mut rest = line.strip_prefix("Index")?.trim_start();

    let mut unique = false;
    if let Some(r) = rest.strip_prefix("unique") {
        unique = true;
        rest = r.trim_start();
    }

    let mut kind = String::new();
    if rest.starts_with("hash") || rest.starts_with("btree") {
        let first = rest.split_whitespace().next().unwrap_or("");
        kind = first.to_string();
        rest = rest[first.len()..].trim_start();
    }

    let (name, cols_part) = {
        let idx = rest.find('(')?;
        (rest[..idx].trim(), &rest[idx..])
    };
    let cols_part = cols_part.trim_matches(|c| c == '(' || c == ')');
    let columns: Vec<String> = cols_part
        .split(',')
        .map(str::trim)
        .filter(|c| !c.is_empty())
        .map(str::to_string)
        .collect();
    if columns.is_empty() {
        return None;
    }
    Some(Index {
        name: name.to_string(),
        columns,
        unique,
        kind,
    })
}
