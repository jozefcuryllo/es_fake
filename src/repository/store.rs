use crate::domain::mapping::Mapping;
use dashmap::DashMap;
use serde_json::Value;
use std::sync::Arc;

#[derive(Clone)]
pub struct IndexData {
    pub mapping: Mapping,
    pub documents: Vec<Value>,
}

pub struct InMemoryStore {
    indices: DashMap<String, Arc<IndexData>>,
}

impl InMemoryStore {
    pub fn new() -> Self {
        Self {
            indices: DashMap::new(),
        }
    }

    pub fn create_index(&self, name: String, mapping: Mapping) {
        let index_data = IndexData {
            mapping,
            documents: Vec::new(),
        };
        self.indices.insert(name, Arc::new(index_data));
    }

    pub fn update_mapping(&self, name: &str, new_mapping: Mapping) -> Result<(), String> {
        let mut index_ref = self
            .indices
            .get_mut(name)
            .ok_or_else(|| "index_not_found_exception".to_string())?;

        let current_data = index_ref.value();
        let mut new_data = (**current_data).clone();
        
        new_data.mapping.update(new_mapping);
        
        *index_ref.value_mut() = Arc::new(new_data);
        Ok(())
    }

    pub fn delete_index(&self, name: &str) -> bool {
        self.indices.remove(name).is_some()
    }

    pub fn refresh(&self, index_name: &str) -> Result<(), String> {
        if self.indices.contains_key(index_name) {
            Ok(())
        } else {
            Err("index_not_found_exception".to_string())
        }
    }

    pub fn add_document(&self, index_name: &str, mut doc: Value) -> Result<String, String> {
        let mut index_ref = self
            .indices
            .get_mut(index_name)
            .ok_or_else(|| "index_not_found_exception".to_string())?;

        let current_data = index_ref.value();

        current_data
            .mapping
            .validate(&doc)
            .map_err(|e| format!("Validation failed: {:?}", e))?;

        let id = doc
            .get("_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                let new_id = uuid::Uuid::new_v4().to_string();
                if let Some(obj) = doc.as_object_mut() {
                    obj.insert("_id".to_string(), Value::String(new_id.clone()));
                }
                new_id
            });

        let mut new_data = (**current_data).clone();

        if let Some(pos) = new_data.documents.iter().position(|d| d["_id"] == id) {
            new_data.documents[pos] = doc;
        } else {
            new_data.documents.push(doc);
        }

        *index_ref.value_mut() = Arc::new(new_data);

        Ok(id)
    }

    pub fn patch_document(
        &self,
        index_name: &str,
        id: &str,
        patch: Value,
    ) -> Result<String, String> {
        let mut existing_doc = self
            .get_document(index_name, id)
            .ok_or_else(|| "document_missing_exception".to_string())?;

        if let (Some(existing_obj), Some(patch_obj)) =
            (existing_doc.as_object_mut(), patch.as_object())
        {
            for (k, v) in patch_obj {
                existing_obj.insert(k.clone(), v.clone());
            }
        }

        self.add_document(index_name, existing_doc)
    }

    pub fn get_document(&self, index_name: &str, id: &str) -> Option<Value> {
        let index = self.get_index(index_name)?;
        index.documents.iter().find(|d| d["_id"] == id).cloned()
    }

    pub fn delete_document(&self, index_name: &str, id: &str) -> bool {
        let mut index_ref = match self.indices.get_mut(index_name) {
            Some(r) => r,
            None => return false,
        };

        let mut new_data = (**index_ref.value()).clone();
        let initial_len = new_data.documents.len();
        new_data.documents.retain(|d| d["_id"] != id);

        let deleted = new_data.documents.len() < initial_len;
        *index_ref.value_mut() = Arc::new(new_data);
        deleted
    }

    pub fn get_index(&self, name: &str) -> Option<Arc<IndexData>> {
        self.indices.get(name).map(|r| Arc::clone(r.value()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::mapping::{FieldType, Property};
    use serde_json::json;
    use std::collections::HashMap;

    fn mock_mapping() -> Mapping {
        let mut properties = HashMap::new();
        properties.insert(
            "id".to_string(),
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
    fn should_update_mapping_in_store() {
        let store = InMemoryStore::new();
        store.create_index("test-index".to_string(), mock_mapping());

        let mut new_props = HashMap::new();
        new_props.insert(
            "description".to_string(),
            Property {
                field_type: FieldType::Text,
            },
        );
        let new_mapping = Mapping {
            dynamic: true,
            properties: new_props,
        };

        store.update_mapping("test-index", new_mapping).unwrap();

        let index = store.get_index("test-index").unwrap();
        assert!(index.mapping.properties.contains_key("id"));
        assert!(index.mapping.properties.contains_key("description"));
        assert!(index.mapping.dynamic);
    }

    #[test]
    fn should_delete_index() {
        let store = InMemoryStore::new();
        store.create_index("to-delete".to_string(), Mapping::default());
        assert!(store.get_index("to-delete").is_some());

        let deleted = store.delete_index("to-delete");
        assert!(deleted);
        assert!(store.get_index("to-delete").is_none());
    }

    #[test]
    fn should_handle_refresh_as_noop() {
        let store = InMemoryStore::new();
        store.create_index("refresh-me".to_string(), Mapping::default());
        let result = store.refresh("refresh-me");
        assert!(result.is_ok());
    }

    #[test]
    fn should_create_and_retrieve_index() {
        let store = InMemoryStore::new();
        let mapping = mock_mapping();

        store.create_index("test-index".to_string(), mapping);

        assert!(store.get_index("test-index").is_some());
    }

    #[test]
    fn should_reject_document_with_wrong_mapping() {
        let store = InMemoryStore::new();
        store.create_index("test-index".to_string(), mock_mapping());

        let invalid_doc = json!({ "id": "not-an-integer" });
        let result = store.add_document("test-index", invalid_doc);

        assert!(result.is_err());
    }

    #[test]
    fn should_accept_valid_document() {
        let store = InMemoryStore::new();
        store.create_index("test-index".to_string(), mock_mapping());

        let valid_doc = json!({ "id": 1 });
        let result = store.add_document("test-index", valid_doc);

        assert!(result.is_ok());
        assert_eq!(store.get_index("test-index").unwrap().documents.len(), 1);
    }

    #[test]
    fn should_accept_extra_fields_on_default_mapping() {
        let store = InMemoryStore::new();
        store.create_index(".migrations".to_string(), Mapping::default());

        let doc = json!({
            "filename": "0001_init.json",
            "executed_at": "2026-02-27T12:00:00Z"
        });

        let result = store.add_document(".migrations", doc);
        assert!(result.is_ok());
    }

    #[test]
    fn should_get_and_delete_document_by_id() {
        let store = InMemoryStore::new();
        store.create_index("test".to_string(), Mapping::default());

        let id = store.add_document("test", json!({"name": "doc1"})).unwrap();

        assert!(store.get_document("test", &id).is_some());
        assert!(store.delete_document("test", &id));
        assert!(store.get_document("test", &id).is_none());
    }

    #[test]
    fn should_update_existing_document_with_same_id() {
        let store = InMemoryStore::new();
        store.create_index("test".to_string(), Mapping::default());

        let doc = json!({"_id": "1", "val": "old"});
        store.add_document("test", doc).unwrap();

        let new_doc = json!({"_id": "1", "val": "new"});
        store.add_document("test", new_doc).unwrap();

        let stored = store.get_document("test", "1").unwrap();
        assert_eq!(stored["val"], "new");
        assert_eq!(store.get_index("test").unwrap().documents.len(), 1);
    }

    #[test]
    fn should_partially_update_document() {
        let store = InMemoryStore::new();
        store.create_index("test".to_string(), Mapping::default());

        store
            .add_document("test", json!({"_id": "1", "a": 1, "b": 2}))
            .unwrap();
        store
            .patch_document("test", "1", json!({"b": 3, "c": 4}))
            .unwrap();

        let doc = store.get_document("test", "1").unwrap();
        assert_eq!(doc["a"], 1);
        assert_eq!(doc["b"], 3);
        assert_eq!(doc["c"], 4);
    }
}