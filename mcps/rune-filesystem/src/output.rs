//! Bound the serialized wire result, not just unescaped file names or row counts.
use serde_json::{Value, json};

/// Includes both text and structured data, JSON escaping, metadata and notices.
/// Kit permits 8 MiB; this 1 MiB budget leaves 7 MiB of transport headroom.
pub(crate) const MAX_RESULT_BYTES: usize = 1024 * 1024;

pub(crate) struct Listing {
    pub items: Vec<Value>,
    pub total: usize,
    kind: Kind,
}

enum Kind {
    Lines,
    Directories,
    Tree(Value),
}

impl Listing {
    pub fn lines(lines: &[String]) -> Self {
        Self {
            items: lines.iter().map(|line| json!(line)).collect(),
            total: lines.len(),
            kind: Kind::Lines,
        }
    }

    pub fn directories(root: &str) -> Self {
        Self {
            items: vec![json!(root)],
            total: 1,
            kind: Kind::Directories,
        }
    }

    pub fn tree(tree: &Value) -> Self {
        fn visit(node: &Value, path: String, items: &mut Vec<Value>) {
            items.push(json!({"name": node["name"], "type": node["type"], "path": path}));
            if let Some(children) = node["children"].as_array() {
                for child in children {
                    let child_path = format!("{}/{}", path, child["name"].as_str().unwrap());
                    visit(child, child_path, items);
                }
            }
        }
        let mut items = Vec::new();
        visit(tree, tree["name"].as_str().unwrap().into(), &mut items);
        Self {
            total: items.len(),
            items,
            kind: Kind::Tree(tree.clone()),
        }
    }

    fn content(&self, count: usize) -> String {
        match &self.kind {
            Kind::Lines => self.items[..count]
                .iter()
                .map(|item| item.as_str().unwrap())
                .collect::<Vec<_>>()
                .join("\n"),
            Kind::Directories => serde_json::to_string(&self.items[..count]).unwrap(),
            Kind::Tree(tree) => {
                // A prefix in traversal order always includes each retained
                // node's ancestors. Prune at node boundaries, never JSON bytes.
                fn prune(node: &Value, remaining: &mut usize) -> Option<Value> {
                    if *remaining == 0 {
                        return None;
                    }
                    *remaining -= 1;
                    let mut kept = node.clone();
                    if let Some(children) = node["children"].as_array() {
                        kept["children"] = children
                            .iter()
                            .map_while(|child| prune(child, remaining))
                            .collect();
                    }
                    Some(kept)
                }
                let mut remaining = count;
                let kept = prune(tree, &mut remaining).unwrap_or(json!([]));
                serde_json::to_string_pretty(&kept).unwrap()
            }
        }
    }
}

pub(crate) fn success(legacy: Value, structured: Value) -> Value {
    let text = json!({"status": "success", "result": legacy}).to_string();
    rune_pdk::structured_tool_result(text, structured.as_object().unwrap().clone())
}

fn bytes(result: &Value) -> usize {
    serde_json::to_vec(result).unwrap().len()
}

/// Find the longest complete prefix that fits, measuring the actual wire bytes.
fn bounded_prefix(max: usize, max_content_bytes: usize, build: impl Fn(usize) -> Value) -> Value {
    let fits = |result: &Value| {
        bytes(result) <= MAX_RESULT_BYTES
            && result["structuredContent"]["content"]
                .as_str()
                .unwrap()
                .len()
                <= max_content_bytes
    };
    let full = build(max);
    if fits(&full) {
        return full;
    }
    let (mut low, mut high) = (0, max);
    while low < high {
        let mid = low + (high - low).div_ceil(2);
        if fits(&build(mid)) {
            low = mid;
        } else {
            high = mid - 1;
        }
    }
    build(low)
}

pub(crate) fn listing_result(legacy: Value, listing: &Listing) -> Value {
    // Keep only the small metadata template; do not retain the old full content
    // in a truncated result, where it could by itself exceed Kit's ceiling.
    let mut metadata = legacy.as_object().unwrap().clone();
    metadata.remove("content");
    // The notice must also fit the existing pilot's 64 KiB content limit.
    let max_content_bytes = metadata
        .get("maxOutputBytes")
        .and_then(Value::as_u64)
        .map(|bytes| bytes as usize)
        .unwrap_or(usize::MAX);
    bounded_prefix(listing.items.len(), max_content_bytes, |count| {
        let truncated = count < listing.total;
        let mut displayed = Value::Object(metadata.clone());
        if truncated {
            displayed["content"] = json!(format!(
                "{}\n[Result truncated: omitted {} of {} items.]",
                listing.content(count),
                listing.total - count,
                listing.total
            ));
            if displayed.get("entriesReturned").is_some() {
                displayed["entriesReturned"] = json!(count);
                displayed["hasMore"] = json!(true);
                displayed["truncated"] = json!(true);
            }
        } else {
            displayed["content"] = legacy["content"].clone();
        }
        let mut structured = displayed.clone();
        structured["entries"] = json!(&listing.items[..count]);
        structured["total"] = json!(listing.total);
        structured["truncated"] = json!(truncated);
        success(displayed, structured)
    })
}

pub(crate) fn text_result(legacy: Value) -> Value {
    let text = legacy["content"].as_str().unwrap();
    let total = text.len();
    bounded_prefix(total, usize::MAX, |end| {
        let mut end = end;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        let truncated = end < total;
        let displayed = if truncated {
            json!({"content": format!(
                "{}\n[Output truncated: omitted {} of {} UTF-8 bytes.]",
                &text[..end], total - end, total
            )})
        } else {
            legacy.clone()
        };
        let mut structured = displayed.clone();
        structured["total"] = json!(total);
        structured["truncated"] = json!(truncated);
        success(displayed, structured)
    })
}
