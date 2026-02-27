use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FieldType {
    Text,
    Keyword,
    Integer,
    Long,
    Double,
    Boolean,
    Date,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Property {
    #[serde(rename = "type")]
    pub field_type: FieldType,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Mapping {
    #[serde(default = "default_dynamic")]
    pub dynamic: bool,
    #[serde(default)]
    pub properties: HashMap<String, Property>,
}

fn default_dynamic() -> bool {
    true
}

impl Default for Mapping {
    fn default() -> Self {
        Self {
            dynamic: true,
            properties: HashMap::new(),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum ValidationError {
    MissingField(String),
    InvalidType {
        field: String,
        expected: FieldType,
    },
    UnknownField(String),
}

impl Mapping {
    pub fn update(&mut self, other: Mapping) {
        for (key, value) in other.properties {
            self.properties.insert(key, value);
        }
        self.dynamic = other.dynamic;
    }

    pub fn validate(&self, document: &serde_json::Value) -> Result<(), ValidationError> {
        let obj = document.as_object().ok_or_else(|| ValidationError::InvalidType {
            field: "root".to_string(),
            expected: FieldType::Keyword,
        })?;

        for (field_name, property) in &self.properties {
            let value = obj
                .get(field_name)
                .ok_or_else(|| ValidationError::MissingField(field_name.clone()))?;

            self.validate_type(field_name, value, &property.field_type)?;
        }

        if !self.dynamic {
            for key in obj.keys() {
                if key != "_id" && !self.properties.contains_key(key) {
                    return Err(ValidationError::UnknownField(key.clone()));
                }
            }
        }

        Ok(())
    }

    fn validate_type(
        &self,
        field: &str,
        value: &serde_json::Value,
        expected: &FieldType,
    ) -> Result<(), ValidationError> {
        let valid = match expected {
            FieldType::Text | FieldType::Keyword => value.is_string(),
            FieldType::Integer | FieldType::Long => value.is_i64() || value.is_u64(),
            FieldType::Double => value.is_f64() || value.is_i64() || value.is_u64(),
            FieldType::Boolean => value.is_boolean(),
            FieldType::Date => value.is_string(),
        };

        if valid {
            Ok(())
        } else {
            Err(ValidationError::InvalidType {
                field: field.to_string(),
                expected: expected.clone(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn setup_mapping() -> Mapping {
        let mut properties = HashMap::new();
        properties.insert(
            "title".to_string(),
            Property {
                field_type: FieldType::Text,
            },
        );
        properties.insert(
            "count".to_string(),
            Property {
                field_type: FieldType::Integer,
            },
        );
        Mapping {
            dynamic: false,
            properties,
        }
    }

    #[test]
    fn should_validate_correct_document() {
        let mapping = setup_mapping();
        let doc = json!({
            "title": "Elasticsearch in Rust",
            "count": 10
        });

        assert!(mapping.validate(&doc).is_ok());
    }

    #[test]
    fn should_return_error_for_missing_field() {
        let mapping = setup_mapping();
        let doc = json!({
            "title": "Missing count"
        });

        assert_eq!(
            mapping.validate(&doc),
            Err(ValidationError::MissingField("count".to_string()))
        );
    }

    #[test]
    fn should_return_error_for_invalid_type() {
        let mapping = setup_mapping();
        let doc = json!({
            "title": "Wrong type",
            "count": "string instead of int"
        });

        assert_eq!(
            mapping.validate(&doc),
            Err(ValidationError::InvalidType {
                field: "count".to_string(),
                expected: FieldType::Integer
            })
        );
    }

    #[test]
    fn should_return_error_for_unknown_field() {
        let mapping = setup_mapping();
        let doc = json!({
            "title": "Title",
            "count": 1,
            "extra": "not in mapping"
        });

        assert_eq!(
            mapping.validate(&doc),
            Err(ValidationError::UnknownField("extra".to_string()))
        );
    }

    #[test]
    fn should_allow_extra_fields_when_dynamic_true() {
        let mut mapping = setup_mapping();
        mapping.dynamic = true;
        let doc = json!({
            "title": "Title",
            "count": 1,
            "extra": "I am allowed"
        });

        assert!(mapping.validate(&doc).is_ok());
    }

    #[test]
    fn should_update_mapping_with_new_properties() {
        let mut mapping = setup_mapping();
        let mut new_properties = HashMap::new();
        new_properties.insert(
            "new_field".to_string(),
            Property {
                field_type: FieldType::Boolean,
            },
        );
        let other = Mapping {
            dynamic: true,
            properties: new_properties,
        };

        mapping.update(other);

        assert!(mapping.properties.contains_key("new_field"));
        assert!(mapping.properties.contains_key("title"));
        assert!(mapping.dynamic);
    }
}