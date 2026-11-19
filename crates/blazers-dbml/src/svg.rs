//! SVG ER-diagram renderer — a port of the Go `svg.go`/`svg_draw.go` pair.
//!
//! Layout: items flow left-to-right in rows of [`SVG_COLS`]; tables are 230px
//! wide, enums 170px. References are cubic-bezier links between column slots.

use crate::generate::escape_xml;
use crate::model::{Project, Reference};
use std::fmt::Write as _;

const SVG_MARGIN: i32 = 48;
const SVG_GAP_X: i32 = 90;
const SVG_GAP_Y: i32 = 70;
const SVG_COLS: usize = 3;
const SVG_TABLE_W: i32 = 230;
const SVG_ENUM_W: i32 = 170;
const SVG_HEADER_H: i32 = 34;
const SVG_ROW_H: i32 = 27;

#[derive(Default)]
struct ItemRow {
    name: String,
    typ: String,
    key: String,
}

#[derive(Default)]
struct Item {
    title: String,
    rows: Vec<ItemRow>,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

impl Project {
    /// Renders the whole schema as a standalone SVG document.
    pub fn to_svg(&self) -> String {
        let mut items = self.svg_items();
        place_items(&mut items);
        let (width, height) = canvas_size(&items);

        let mut out = String::new();
        write_svg_head(&mut out, width, height);

        let boxes: std::collections::HashMap<&str, &Item> = items
            .iter()
            .map(|item| (item.title.as_str(), item))
            .collect();
        for reference in &self.references {
            draw_reference(&mut out, &boxes, reference);
        }
        for item in &items {
            draw_item(&mut out, item);
        }
        out.push_str("</svg>\n");
        out
    }

    fn svg_items(&self) -> Vec<Item> {
        let mut items = Vec::with_capacity(self.tables.len() + self.enums.len());
        for table in &self.tables {
            let rows = table
                .columns
                .iter()
                .map(|col| {
                    let key = if col.is_primary {
                        "PK"
                    } else if col.is_unique {
                        "UK"
                    } else {
                        ""
                    };
                    let mut typ = col.sql_type();
                    if col.is_not_null {
                        typ.push_str(" NOT NULL");
                    }
                    ItemRow {
                        name: col.name.clone(),
                        typ,
                        key: key.to_string(),
                    }
                })
                .collect();
            items.push(Item {
                title: table.name.clone(),
                rows,
                w: SVG_TABLE_W,
                ..Item::default()
            });
        }
        for enumeration in &self.enums {
            let rows = enumeration
                .values
                .iter()
                .map(|value| ItemRow {
                    name: value.clone(),
                    ..ItemRow::default()
                })
                .collect();
            items.push(Item {
                title: format!("enum {}", enumeration.name),
                rows,
                w: SVG_ENUM_W,
                ..Item::default()
            });
        }
        items
    }
}

fn place_items(items: &mut [Item]) {
    let (mut x, mut y, mut row_max) = (SVG_MARGIN, SVG_MARGIN, 0);
    for (i, item) in items.iter_mut().enumerate() {
        item.h = SVG_HEADER_H + item.rows.len() as i32 * SVG_ROW_H;
        if i > 0 && i % SVG_COLS == 0 {
            x = SVG_MARGIN;
            y += row_max + SVG_GAP_Y;
            row_max = 0;
        }
        item.x = x;
        item.y = y;
        x += item.w + SVG_GAP_X;
        row_max = row_max.max(item.h);
    }
}

fn canvas_size(items: &[Item]) -> (i32, i32) {
    let mut width = 0;
    let mut height = 0;
    for item in items {
        width = width.max(item.x + item.w + SVG_MARGIN);
        height = height.max(item.y + item.h + SVG_MARGIN);
    }
    (width, height)
}

fn write_svg_head(out: &mut String, width: i32, height: i32) {
    let _ = writeln!(
        out,
        r##"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}" font-family="ui-monospace, SFMono-Regular, Menlo, monospace">"##
    );
    out.push_str(
        r##"<defs>
<marker id="arrow" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto"><path d="M0 0 L10 5 L0 10 z" fill="#7c3aed"/></marker>
<marker id="dot" viewBox="0 0 10 10" refX="5" refY="5" markerWidth="6" markerHeight="6"><circle cx="5" cy="5" r="3.2" fill="#6366f1"/></marker>
</defs>
"##,
    );
    let _ = writeln!(
        out,
        r##"<rect width="{width}" height="{height}" fill="#ffffff"/>"##
    );
}

fn draw_item(out: &mut String, item: &Item) {
    let _ = writeln!(
        out,
        r##"<rect x="{}" y="{}" width="{}" height="{}" fill="#0f172a" rx="6"/>"##,
        item.x, item.y, item.w, SVG_HEADER_H
    );
    let _ = writeln!(
        out,
        r##"<text x="{}" y="{}" font-size="13" font-weight="bold" fill="#ffffff" text-anchor="middle">{}</text>"##,
        item.x + item.w / 2,
        item.y + 21,
        escape_xml(&item.title)
    );

    let mut row_y = item.y + SVG_HEADER_H;
    for (i, row) in item.rows.iter().enumerate() {
        let mut fill = "#ffffff";
        if i % 2 == 1 {
            fill = "#f8fafc";
        }
        if row.key == "PK" {
            fill = "#f5f3ff";
        }
        let _ = writeln!(
            out,
            r##"<rect x="{}" y="{}" width="{}" height="{}" fill="{}"/>"##,
            item.x, row_y, item.w, SVG_ROW_H, fill
        );

        let mut name_x = item.x + 8;
        if !row.key.is_empty() {
            let _ = writeln!(
                out,
                r##"<text x="{}" y="{}" font-size="9" font-weight="bold" fill="#7c3aed">{}</text>"##,
                name_x,
                row_y + 17,
                row.key
            );
            name_x += 26;
        }
        let _ = writeln!(
            out,
            r##"<text x="{}" y="{}" font-size="11" fill="#0f172a">{}</text>"##,
            name_x,
            row_y + 17,
            escape_xml(&row.name)
        );

        if !row.typ.is_empty() {
            let _ = writeln!(
                out,
                r##"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="#e2e8f0"/>"##,
                item.x + item.w / 2,
                row_y,
                item.x + item.w / 2,
                row_y + SVG_ROW_H
            );
            let _ = writeln!(
                out,
                r##"<text x="{}" y="{}" font-size="9" fill="#64748b" text-anchor="end">{}</text>"##,
                item.x + item.w - 8,
                row_y + 17,
                escape_xml(&row.typ)
            );
        }
        row_y += SVG_ROW_H;
    }

    let _ = writeln!(
        out,
        r##"<rect x="{}" y="{}" width="{}" height="{}" fill="none" stroke="#e2e8f0" rx="6"/>"##,
        item.x, item.y, item.w, item.h
    );
}

fn draw_reference(
    out: &mut String,
    boxes: &std::collections::HashMap<&str, &Item>,
    reference: &Reference,
) {
    let (Some(src), Some(dst)) = (
        boxes.get(reference.table2.as_str()),
        boxes.get(reference.table1.as_str()),
    ) else {
        return;
    };
    let sy = slot_y(src, &reference.column2);
    let ty = slot_y(dst, &reference.column1);

    if dst.x > src.x + src.w {
        draw_link(out, src.x + src.w, sy, dst.x, ty);
    } else if dst.x + dst.w < src.x {
        draw_link(out, src.x, sy, dst.x + dst.w, ty);
    } else {
        draw_link(
            out,
            src.x + src.w / 2,
            src.y + src.h,
            dst.x + dst.w / 2,
            dst.y,
        );
    }
}

fn slot_y(item: &Item, column: &str) -> i32 {
    for (i, row) in item.rows.iter().enumerate() {
        if row.name == column {
            return item.y + SVG_HEADER_H + i as i32 * SVG_ROW_H + SVG_ROW_H / 2;
        }
    }
    item.y + SVG_HEADER_H / 2
}

fn draw_link(out: &mut String, ex: i32, ey: i32, px: i32, py: i32) {
    let (dx, dy) = (px - ex, py - ey);
    if dx == 0 && dy == 0 {
        return;
    }
    let (mut c1x, mut c1y, mut c2x, mut c2y) = (ex, ey, px, py);
    if dx.abs() >= dy.abs() {
        let k = (dx.abs() / 2).clamp(24, 90) * dx.signum();
        c1x = ex + k;
        c2x = px - k;
    } else {
        let k = (dy.abs() / 2).clamp(24, 90) * dy.signum();
        c1y = ey + k;
        c2y = py - k;
    }
    let _ = writeln!(
        out,
        r##"<path d="M{ex} {ey} C{c1x} {c1y} {c2x} {c2y} {px} {py}" fill="none" stroke="#7c3aed" stroke-width="1.6" marker-start="url(#dot)" marker-end="url(#arrow)"/>"##
    );
}
