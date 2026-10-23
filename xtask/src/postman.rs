use serde_json::{Value, json};
use std::{fs, path::Path};
#[allow(dead_code)]
#[path = "../../plugins/mhb-mconnect/src/definitions.rs"]
mod mhb_mconnect;
#[allow(dead_code)]
#[path = "../../plugins/rune-audio/src/definitions.rs"]
mod rune_audio;
#[allow(dead_code)]
#[path = "../../plugins/rune-browser/src/definitions.rs"]
mod rune_browser;
#[allow(dead_code)]
#[path = "../../plugins/rune-email/src/definitions.rs"]
mod rune_email;
#[allow(dead_code)]
#[path = "../../plugins/rune-fetch/src/definitions.rs"]
mod rune_fetch;
#[allow(dead_code)]
#[path = "../../plugins/rune-figma/src/definitions.rs"]
mod rune_figma;
#[allow(dead_code)]
#[path = "../../plugins/rune-filesystem/src/definitions.rs"]
mod rune_filesystem;
#[allow(dead_code)]
#[path = "../../plugins/rune-git/src/definitions.rs"]
mod rune_git;
#[allow(dead_code)]
#[path = "../../plugins/rune-image/src/definitions.rs"]
mod rune_image;
#[allow(dead_code)]
#[path = "../../plugins/rune-memory/src/definitions.rs"]
mod rune_memory;
#[allow(dead_code)]
#[path = "../../plugins/rune-print/src/definitions.rs"]
mod rune_print;
#[allow(dead_code)]
#[path = "../../plugins/rune-scan/src/definitions.rs"]
mod rune_scan;
#[allow(dead_code)]
#[path = "../../plugins/rune-sequential-thinking/src/definitions.rs"]
mod rune_sequential_thinking;
#[allow(dead_code)]
#[path = "../../plugins/rune-slides/src/definitions.rs"]
mod rune_slides;
#[allow(dead_code)]
#[path = "../../plugins/rune-ssh/src/definitions.rs"]
mod rune_ssh;
#[allow(dead_code)]
#[path = "../../plugins/rune-time/src/definitions.rs"]
mod rune_time;
#[allow(dead_code)]
#[path = "../../plugins/rune-video/src/definitions.rs"]
mod rune_video;
fn plugins() -> Vec<(&'static str, Vec<rune_pdk::ToolDefinition>)> {
    vec![
        ("mhb-mconnect", mhb_mconnect::tool_definitions()),
        ("rune-audio", rune_audio::tool_definitions()),
        ("rune-browser", rune_browser::tool_definitions()),
        ("rune-email", rune_email::tool_definitions()),
        ("rune-fetch", rune_fetch::tool_definitions()),
        ("rune-figma", rune_figma::tool_definitions()),
        ("rune-filesystem", rune_filesystem::tool_definitions()),
        ("rune-git", rune_git::tool_definitions()),
        ("rune-image", rune_image::tool_definitions()),
        ("rune-memory", rune_memory::tool_definitions()),
        ("rune-print", rune_print::tool_definitions()),
        ("rune-scan", rune_scan::tool_definitions()),
        (
            "rune-sequential-thinking",
            rune_sequential_thinking::tool_definitions(),
        ),
        ("rune-slides", rune_slides::tool_definitions()),
        ("rune-ssh", rune_ssh::tool_definitions()),
        ("rune-time", rune_time::tool_definitions()),
        ("rune-video", rune_video::tool_definitions()),
    ]
}

fn quote(value: &str) -> String {
    serde_json::to_string(value).unwrap()
}

fn example(schema: &Value) -> Value {
    if let Some(value) = schema.get("default").or_else(|| schema.get("example")) {
        return value.clone();
    }
    if let Some(value) = schema.get("enum").and_then(|v| v.get(0)) {
        return value.clone();
    }
    match schema["type"].as_str().unwrap_or("object") {
        "string" => json!("<set this value>"),
        "integer" | "number" => schema.get("minimum").cloned().unwrap_or(json!(1)),
        "boolean" => json!(false),
        "array" => json!([example(&schema["items"])]),
        _ => {
            let mut object = serde_json::Map::new();
            if let Some(required) = schema["required"].as_array() {
                for field in required.iter().filter_map(Value::as_str) {
                    object.insert(field.into(), example(&schema["properties"][field]));
                }
            }
            Value::Object(object)
        }
    }
}

fn write_or_check(path: &Path, content: &str, check: bool) -> Result<(), String> {
    if check {
        if fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))? != content {
            return Err(format!(
                "{} is out of date; run cargo xtask postman",
                path.display()
            ));
        }
    } else {
        fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
        fs::write(path, content).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn command(check: bool) -> Result<(), String> {
    let base = Path::new("postman/collections/Rune Kit");
    let mut total = 0;
    let mut inventory = Vec::new();
    let mut root_definition = String::from(
        "$kind: collection\nname: Rune Kit\ndescription: All plugin tool calls through public Kit MCP 2026-07-28 stdio.\nvariables:\n  kit_executable: rune-kit\n  read_only_flag: --read-only\n  allowed_root: \"\"\n  read_path: README.md\n  rules_namespace: git\n  git_validator: git__validate_commit_message\n  time_tool: time__get_current_time\n  file_reader: filesystem__read_text_file\n  directory_tool: filesystem__list_directory\n  file_writer: filesystem__write_file\n",
    );
    for (plugin, tools) in plugins() {
        let folder = base.join(plugin);
        let manifest: toml::Value = toml::from_str(
            &fs::read_to_string(format!("plugins/{plugin}/plugin.toml"))
                .map_err(|e| e.to_string())?
                .replace("\r\n", "\n"),
        )
        .map_err(|e| e.to_string())?;
        let version = manifest
            .get("plugin")
            .and_then(|v| v.get("version"))
            .and_then(toml::Value::as_str)
            .unwrap_or("not specified in plugin.toml");
        let key = plugin.replace('-', "_");
        root_definition.push_str(&format!(
            "  {key}_selector: {}\n  {key}_namespace: {}\n  {key}_parameters: {}\n",
            quote(plugin),
            quote(plugin),
            quote(if plugin == "rune-filesystem" {
                "-p \"allowed_dir={{allowed_root}}\""
            } else {
                ""
            })
        ));

        let definition = format!(
            "$kind: collection\nname: {}\ndescription: {}\n",
            quote(plugin),
            quote(&format!(
                "Source version: {version}. Configure {key}_selector, {key}_namespace and {key}_parameters in collection variables. Review plugin README and argument placeholders. Read-only mode is enabled by default."
            ))
        );
        write_or_check(
            &folder.join(".resources/definition.yaml"),
            &definition,
            check,
        )?;
        for tool in &tools {
            let message = json!({"jsonrpc":"2.0","id":format!("{plugin}/{}",tool.name),"method":"tools/call","params":{"name":format!("{{{{{key}_namespace}}}}__{}",tool.name),"arguments":example(&tool.input_schema),"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{},"io.modelcontextprotocol/clientInfo":{"name":"postman-rune-tools","version":"1.0.0"}}}});
            let description = format!(
                "{}\n\nArgument TEMPLATE only: replace <set this value> and review optional properties in inputSchema. Source: plugins/{plugin}/src/definitions.rs ({version}). This request may have external effects when read_only_flag is cleared; no call is executed by generation.\n\ninputSchema:\n{}\n\nPlugin rules:\n{}",
                tool.description,
                serde_json::to_string_pretty(&tool.input_schema).unwrap(),
                serde_json::to_string_pretty(&tool.rules).unwrap()
            );
            let content = format!(
                "$kind: mcp-request\nname: {}\ndescription: {}\ntransport: stdio\ncommand: '\"{{{{kit_executable}}}}\" run \"{{{{{key}_selector}}}}\" {{{{read_only_flag}}}} --timeout 10 {{{{{key}_parameters}}}}'\nsettings:\n  version: '2026-07-28'\n  requestTimeout: 15000\nmessage: |-\n  {}\n",
                quote(&tool.name),
                quote(&description),
                serde_json::to_string(&message).unwrap()
            );
            write_or_check(
                &folder.join(format!("{}.request.yaml", tool.name)),
                &content,
                check,
            )?;
        }
        let existing = fs::read_dir(&folder)
            .map_err(|e| e.to_string())?
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .ends_with(".request.yaml")
            })
            .count();
        if existing != tools.len() {
            return Err(format!(
                "{plugin}: stale or missing request files: expected {}, found {existing}",
                tools.len()
            ));
        }
        total += tools.len();
        inventory.push(json!({"plugin":plugin,"version":version,"tools":tools.iter().map(|t| &t.name).collect::<Vec<_>>()}));
    }
    write_or_check(
        &base.join(".resources/definition.yaml"),
        &root_definition,
        check,
    )?;
    let count = inventory.len();
    write_or_check(
        Path::new("postman/tool-coverage.json"),
        &(serde_json::to_string_pretty(&inventory).unwrap() + "\n"),
        check,
    )?;
    println!(
        "{} {total} tool calls across {count} plugin folders",
        if check { "Verified" } else { "Generated" }
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn required_arguments_preserve_types_and_prefer_defaults_and_enums() {
        let schema = json!({"type":"object","required":["name","count","mode","enabled","paths"],"properties":{"name":{"type":"string"},"count":{"type":"integer","minimum":3},"mode":{"type":"string","enum":["read","write"]},"enabled":{"type":"boolean","default":true},"paths":{"type":"array","items":{"type":"string"}},"optional":{"type":"string"}}});
        assert_eq!(
            example(&schema),
            json!({"name":"<set this value>","count":3,"mode":"read","enabled":true,"paths":["<set this value>"]})
        );
    }
}
