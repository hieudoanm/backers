//! `backers db` — DBML conversion subcommands.
//!
//! Ports the Go `dbml` CLI: `sql`, `mermaid`, `svg`, `all` and `parse` turn a
//! `.dbml` file (or stdin via `-`) into PostgreSQL DDL, a Mermaid ER diagram,
//! an SVG diagram or a JSON schema summary.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use backers_dbml::Project;

/// The conversion formats `backers db` understands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Format {
    Sql,
    Mermaid,
    Svg,
}

impl Format {
    fn render(self, project: &Project) -> String {
        match self {
            Format::Sql => project.to_sql(),
            Format::Mermaid => project.to_mermaid(),
            Format::Svg => project.to_svg(),
        }
    }

    fn default_name(self) -> &'static str {
        match self {
            Format::Sql => "initial_schema.sql",
            Format::Mermaid => "database-erd.mmd",
            Format::Svg => "schema.svg",
        }
    }
}

/// Parsed `backers db <command> [file] [flags]` invocation.
pub struct DbCommand {
    pub command: String,
    input: Option<String>,
    output: Option<String>,
    output_dir: Option<String>,
}

/// Reports whether `args` selects `backers db …` and parses it.
///
/// Returns `None` when the first argument is not `db`, and `Some(Err)` style
/// usage failures are reported by the caller via [`usage_error`].
pub fn db_args(args: &[String]) -> Option<Result<DbCommand, String>> {
    if args.first().map(String::as_str) != Some("db") {
        return None;
    }
    let Some(command) = args.get(1).cloned() else {
        return Some(Err(
            "missing db command (sql|mermaid|svg|all|parse)".to_string()
        ));
    };
    if !matches!(
        command.as_str(),
        "sql" | "mermaid" | "svg" | "all" | "parse"
    ) {
        return Some(Err(format!("unknown db command: {command}")));
    }
    let mut cmd = DbCommand {
        command,
        input: None,
        output: None,
        output_dir: None,
    };
    let mut i = 2;
    while i < args.len() {
        let take = |i: &mut usize| -> Option<String> {
            let value = args.get(*i + 1)?.clone();
            *i += 2;
            Some(value)
        };
        match args[i].as_str() {
            "-o" | "--output" => cmd.output = Some(take(&mut i)?),
            "-d" | "--output-dir" => cmd.output_dir = Some(take(&mut i)?),
            // "-" is the stdin sentinel, not a flag.
            positional if positional == "-" || !positional.starts_with('-') => {
                if cmd.input.is_some() {
                    return Some(Err("too many arguments".to_string()));
                }
                cmd.input = Some(positional.to_string());
                i += 1;
            }
            flag if flag.starts_with('-') => {
                return Some(Err(format!("unknown flag: {flag}")));
            }
            _ => unreachable!("every argument is a flag or a positional"),
        }
    }
    Some(Ok(cmd))
}

/// Loads the DBML source: stdin for `-`, otherwise the named file
/// (default `schema.dbml`, matching the Go CLI).
fn load_source(input: Option<&str>) -> Result<String, String> {
    match input.unwrap_or("schema.dbml") {
        "-" => {
            let mut buffer = String::new();
            std::io::stdin()
                .read_to_string(&mut buffer)
                .map_err(|err| format!("read stdin: {err}"))?;
            Ok(buffer)
        }
        path => std::fs::read_to_string(path).map_err(|err| format!("read {path}: {err}")),
    }
}

fn parse_source(source: &str) -> Result<Project, String> {
    backers_dbml::parse(source.as_bytes())
}

fn write_output(path: &Path, content: &str) -> Result<(), String> {
    std::fs::write(path, content).map_err(|err| format!("write {}: {err}", path.display()))
}

/// Runs the parsed command, printing progress to stdout.
pub fn run(cmd: &DbCommand) -> Result<(), String> {
    let source = load_source(cmd.input.as_deref())?;
    let project = parse_source(&source)?;
    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    match cmd.command.as_str() {
        "parse" => {
            let summary = backers_dbml::summary::summarize(&project)
                .map_err(|err| format!("summarize: {err}"))?;
            writeln!(out, "{summary}").map_err(|err| format!("write stdout: {err}"))?;
        }
        "all" => {
            let dir = PathBuf::from(cmd.output_dir.as_deref().unwrap_or("."));
            std::fs::create_dir_all(&dir)
                .map_err(|err| format!("create {}: {err}", dir.display()))?;
            for format in [Format::Sql, Format::Mermaid, Format::Svg] {
                let path = dir.join(format.default_name());
                write_output(&path, &format.render(&project))?;
                writeln!(out, "Generated {}: {}", format_name(format), path.display())
                    .map_err(|err| format!("write stdout: {err}"))?;
            }
        }
        name => {
            let format = match name {
                "sql" => Format::Sql,
                "mermaid" => Format::Mermaid,
                _ => Format::Svg,
            };
            let output = cmd
                .output
                .clone()
                .unwrap_or_else(|| format.default_name().to_string());
            write_output(Path::new(&output), &format.render(&project))?;
            writeln!(out, "Generated {}: {output}", format_name(format))
                .map_err(|err| format!("write stdout: {err}"))?;
        }
    }
    Ok(())
}

fn format_name(format: Format) -> &'static str {
    match format {
        Format::Sql => "SQL schema",
        Format::Mermaid => "Mermaid ER diagram",
        Format::Svg => "SVG diagram",
    }
}
