use rune_pdk::ToolDefinition;
use serde_json::json;

pub fn tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "create_entities".to_string(),
            description: "Create multiple new entities in the knowledge graph".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "entities": {
                        "type": "array",
                        "description": "List of entities to create in the knowledge graph",
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
                        "description": "List of relations to establish between entities",
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
                        "description": "List of observations to append to existing entities",
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
                    "entityNames": {
                        "type": "array",
                        "description": "Array of entity names to delete",
                        "items": { "type": "string" }
                    }
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
                        "description": "List of observations to remove from entities",
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
                        "description": "List of relations to remove from the knowledge graph",
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
                    "names": {
                        "type": "array",
                        "description": "Array of entity names to open",
                        "items": { "type": "string" }
                    }
                },
                "required": ["names"]
            }),
        },
    ]
}
