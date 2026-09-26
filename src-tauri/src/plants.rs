use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct PlantDefinition {
    pub id: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    pub min_focus_secs: u32,
    pub accent: &'static str,
}

pub const DEFAULT_PLANT_ID: &str = "clover";

const CATALOG: &[PlantDefinition] = &[
    PlantDefinition { id: "clover", name: "Clover", category: "groundcover", min_focus_secs: 60, accent: "#77b255" },
    PlantDefinition { id: "daisy", name: "Daisy", category: "flower", min_focus_secs: 10 * 60, accent: "#f5d76e" },
    PlantDefinition { id: "cactus", name: "Cactus", category: "succulent", min_focus_secs: 15 * 60, accent: "#4f9b68" },
    PlantDefinition { id: "lavender", name: "Lavender", category: "flower", min_focus_secs: 20 * 60, accent: "#967bb6" },
    PlantDefinition { id: "cherry", name: "Cherry Blossom", category: "tree", min_focus_secs: 25 * 60, accent: "#e99aaa" },
    PlantDefinition { id: "pine", name: "Pine", category: "tree", min_focus_secs: 30 * 60, accent: "#39715a" },
    PlantDefinition { id: "maple", name: "Maple", category: "tree", min_focus_secs: 45 * 60, accent: "#d46a3a" },
    PlantDefinition { id: "oak", name: "Ancient Oak", category: "tree", min_focus_secs: 60 * 60, accent: "#65844a" },
];

pub fn catalog() -> Vec<PlantDefinition> {
    CATALOG.to_vec()
}

pub fn find(id: &str) -> Option<&'static PlantDefinition> {
    CATALOG.iter().find(|plant| plant.id == id)
}

pub fn is_valid(id: &str) -> bool {
    find(id).is_some()
}

/// Completed focus duration determines the visual size of the planted tree.
pub fn growth_stage(plant_id: &str, duration_secs: u32) -> Option<&'static str> {
    let plant = find(plant_id)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn respects_minimum_focus_time() {
        assert_eq!(growth_stage("oak", 59 * 60), None);
        assert_eq!(growth_stage("oak", 60 * 60), Some("large"));
        assert_eq!(growth_stage("daisy", 10 * 60), Some("small"));
    }
}
