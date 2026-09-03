use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Entity {
    pub name: String,
    #[serde(rename = "entityType", alias = "entity_type")]
    pub entity_type: String,
    #[serde(default)]
    pub observations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Relation {
    pub from: String,
    pub to: String,
    #[serde(rename = "relationType", alias = "relation_type")]
    pub relation_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct KnowledgeGraph {
    #[serde(default)]
    pub entities: Vec<Entity>,
    #[serde(default)]
    pub relations: Vec<Relation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddObservation {
    #[serde(rename = "entityName", alias = "entity_name")]
    pub entity_name: String,
    pub contents: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteObservation {
    #[serde(rename = "entityName", alias = "entity_name")]
    pub entity_name: String,
    pub observations: Vec<String>,
}
