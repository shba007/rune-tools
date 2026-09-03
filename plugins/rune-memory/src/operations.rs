// plugins/rune-memory/src/operations.rs
use crate::types::{AddObservation, DeleteObservation, Entity, KnowledgeGraph, Relation};
use rune_pdk::ToolCallRequest;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(target_arch = "wasm32")]
fn get_config(key: &str) -> Option<String> {
    extism_pdk::config::get(key).ok().flatten()
}

#[cfg(not(target_arch = "wasm32"))]
fn get_config(key: &str) -> Option<String> {
    let upper = key.to_ascii_uppercase();
    let lower = key.to_ascii_lowercase();
    std::env::var(&upper)
        .or_else(|_| std::env::var(&lower))
        .or_else(|_| std::env::var(key))
        .ok()
}

pub fn resolve_storage_path(args: Option<&Value>) -> PathBuf {
    let explicit_file = args
        .and_then(|a| a.get("memoryFile").or_else(|| a.get("memory_file")))
        .and_then(Value::as_str)
        .map(ToString::to_string);

    let filename = explicit_file
        .or_else(|| get_config("memory_file"))
        .unwrap_or_else(|| "memory.json".to_string());

    let explicit_dir = args
        .and_then(|a| a.get("allowedDir").or_else(|| a.get("allowed_dir")))
        .and_then(Value::as_str)
        .map(ToString::to_string);

    if let Some(allowed_root) = explicit_dir.or_else(|| get_config("allowed_dir")) {
        PathBuf::from(allowed_root).join(filename)
    } else {
        PathBuf::from(filename)
    }
}

pub fn load_graph_from_path(path: &Path) -> Result<KnowledgeGraph, String> {
    if !path.exists() {
        return Ok(KnowledgeGraph::default());
    }
    let data = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read memory file '{}': {}", path.display(), e))?;
    if data.trim().is_empty() {
        return Ok(KnowledgeGraph::default());
    }
    serde_json::from_str(&data).map_err(|e| {
        format!(
            "Corrupt knowledge graph JSON in '{}': {}",
            path.display(),
            e
        )
    })
}

pub fn save_graph_to_path(path: &Path, graph: &KnowledgeGraph) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            let _ = fs::create_dir_all(parent);
        }
    }
    let data = serde_json::to_string_pretty(graph)
        .map_err(|e| format!("Failed to serialize knowledge graph: {}", e))?;
    fs::write(path, data).map_err(|e| {
        format!(
            "Failed to write knowledge graph to '{}': {}",
            path.display(),
            e
        )
    })?;
    Ok(())
}

pub fn execute_tool(request: ToolCallRequest) -> Result<Value, String> {
    let storage_path = resolve_storage_path(Some(&request.arguments));
    let mut graph = load_graph_from_path(&storage_path)?;

    match request.name.as_str() {
        "create_entities" => {
            let entities_val = request
                .arguments
                .get("entities")
                .ok_or_else(|| "Missing 'entities' parameter".to_string())?;
            let new_entities: Vec<Entity> = serde_json::from_value(entities_val.clone())
                .map_err(|e| format!("Invalid entities format: {}", e))?;

            let mut created = Vec::new();
            for entity in new_entities {
                if !graph
                    .entities
                    .iter()
                    .any(|e| e.name.eq_ignore_ascii_case(&entity.name))
                {
                    graph.entities.push(entity.clone());
                    created.push(entity);
                }
            }

            save_graph_to_path(&storage_path, &graph)?;
            Ok(json!({ "created": created }))
        }

        "create_relations" => {
            let relations_val = request
                .arguments
                .get("relations")
                .ok_or_else(|| "Missing 'relations' parameter".to_string())?;
            let new_relations: Vec<Relation> = serde_json::from_value(relations_val.clone())
                .map_err(|e| format!("Invalid relations format: {}", e))?;

            let mut created = Vec::new();
            for rel in new_relations {
                let exists = graph.relations.iter().any(|r| {
                    r.from.eq_ignore_ascii_case(&rel.from)
                        && r.to.eq_ignore_ascii_case(&rel.to)
                        && r.relation_type.eq_ignore_ascii_case(&rel.relation_type)
                });

                if !exists {
                    graph.relations.push(rel.clone());
                    created.push(rel);
                }
            }

            save_graph_to_path(&storage_path, &graph)?;
            Ok(json!({ "created": created }))
        }

        "add_observations" => {
            let obs_val = request
                .arguments
                .get("observations")
                .ok_or_else(|| "Missing 'observations' parameter".to_string())?;
            let additions: Vec<AddObservation> = serde_json::from_value(obs_val.clone())
                .map_err(|e| format!("Invalid observations format: {}", e))?;

            let mut updated = Vec::new();
            for item in additions {
                if let Some(entity) = graph
                    .entities
                    .iter_mut()
                    .find(|e| e.name.eq_ignore_ascii_case(&item.entity_name))
                {
                    for content in item.contents {
                        if !entity.observations.iter().any(|o| o == &content) {
                            entity.observations.push(content.clone());
                        }
                    }
                    updated.push(entity.clone());
                } else {
                    return Err(format!("Entity '{}' not found", item.entity_name));
                }
            }

            save_graph_to_path(&storage_path, &graph)?;
            Ok(json!({ "updated": updated }))
        }

        "delete_entities" => {
            let names_val = request
                .arguments
                .get("entityNames")
                .or_else(|| request.arguments.get("entity_names"))
                .ok_or_else(|| "Missing 'entityNames' parameter".to_string())?;
            let names: Vec<String> = serde_json::from_value(names_val.clone())
                .map_err(|e| format!("Invalid entityNames format: {}", e))?;

            graph
                .entities
                .retain(|e| !names.iter().any(|n| n.eq_ignore_ascii_case(&e.name)));
            graph.relations.retain(|r| {
                !names
                    .iter()
                    .any(|n| n.eq_ignore_ascii_case(&r.from) || n.eq_ignore_ascii_case(&r.to))
            });

            save_graph_to_path(&storage_path, &graph)?;
            Ok(json!({ "deleted": names }))
        }

        "delete_observations" => {
            let del_val = request
                .arguments
                .get("deletions")
                .ok_or_else(|| "Missing 'deletions' parameter".to_string())?;
            let deletions: Vec<DeleteObservation> = serde_json::from_value(del_val.clone())
                .map_err(|e| format!("Invalid deletions format: {}", e))?;

            for item in deletions {
                if let Some(entity) = graph
                    .entities
                    .iter_mut()
                    .find(|e| e.name.eq_ignore_ascii_case(&item.entity_name))
                {
                    entity
                        .observations
                        .retain(|obs| !item.observations.contains(obs));
                }
            }

            save_graph_to_path(&storage_path, &graph)?;
            Ok(json!({ "status": "success" }))
        }

        "delete_relations" => {
            let rel_val = request
                .arguments
                .get("relations")
                .ok_or_else(|| "Missing 'relations' parameter".to_string())?;
            let deletions: Vec<Relation> = serde_json::from_value(rel_val.clone())
                .map_err(|e| format!("Invalid relations format: {}", e))?;

            graph.relations.retain(|r| {
                !deletions.iter().any(|del| {
                    del.from.eq_ignore_ascii_case(&r.from)
                        && del.to.eq_ignore_ascii_case(&r.to)
                        && del.relation_type.eq_ignore_ascii_case(&del.relation_type)
                })
            });

            save_graph_to_path(&storage_path, &graph)?;
            Ok(json!({ "status": "success" }))
        }

        "read_graph" => Ok(json!({
            "entities": graph.entities,
            "relations": graph.relations
        })),

        "search_nodes" => {
            let query = request
                .arguments
                .get("query")
                .and_then(Value::as_str)
                .ok_or_else(|| "Missing 'query' parameter".to_string())?
                .to_lowercase();

            let matched_entities: Vec<Entity> = graph
                .entities
                .into_iter()
                .filter(|e| {
                    e.name.to_lowercase().contains(&query)
                        || e.entity_type.to_lowercase().contains(&query)
                        || e.observations
                            .iter()
                            .any(|obs| obs.to_lowercase().contains(&query))
                })
                .collect();

            let matched_names: Vec<String> =
                matched_entities.iter().map(|e| e.name.clone()).collect();

            let matched_relations: Vec<Relation> = graph
                .relations
                .into_iter()
                .filter(|r| {
                    r.relation_type.to_lowercase().contains(&query)
                        || matched_names.iter().any(|n| {
                            n.eq_ignore_ascii_case(&r.from) || n.eq_ignore_ascii_case(&r.to)
                        })
                })
                .collect();

            Ok(json!({
                "entities": matched_entities,
                "relations": matched_relations
            }))
        }

        "open_nodes" => {
            let names_val = request
                .arguments
                .get("names")
                .ok_or_else(|| "Missing 'names' parameter".to_string())?;
            let names: Vec<String> = serde_json::from_value(names_val.clone())
                .map_err(|e| format!("Invalid 'names' format: {}", e))?;

            let matched_entities: Vec<Entity> = graph
                .entities
                .into_iter()
                .filter(|e| names.iter().any(|n| n.eq_ignore_ascii_case(&e.name)))
                .collect();

            let matched_relations: Vec<Relation> = graph
                .relations
                .into_iter()
                .filter(|r| {
                    names
                        .iter()
                        .any(|n| n.eq_ignore_ascii_case(&r.from) || n.eq_ignore_ascii_case(&r.to))
                })
                .collect();

            Ok(json!({
                "entities": matched_entities,
                "relations": matched_relations
            }))
        }

        unknown => Err(format!("Unknown memory tool: {}", unknown)),
    }
}
