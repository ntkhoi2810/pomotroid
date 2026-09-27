use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use image::{ImageFormat, ImageReader};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PlantDefinition {
    pub id: String,
    pub name: String,
    pub category: String,
    pub min_focus_secs: u32,
    pub accent: String,
    pub is_builtin: bool,
    pub hidden: bool,
    pub small_icon_path: Option<String>,
    pub medium_icon_path: Option<String>,
    pub large_icon_path: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PlantInput {
    pub id: Option<String>,
    pub name: String,
    pub category: String,
    pub min_focus_secs: u32,
    pub accent: String,
    #[serde(alias = "small_icon_source")]
    pub small_icon_source_path: Option<String>,
    #[serde(alias = "medium_icon_source")]
    pub medium_icon_source_path: Option<String>,
    #[serde(alias = "large_icon_source")]
    pub large_icon_source_path: Option<String>,
}

struct BuiltinPlant {
    id: &'static str,
    name: &'static str,
    category: &'static str,
    min_focus_secs: u32,
    accent: &'static str,
}

pub const DEFAULT_PLANT_ID: &str = "clover";
const MAX_IMAGE_BYTES: u64 = 2 * 1024 * 1024;
const MAX_IMAGE_DIMENSION: u32 = 2048;
static UNIQUE_COUNTER: AtomicU64 = AtomicU64::new(0);

const CATALOG: &[BuiltinPlant] = &[
    BuiltinPlant {
        id: "clover",
        name: "Clover",
        category: "groundcover",
        min_focus_secs: 60,
        accent: "#77b255",
    },
    BuiltinPlant {
        id: "daisy",
        name: "Daisy",
        category: "flower",
        min_focus_secs: 10 * 60,
        accent: "#f5d76e",
    },
    BuiltinPlant {
        id: "cactus",
        name: "Cactus",
        category: "succulent",
        min_focus_secs: 15 * 60,
        accent: "#4f9b68",
    },
    BuiltinPlant {
        id: "lavender",
        name: "Lavender",
        category: "flower",
        min_focus_secs: 20 * 60,
        accent: "#967bb6",
    },
    BuiltinPlant {
        id: "cherry",
        name: "Cherry Blossom",
        category: "tree",
        min_focus_secs: 25 * 60,
        accent: "#e99aaa",
    },
    BuiltinPlant {
        id: "pine",
        name: "Pine",
        category: "tree",
        min_focus_secs: 30 * 60,
        accent: "#39715a",
    },
    BuiltinPlant {
        id: "maple",
        name: "Maple",
        category: "tree",
        min_focus_secs: 45 * 60,
        accent: "#d46a3a",
    },
    BuiltinPlant {
        id: "oak",
        name: "Ancient Oak",
        category: "tree",
        min_focus_secs: 60 * 60,
        accent: "#65844a",
    },
];

pub fn list(conn: &Connection, include_hidden: bool) -> rusqlite::Result<Vec<PlantDefinition>> {
    let mut plants = Vec::new();
    for builtin in CATALOG {
        let plant = find(conn, builtin.id)?.expect("built-in plant must resolve");
        if include_hidden || !plant.hidden {
            plants.push(plant);
        }
    }

    let mut stmt = conn.prepare(
        "SELECT id, name, category, min_focus_secs, accent, hidden,
                small_icon_path, medium_icon_path, large_icon_path
         FROM custom_plants
         WHERE ?1 OR hidden = 0
         ORDER BY name COLLATE NOCASE, id",
    )?;
    let custom = stmt.query_map([include_hidden], |row| row_to_plant(row, false))?;
    plants.extend(custom.collect::<rusqlite::Result<Vec<_>>>()?);
    Ok(plants)
}

pub fn find(conn: &Connection, id: &str) -> rusqlite::Result<Option<PlantDefinition>> {
    if let Some(builtin) = CATALOG.iter().find(|plant| plant.id == id) {
        let override_plant = conn
            .query_row(
                "SELECT id, name, category, min_focus_secs, accent, hidden,
                        small_icon_path, medium_icon_path, large_icon_path
                 FROM plant_overrides WHERE id = ?1",
                [id],
                |row| row_to_plant(row, true),
            )
            .optional()?;
        return Ok(Some(
            override_plant.unwrap_or_else(|| builtin_definition(builtin)),
        ));
    }

    conn.query_row(
        "SELECT id, name, category, min_focus_secs, accent, hidden,
                small_icon_path, medium_icon_path, large_icon_path
         FROM custom_plants WHERE id = ?1",
        [id],
        |row| row_to_plant(row, false),
    )
    .optional()
}

pub fn growth_stage(plant: &PlantDefinition, duration_secs: u32) -> Option<&'static str> {
    if duration_secs < plant.min_focus_secs {
        return None;
    }
    Some(if duration_secs < 25 * 60 {
        "small"
    } else if duration_secs < 45 * 60 {
        "medium"
    } else {
        "large"
    })
}

pub fn stage_icon_path(plant: &PlantDefinition, stage: &str) -> Option<String> {
    match stage {
        "small" => plant.small_icon_path.clone(),
        "medium" => plant.medium_icon_path.clone(),
        "large" => plant.large_icon_path.clone(),
        _ => None,
    }
}

pub fn save(
    conn: &Connection,
    input: PlantInput,
    app_data_dir: &Path,
) -> Result<PlantDefinition, String> {
    validate_fields(&input)?;
    let existing = input
        .id
        .as_deref()
        .map(|id| find(conn, id).map_err(|e| e.to_string()))
        .transpose()?
        .flatten();
    if input.id.is_some() && existing.is_none() {
        return Err("unknown plant".to_string());
    }

    let is_builtin = input
        .id
        .as_deref()
        .is_some_and(|id| CATALOG.iter().any(|plant| plant.id == id));
    let id = input.id.clone().unwrap_or_else(new_custom_id);
    let is_create = input.id.is_none();
    if is_create
        && (input.small_icon_source_path.is_none()
            || input.medium_icon_source_path.is_none()
            || input.large_icon_source_path.is_none())
    {
        return Err("custom plants require all three stage images".to_string());
    }

    let mut imported = Vec::new();
    let result = (|| {
        let small = import_or_retain(
            input.small_icon_source_path.as_deref(),
            existing
                .as_ref()
                .and_then(|plant| plant.small_icon_path.clone()),
            app_data_dir,
            &mut imported,
        )?;
        let medium = import_or_retain(
            input.medium_icon_source_path.as_deref(),
            existing
                .as_ref()
                .and_then(|plant| plant.medium_icon_path.clone()),
            app_data_dir,
            &mut imported,
        )?;
        let large = import_or_retain(
            input.large_icon_source_path.as_deref(),
            existing
                .as_ref()
                .and_then(|plant| plant.large_icon_path.clone()),
            app_data_dir,
            &mut imported,
        )?;
        if !is_builtin && (small.is_none() || medium.is_none() || large.is_none()) {
            return Err("custom plants require all three stage images".to_string());
        }

        let hidden = existing.as_ref().is_some_and(|plant| plant.hidden);
        let table = if is_builtin {
            "plant_overrides"
        } else {
            "custom_plants"
        };
        conn.execute(
            &format!(
                "INSERT INTO {table}
                    (id, name, category, min_focus_secs, accent, hidden,
                     small_icon_path, medium_icon_path, large_icon_path)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                 ON CONFLICT(id) DO UPDATE SET
                    name=excluded.name, category=excluded.category,
                    min_focus_secs=excluded.min_focus_secs, accent=excluded.accent,
                    hidden=excluded.hidden, small_icon_path=excluded.small_icon_path,
                    medium_icon_path=excluded.medium_icon_path,
                    large_icon_path=excluded.large_icon_path"
            ),
            params![
                id,
                input.name.trim(),
                input.category.trim(),
                input.min_focus_secs,
                input.accent.trim(),
                hidden,
                small,
                medium,
                large,
            ],
        )
        .map_err(|e| e.to_string())?;
        find(conn, &id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "saved plant could not be loaded".to_string())
    })();

    if result.is_err() {
        for path in imported {
            let _ = fs::remove_file(path);
        }
    }
    result
}

pub fn set_hidden(conn: &Connection, id: &str, hidden: bool) -> Result<(), String> {
    let table = if CATALOG.iter().any(|plant| plant.id == id) {
        if conn
            .query_row("SELECT 1 FROM plant_overrides WHERE id = ?1", [id], |_| {
                Ok(())
            })
            .optional()
            .map_err(|e| e.to_string())?
            .is_none()
        {
            let builtin = CATALOG.iter().find(|plant| plant.id == id).unwrap();
            conn.execute(
                "INSERT INTO plant_overrides
                    (id, name, category, min_focus_secs, accent, hidden)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    builtin.id,
                    builtin.name,
                    builtin.category,
                    builtin.min_focus_secs,
                    builtin.accent,
                    hidden
                ],
            )
            .map_err(|e| e.to_string())?;
            return Ok(());
        }
        "plant_overrides"
    } else {
        "custom_plants"
    };
    let changed = conn
        .execute(
            &format!("UPDATE {table} SET hidden = ?1 WHERE id = ?2"),
            params![hidden, id],
        )
        .map_err(|e| e.to_string())?;
    if changed == 0 {
        return Err(format!("unknown plant: '{id}'"));
    }
    Ok(())
}

pub fn restore_default(conn: &Connection, id: &str) -> Result<Vec<String>, String> {
    if !CATALOG.iter().any(|plant| plant.id == id) {
        return Err(format!("plant '{id}' is not built in"));
    }
    let old_paths = find(conn, id)
        .map_err(|e| e.to_string())?
        .into_iter()
        .flat_map(|plant| {
            [
                plant.small_icon_path,
                plant.medium_icon_path,
                plant.large_icon_path,
            ]
        })
        .flatten()
        .collect();
    conn.execute("DELETE FROM plant_overrides WHERE id = ?1", [id])
        .map_err(|e| e.to_string())?;
    Ok(old_paths)
}

pub fn delete_custom(conn: &Connection, id: &str) -> Result<Vec<String>, String> {
    if CATALOG.iter().any(|plant| plant.id == id) {
        return Err("built-in plants can only be hidden or restored".to_string());
    }
    let plant = find(conn, id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("unknown plant: '{id}'"))?;
    let paths = [
        plant.small_icon_path,
        plant.medium_icon_path,
        plant.large_icon_path,
    ]
    .into_iter()
    .flatten()
    .collect();
    conn.execute("DELETE FROM custom_plants WHERE id = ?1", [id])
        .map_err(|e| e.to_string())?;
    Ok(paths)
}

pub fn remove_unreferenced_files(conn: &Connection, paths: impl IntoIterator<Item = String>) {
    for path in paths {
        let referenced: bool = conn
            .query_row(
                "SELECT EXISTS(
                    SELECT 1 FROM plant_overrides
                     WHERE small_icon_path=?1 OR medium_icon_path=?1 OR large_icon_path=?1
                    UNION ALL
                    SELECT 1 FROM custom_plants
                     WHERE small_icon_path=?1 OR medium_icon_path=?1 OR large_icon_path=?1
                    UNION ALL
                    SELECT 1 FROM sessions WHERE plant_icon_path=?1
                 )",
                [&path],
                |row| row.get(0),
            )
            .unwrap_or(true);
        if !referenced {
            let _ = fs::remove_file(path);
        }
    }
}

fn builtin_definition(plant: &BuiltinPlant) -> PlantDefinition {
    PlantDefinition {
        id: plant.id.to_string(),
        name: plant.name.to_string(),
        category: plant.category.to_string(),
        min_focus_secs: plant.min_focus_secs,
        accent: plant.accent.to_string(),
        is_builtin: true,
        hidden: false,
        small_icon_path: None,
        medium_icon_path: None,
        large_icon_path: None,
    }
}

fn row_to_plant(row: &rusqlite::Row<'_>, is_builtin: bool) -> rusqlite::Result<PlantDefinition> {
    Ok(PlantDefinition {
        id: row.get(0)?,
        name: row.get(1)?,
        category: row.get(2)?,
        min_focus_secs: row.get(3)?,
        accent: row.get(4)?,
        is_builtin,
        hidden: row.get(5)?,
        small_icon_path: row.get(6)?,
        medium_icon_path: row.get(7)?,
        large_icon_path: row.get(8)?,
    })
}

fn validate_fields(input: &PlantInput) -> Result<(), String> {
    if input.name.trim().is_empty() || input.category.trim().is_empty() {
        return Err("plant name and category are required".to_string());
    }
    if input.min_focus_secs == 0 {
        return Err("minimum focus duration must be greater than zero".to_string());
    }
    if input.min_focus_secs > 90 * 60 {
        return Err("minimum focus duration may not exceed 90 minutes".to_string());
    }
    let accent = input.accent.trim().as_bytes();
    if accent.len() != 7 || accent[0] != b'#' || !accent[1..].iter().all(u8::is_ascii_hexdigit) {
        return Err("plant accent must be a #RRGGBB color".to_string());
    }
    Ok(())
}

fn import_or_retain(
    source: Option<&str>,
    existing: Option<String>,
    app_data_dir: &Path,
    imported: &mut Vec<PathBuf>,
) -> Result<Option<String>, String> {
    let Some(source) = source else {
        return Ok(existing);
    };
    let source = Path::new(source);
    let metadata = fs::metadata(source).map_err(|e| format!("failed to read image: {e}"))?;
    if !metadata.is_file() || metadata.len() > MAX_IMAGE_BYTES {
        return Err("plant images must be files no larger than 2 MB".to_string());
    }

    let reader = ImageReader::open(source)
        .map_err(|e| format!("failed to open image: {e}"))?
        .with_guessed_format()
        .map_err(|e| format!("failed to identify image: {e}"))?;
    let format = reader
        .format()
        .ok_or_else(|| "unknown image format".to_string())?;
    if !matches!(format, ImageFormat::Png | ImageFormat::WebP) {
        return Err("plant images must be PNG or WebP".to_string());
    }
    let image = reader
        .decode()
        .map_err(|e| format!("failed to decode image: {e}"))?;
    if image.width() > MAX_IMAGE_DIMENSION || image.height() > MAX_IMAGE_DIMENSION {
        return Err("plant images may not exceed 2048x2048".to_string());
    }

    let dir = app_data_dir.join("plants");
    fs::create_dir_all(&dir).map_err(|e| format!("failed to create plant image directory: {e}"))?;
    let extension = if format == ImageFormat::Png {
        "png"
    } else {
        "webp"
    };
    let destination = dir.join(format!("plant-{}.{}", unique_value(), extension));
    fs::copy(source, &destination).map_err(|e| format!("failed to import image: {e}"))?;
    imported.push(destination.clone());
    Ok(Some(destination.to_string_lossy().into_owned()))
}

fn unique_value() -> u128 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    nanos + UNIQUE_COUNTER.fetch_add(1, Ordering::Relaxed) as u128
}

fn new_custom_id() -> String {
    format!("custom-{}", unique_value())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrations;

    fn setup() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        migrations::run(&conn).unwrap();
        conn
    }

    #[test]
    fn builtin_override_can_be_hidden_and_restored() {
        let conn = setup();
        set_hidden(&conn, "clover", true).unwrap();
        assert!(find(&conn, "clover").unwrap().unwrap().hidden);
        restore_default(&conn, "clover").unwrap();
        assert!(!find(&conn, "clover").unwrap().unwrap().hidden);
    }

    #[test]
    fn growth_uses_resolved_minimum() {
        let conn = setup();
        conn.execute(
            "INSERT INTO plant_overrides
                (id, name, category, min_focus_secs, accent, hidden)
             VALUES ('clover', 'Clover', 'groundcover', 600, '#77b255', 0)",
            [],
        )
        .unwrap();
        let clover = find(&conn, "clover").unwrap().unwrap();
        assert_eq!(growth_stage(&clover, 599), None);
        assert_eq!(growth_stage(&clover, 600), Some("small"));
    }
}
