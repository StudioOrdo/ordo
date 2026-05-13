use anyhow::{anyhow, bail, Context, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

static VALIDATOR_CACHE: OnceLock<Mutex<HashMap<String, Arc<jsonschema::Validator>>>> =
    OnceLock::new();

#[derive(Clone)]
pub struct JsonContract {
    validator: Arc<jsonschema::Validator>,
}

pub fn compile_json_schema(schema: &Value, label: &str) -> Result<JsonContract> {
    reject_untrusted_refs(schema, label)?;
    jsonschema::meta::validate(schema)
        .map_err(|error| anyhow!(format_schema_error(label, &error)))?;

    let cache_key = serde_json::to_string(schema)
        .with_context(|| format!("{label} could not be serialized for validator cache"))?;
    if let Some(validator) = validator_from_cache(&cache_key)? {
        return Ok(JsonContract { validator });
    }

    let compiled = Arc::new(
        jsonschema::validator_for(schema)
            .map_err(|error| anyhow!(format_schema_error(label, &error)))?,
    );
    let validator = cache_validator(cache_key, compiled)?;
    Ok(JsonContract { validator })
}

pub fn validate_json_schema_document(schema: &Value, label: &str) -> Result<()> {
    compile_json_schema(schema, label).map(|_| ())
}

pub fn validate_json_value(schema: &Value, instance: &Value, label: &str) -> Result<()> {
    let contract = compile_json_schema(schema, label)?;
    contract.validate(instance, label)
}

impl JsonContract {
    pub fn validate(&self, instance: &Value, label: &str) -> Result<()> {
        if let Err(error) = self.validator.validate(instance) {
            bail!("{}", format_validation_error(label, &error));
        }
        Ok(())
    }

    #[cfg(test)]
    fn cache_identity(&self) -> usize {
        Arc::as_ptr(&self.validator) as usize
    }
}

fn validator_cache() -> &'static Mutex<HashMap<String, Arc<jsonschema::Validator>>> {
    VALIDATOR_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn validator_from_cache(cache_key: &str) -> Result<Option<Arc<jsonschema::Validator>>> {
    let cache = validator_cache()
        .lock()
        .map_err(|_| anyhow!("JSON Schema validator cache lock was poisoned"))?;
    Ok(cache.get(cache_key).cloned())
}

fn cache_validator(
    cache_key: String,
    compiled: Arc<jsonschema::Validator>,
) -> Result<Arc<jsonschema::Validator>> {
    let mut cache = validator_cache()
        .lock()
        .map_err(|_| anyhow!("JSON Schema validator cache lock was poisoned"))?;
    Ok(cache.entry(cache_key).or_insert(compiled).clone())
}

fn reject_untrusted_refs(schema: &Value, label: &str) -> Result<()> {
    reject_untrusted_refs_at(schema, label, "")
}

fn reject_untrusted_refs_at(value: &Value, label: &str, pointer: &str) -> Result<()> {
    match value {
        Value::Object(object) => {
            for keyword in ["$ref", "$dynamicRef"] {
                if let Some(reference) = object.get(keyword).and_then(Value::as_str) {
                    if !reference.starts_with('#') {
                        bail!(
                            "{label} contains unsupported {keyword} at {}; only local fragment references are allowed",
                            pointer_for_child(pointer, keyword)
                        );
                    }
                }
            }
            for (key, child) in object {
                reject_untrusted_refs_at(child, label, &pointer_for_child(pointer, key))?;
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                reject_untrusted_refs_at(
                    child,
                    label,
                    &pointer_for_child(pointer, &index.to_string()),
                )?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn pointer_for_child(parent: &str, child: &str) -> String {
    let escaped = child.replace('~', "~0").replace('/', "~1");
    if parent.is_empty() {
        format!("/{escaped}")
    } else {
        format!("{parent}/{escaped}")
    }
}

fn format_schema_error(label: &str, error: &jsonschema::ValidationError<'_>) -> String {
    format!(
        "{label} is not a valid JSON Schema at {} (schema {}): {}",
        display_location(error.instance_path()),
        display_location(error.schema_path()),
        error.masked()
    )
}

fn format_validation_error(label: &str, error: &jsonschema::ValidationError<'_>) -> String {
    let message = if error
        .schema_path()
        .to_string()
        .ends_with("/additionalProperties")
        && error.kind().keyword() == "falseSchema"
    {
        "Additional properties are not allowed".to_string()
    } else {
        error.masked().to_string()
    };
    format!(
        "{label} failed JSON Schema validation at {} (schema {}): {}",
        display_location(error.instance_path()),
        display_location(error.schema_path()),
        message
    )
}

fn display_location(location: &impl ToString) -> String {
    let location = location.to_string();
    if location.is_empty() {
        "/".to_string()
    } else {
        location
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn pilot_task_schema() -> Value {
        json!({
            "type": "object",
            "required": ["request"],
            "properties": {
                "request": {
                    "type": "object",
                    "required": ["title", "format", "shots"],
                    "properties": {
                        "title": { "type": "string", "minLength": 1 },
                        "format": { "enum": ["short", "report"] },
                        "shots": {
                            "type": "array",
                            "minItems": 1,
                            "items": {
                                "type": "object",
                                "required": ["seconds", "visual"],
                                "properties": {
                                    "seconds": { "type": "integer", "minimum": 3, "maximum": 15 },
                                    "visual": { "type": "string" }
                                },
                                "additionalProperties": false
                            }
                        },
                        "metadata": {
                            "type": "object",
                            "additionalProperties": true
                        }
                    },
                    "additionalProperties": false
                }
            },
            "additionalProperties": false
        })
    }

    #[test]
    fn validates_nested_objects_arrays_enums_and_extension_metadata() {
        let schema = pilot_task_schema();
        let instance = json!({
            "request": {
                "title": "Photosynthesis explainer",
                "format": "short",
                "shots": [
                    { "seconds": 6, "visual": "leaf absorbing sunlight" },
                    { "seconds": 9, "visual": "glucose molecule animation" }
                ],
                "metadata": {
                    "legacyOfferId": "offer_123",
                    "experiment": { "variant": "nyc-pilot" }
                }
            }
        });

        validate_json_schema_document(&schema, "taskInput").unwrap();
        validate_json_value(&schema, &instance, "taskInput").unwrap();
    }

    #[test]
    fn rejects_required_nested_properties() {
        let schema = pilot_task_schema();
        let instance = json!({
            "request": {
                "title": "Photosynthesis explainer",
                "format": "short",
                "shots": [{ "seconds": 6 }]
            }
        });

        let error = validate_json_value(&schema, &instance, "taskInput")
            .unwrap_err()
            .to_string();

        assert!(error.contains("/request/shots/0"));
        assert!(error.contains("visual"));
        assert!(error.contains("required property"));
    }

    #[test]
    fn rejects_array_enum_and_additional_properties_violations() {
        let schema = pilot_task_schema();
        let bad_enum = json!({
            "request": {
                "title": "Photosynthesis explainer",
                "format": "thread",
                "shots": [{ "seconds": 6, "visual": "leaf" }]
            }
        });
        let extra_execution_field = json!({
            "request": {
                "title": "Photosynthesis explainer",
                "format": "short",
                "shots": [{ "seconds": 6, "visual": "leaf", "shellCommand": "publish-now" }]
            }
        });

        let enum_error = validate_json_value(&schema, &bad_enum, "taskInput")
            .unwrap_err()
            .to_string();
        let extra_error = validate_json_value(&schema, &extra_execution_field, "taskInput")
            .unwrap_err()
            .to_string();

        assert!(enum_error.contains("/request/format"));
        assert!(enum_error.contains("not one of"));
        assert!(extra_error.contains("Additional properties are not allowed"));
        assert!(extra_error.contains("shellCommand"));
    }

    #[test]
    fn rejects_invalid_schema_documents_with_safe_errors() {
        let schema = json!({
            "type": "object",
            "required": [42]
        });

        let error = validate_json_schema_document(&schema, "inputSchema")
            .unwrap_err()
            .to_string();

        assert!(error.contains("not a valid JSON Schema"));
        assert!(error.contains("/required/0"));
        assert!(!error.contains("42"));
    }

    #[test]
    fn rejects_non_local_refs_without_echoing_target() {
        let schema = json!({
            "$ref": "https://example.com/secret-schema.json"
        });

        let error = validate_json_schema_document(&schema, "inputSchema")
            .unwrap_err()
            .to_string();

        assert!(error.contains("unsupported $ref"));
        assert!(error.contains("only local fragment references are allowed"));
        assert!(!error.contains("example.com"));
    }

    #[test]
    fn allows_local_fragment_refs() {
        let schema = json!({
            "$defs": {
                "nonEmptyText": { "type": "string", "minLength": 1 }
            },
            "type": "object",
            "required": ["title"],
            "properties": {
                "title": { "$ref": "#/$defs/nonEmptyText" }
            },
            "additionalProperties": false
        });

        validate_json_value(&schema, &json!({ "title": "NYC pilot" }), "taskInput").unwrap();
    }

    #[test]
    fn validation_errors_do_not_include_secret_instance_values() {
        let schema = json!({
            "type": "object",
            "required": ["apiKey"],
            "properties": {
                "apiKey": { "type": "string", "maxLength": 3 }
            },
            "additionalProperties": false
        });
        let instance = json!({ "apiKey": "sk-secret-token" });

        let error = validate_json_value(&schema, &instance, "providerConfig")
            .unwrap_err()
            .to_string();

        assert!(error.contains("value is longer than 3 characters"));
        assert!(!error.contains("sk-secret-token"));
    }

    #[test]
    fn compiled_schema_validators_are_cached() {
        let schema = pilot_task_schema();

        let first = compile_json_schema(&schema, "taskInput").unwrap();
        let second = compile_json_schema(&schema, "taskInput").unwrap();

        assert_eq!(first.cache_identity(), second.cache_identity());
    }
}
