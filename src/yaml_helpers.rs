use anyhow::Result;
use serde::de::DeserializeOwned;
use std::collections::HashMap;

pub fn parse_unity_yaml<T: DeserializeOwned>(file: &str) -> Result<HashMap<i64, T>> {
    let file = cleanup_unity_yaml(file)?;
    let parse: HashMap<i64, T> = serde_yaml::from_str(&file)?;

    Ok(parse)
}

fn cleanup_unity_yaml(yaml: &str) -> Result<String> {
    let lines: Vec<String> = yaml
        .lines()
        .filter_map(|line| {
            if line.starts_with("%YAML") || line.starts_with("%TAG") {
                // unity specific headers. SKIP!
                None
            } else if line.starts_with("--- !u!") {
                // unity object id declared on this line
                // --- !u!104 &2 => 104 is object type and 2 is object id
                let mut splits = line.split_whitespace();
                let object_id: i64 = splits
                    .find(|&part| part.starts_with('&'))
                    .and_then(|num| num[1..].parse().ok())?;

                Some(format!("{}:", object_id))
            } else if line.starts_with(' ') {
                Some(line.to_string())
            } else {
                Some(format!("  object_type: {}", line.replace(':', "")))
            }
        })
        .collect();

    let mut lines = lines.join("\n");

    lines.push('\n'); // insert new line at the end

    Ok(lines)
}
