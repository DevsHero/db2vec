use serde::Deserialize;
use serde_json::Value;
use std::{collections::HashMap, fs, path::Path};

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct ExcludeEntry {
    pub table: String,
    pub ignore_table: bool,
    #[serde(default)]
    pub exclude_fields: HashMap<String, FieldExclude>,
}

impl Default for ExcludeEntry {
    fn default() -> Self {
        ExcludeEntry {
            table: String::new(),
            ignore_table: false,
            exclude_fields: HashMap::new(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum FieldExclude {
    All(bool),
    Sub(Vec<String>),
}

pub struct Excluder {
    entries: HashMap<String, ExcludeEntry>,
}

impl Excluder {
    pub fn load<P: AsRef<Path>>(path: P) -> Self {
        let data = fs::read_to_string(path).unwrap_or_else(|_| "[]".into());
        let list: Vec<ExcludeEntry> =
            serde_json::from_str(&data).unwrap_or_else(|_| Vec::new());
        let entries = list.into_iter()
            .map(|e| (e.table.clone(), e))
            .collect();
        Excluder { entries }
    }

    pub fn ignore_table(&self, table: &str) -> bool {
        self.entries
            .get(table)
            .map(|e| e.ignore_table)
            .unwrap_or(false)
    }


    pub fn filter_record(&self, record: &mut Value) {
        let table = match record.get("table").and_then(Value::as_str) {
            Some(t) => t,
            None => return,
        };
        
        if let Some(entry) = self.entries.get(table) {
            if let Value::Object(map) = record {
                for (field, rule) in &entry.exclude_fields {
                    match rule {
                        FieldExclude::All(true) => {
                            map.remove(field);
                        }
                        FieldExclude::Sub(keys) => {
                            if let Some(Value::Object(sub_map)) = map.get_mut(field) {
                                for k in keys {
                                    sub_map.remove(k);
                                }
                            } 
                         
                            else if let Some(Value::String(obj_str)) = map.get_mut(field) {
                                if obj_str.trim().starts_with('{') && obj_str.trim().ends_with('}') {
                                    for key in keys {
                                        let patterns = [
                                            format!("{}:\\s*[^,}}]+,", regex::escape(key)),
                                            format!("{}:\\s*[^,}}]+}}", regex::escape(key)),
                                            format!("\"{}\":\\s*[^,}}]+,", regex::escape(key)),
                                            format!("'{}\':\\s*[^,}}]+,", regex::escape(key)),
                                        ];
                                        
                                        for pattern in patterns {
                                            if let Ok(re) = regex::Regex::new(&pattern) {
                                                *obj_str = re.replace(obj_str, "").to_string();
                                            }
                                        }
                                        
                                        if let Ok(re) = regex::Regex::new(r",\s*}") {
                                            *obj_str = re.replace(obj_str, "}").to_string();
                                        }
                                        if let Ok(re) = regex::Regex::new(r",\s*,") {
                                            *obj_str = re.replace(obj_str, ",").to_string();
                                        }
                                    }
                                }
                            }
                        },
                        _ => {}
                    }
                }
            }
        }
    }

}