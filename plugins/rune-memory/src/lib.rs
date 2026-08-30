// plugins/rune-memory/src/lib.rs
use extism_pdk::*;
use rune_pdk::{ToolCallRequest, ToolDefinition};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Entity {
    pub name: String,
    #[serde(rename = "entityType", alias = "entity_type")]
    pub entity_type: String,
    #[serde(default)]
    pub observations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Relation {
    pub from: String,
    pub to: String,
    #[serde(rename = "relationType", alias = "relation_type")]
    pub relation_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KnowledgeGraph {
    #[serde(default)]
    pub entities: Vec<Entity>,
    #[serde(default)]
    pub relations: Vec<Relation>,
}

fn resolve_storage_path() -> PathBuf {
    let filename = config::get("memory_file")
        .unwrap_or(None)
        .unwrap_or_else(|| "memory.json".to_string());

    if let Ok(Some(allowed_root)) = config::get("allowed_dir") {
        PathBuf::from(allowed_root).join(filename)
    } else {
        PathBuf::from(filename)
    }
}

fn load_graph() -> KnowledgeGraph {
    let path = resolve_storage_path();
    if let Ok(data) = fs::read_to_string(&path) {
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        KnowledgeGraph::default()
    }
}

fn save_graph(graph: &KnowledgeGraph) -> Result<(), String> {
    let path = resolve_storage_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let data = serde_json::to_string_pretty(graph)
        .map_err(|e| format!("Failed to serialize knowledge graph: {}", e))?;
    fs::write(&path, data).map_err(|e| {
        format!(
            "Failed to write knowledge graph to '{}': {}",
            path.display(),
            e
        )
    })?;
    Ok(())
}

fn execute_tool(request: ToolCallRequest) -> Result<Value, String> {
    let mut graph = load_graph();

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

            save_graph(&graph)?;
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

            save_graph(&graph)?;
            Ok(json!({ "created": created }))
        }

        "add_observations" => {
            #[derive(Deserialize)]
            struct AddObs {
                #[serde(rename = "entityName", alias = "entity_name")]
                entity_name: String,
                contents: Vec<String>,
            }

            let obs_val = request
                .arguments
                .get("observations")
                .ok_or_else(|| "Missing 'observations' parameter".to_string())?;
            let additions: Vec<AddObs> = serde_json::from_value(obs_val.clone())
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

            save_graph(&graph)?;
            Ok(json!({ "updated": updated }))
        }

        "delete_entities" => {
            let names: Vec<String> = request
                .arguments
                .get("entityNames")
                .or_else(|| request.arguments.get("entity_names"))
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .ok_or_else(|| "Missing 'entityNames' parameter".to_string())?;

            graph
                .entities
                .retain(|e| !names.iter().any(|n| n.eq_ignore_ascii_case(&e.name)));
            graph.relations.retain(|r| {
                !names
                    .iter()
                    .any(|n| n.eq_ignore_ascii_case(&r.from) || n.eq_ignore_ascii_case(&r.to))
            });

            save_graph(&graph)?;
            Ok(json!({ "deleted": names }))
        }

        "delete_observations" => {
            #[derive(Deserialize)]
            struct DelObs {
                #[serde(rename = "entityName", alias = "entity_name")]
                entity_name: String,
                observations: Vec<String>,
            }

            let del_val = request
                .arguments
                .get("deletions")
                .ok_or_else(|| "Missing 'deletions' parameter".to_string())?;
            let deletions: Vec<DelObs> = serde_json::from_value(del_val.clone())
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

            save_graph(&graph)?;
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
                        && del.relation_type.eq_ignore_ascii_case(&r.relation_type)
                })
            });

            save_graph(&graph)?;
            Ok(json!({ "status": "success" }))
        }

        "read_graph" => Ok(json!({
            "entities": graph.entities,
            "relations": graph.relations
        })),

        "search_nodes" => {
            let query = request.arguments["query"]
                .as_str()
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
            let names: Vec<String> = request
                .arguments
                .get("names")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .ok_or_else(|| "Missing 'names' parameter".to_string())?;

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

#[plugin_fn]
pub fn mcp_info(_: ()) -> FnResult<String> {
    let info = json!({
        "name": env!("CARGO_PKG_NAME"),
        "version": env!("CARGO_PKG_VERSION"),
        "description": option_env!("CARGO_PKG_DESCRIPTION")
    });
    Ok(serde_json::to_string(&info)?)
}

#[plugin_fn]
pub fn mcp_list_tools(_: ()) -> FnResult<String> {
    let tools = vec![
        ToolDefinition {
            name: "create_entities".to_string(),
            description: "Create multiple new entities in the knowledge graph".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "entities": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "name": { "type": "string", "description": "The name of the entity" },
                                "entityType": { "type": "string", "description": "The type of the entity" },
                                "observations": { "type": "array", "items": { "type": "string" }, "description": "Observations associated with the entity" }
                            },
                            "required": ["name", "entityType", "observations"]
                        }
                    }
                },
                "required": ["entities"]
            }),
        },
        ToolDefinition {
            name: "create_relations".to_string(),
            description: "Create multiple new relations between entities in the knowledge graph"
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "relations": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "from": { "type": "string", "description": "The source entity name" },
                                "to": { "type": "string", "description": "The target entity name" },
                                "relationType": { "type": "string", "description": "The type of relation" }
                            },
                            "required": ["from", "to", "relationType"]
                        }
                    }
                },
                "required": ["relations"]
            }),
        },
        ToolDefinition {
            name: "add_observations".to_string(),
            description: "Add new observations to existing entities in the knowledge graph"
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "observations": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "entityName": { "type": "string", "description": "The entity name" },
                                "contents": { "type": "array", "items": { "type": "string" }, "description": "Observations to append" }
                            },
                            "required": ["entityName", "contents"]
                        }
                    }
                },
                "required": ["observations"]
            }),
        },
        ToolDefinition {
            name: "delete_entities".to_string(),
            description: "Delete multiple entities and their relations from the knowledge graph"
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "entityNames": { "type": "array", "items": { "type": "string" }, "description": "Array of entity names to delete" }
                },
                "required": ["entityNames"]
            }),
        },
        ToolDefinition {
            name: "delete_observations".to_string(),
            description: "Delete specific observations from entities in the knowledge graph"
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "deletions": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "entityName": { "type": "string", "description": "The entity name" },
                                "observations": { "type": "array", "items": { "type": "string" }, "description": "Observations to delete" }
                            },
                            "required": ["entityName", "observations"]
                        }
                    }
                },
                "required": ["deletions"]
            }),
        },
        ToolDefinition {
            name: "delete_relations".to_string(),
            description: "Delete multiple relations from the knowledge graph".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "relations": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "from": { "type": "string" },
                                "to": { "type": "string" },
                                "relationType": { "type": "string" }
                            },
                            "required": ["from", "to", "relationType"]
                        }
                    }
                },
                "required": ["relations"]
            }),
        },
        ToolDefinition {
            name: "read_graph".to_string(),
            description: "Read the entire knowledge graph of entities and relations".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
        },
        ToolDefinition {
            name: "search_nodes".to_string(),
            description:
                "Search for nodes and relations in the knowledge graph based on query substring"
                    .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Search keyword" }
                },
                "required": ["query"]
            }),
        },
        ToolDefinition {
            name: "open_nodes".to_string(),
            description: "Open specific nodes by name to inspect details and connected relations"
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "names": { "type": "array", "items": { "type": "string" }, "description": "Array of entity names to open" }
                },
                "required": ["names"]
            }),
        },
    ];

    Ok(serde_json::to_string(&tools)?)
}

#[plugin_fn]
pub fn mcp_call_tool(input: String) -> FnResult<String> {
    let request: ToolCallRequest = serde_json::from_str(&input)?;
    let result = execute_tool(request);

    let output = match result {
        Ok(val) => json!({ "status": "success", "result": val }),
        Err(err) => json!({ "status": "error", "error": err }),
    };

    Ok(serde_json::to_string(&output)?)
}
