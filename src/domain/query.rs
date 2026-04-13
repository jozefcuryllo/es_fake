use crate::domain::engine::{SortOptions, SortOrder};
use serde_json::Value;
use std::fmt::Debug;

pub trait Query: Debug + Send + Sync {
    fn matches(&self, doc: &Value) -> bool;
}

#[derive(Debug)]
pub struct MatchAllQuery;

impl Query for MatchAllQuery {
    fn matches(&self, _doc: &Value) -> bool {
        true
    }
}

#[derive(Debug)]
pub struct TermQuery {
    pub field: String,
    pub value: Value,
}

impl Query for TermQuery {
    fn matches(&self, doc: &Value) -> bool {
        let field_path = self.field.strip_suffix(".keyword").unwrap_or(&self.field);
        Self::matches_path(doc, field_path, &self.value)
    }
}

#[derive(Debug)]
pub struct BoolQuery {
    pub must: Vec<Box<dyn Query>>,
    pub should: Vec<Box<dyn Query>>,
    pub must_not: Vec<Box<dyn Query>>,
}

impl Query for BoolQuery {
    fn matches(&self, doc: &Value) -> bool {
        let must_matches = self.must.iter().all(|q| q.matches(doc));
        let must_not_matches = self.must_not.iter().all(|q| !q.matches(doc));

        if !must_matches || !must_not_matches {
            return false;
        }

        if self.should.is_empty() {
            return true;
        }

        self.should.iter().any(|q| q.matches(doc))
    }
}

impl TermQuery {
    fn matches_path(current_value: &Value, path: &str, target: &Value) -> bool {
        if path.is_empty() {
            return current_value == target;
        }

        let mut parts = path.splitn(2, '.');
        let current_key = parts.next().unwrap();
        let remaining_path = parts.next().unwrap_or("");

        match current_value {
            Value::Object(map) => {
                if let Some(next_val) = map.get(current_key) {
                    return Self::matches_path(next_val, remaining_path, target);
                }
                false
            }
            Value::Array(arr) => arr.iter().any(|item| {
                if let Some(obj) = item.as_object() {
                    if let Some(next_val) = obj.get(current_key) {
                        return Self::matches_path(next_val, remaining_path, target);
                    }
                }
                Self::matches_path(item, path, target)
            }),
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TermsAggregation {
    pub name: String,
    pub field: String,
}

pub fn parse_query(json: &Value) -> Box<dyn Query> {
    if let Some(query_obj) = json.get("query") {
        return parse_query_internal(query_obj);
    }
    Box::new(MatchAllQuery)
}

pub fn parse_aggregations(json: &Value) -> Vec<TermsAggregation> {
    let mut aggregations = Vec::new();
    let aggs_node = json.get("aggs").or_else(|| json.get("aggregations"));

    if let Some(aggs_obj) = aggs_node.and_then(|v| v.as_object()) {
        for (name, body) in aggs_obj {
            if let Some(terms) = body
                .get("terms")
                .and_then(|t| t.get("field"))
                .and_then(|f| f.as_str())
            {
                aggregations.push(TermsAggregation {
                    name: name.clone(),
                    field: terms.to_string(),
                });
            }
        }
    }
    aggregations
}

pub fn parse_pagination(json: &Value) -> (usize, usize) {
    let from = json.get("from").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
    let size = json.get("size").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
    (from, size)
}

fn parse_query_internal(json: &Value) -> Box<dyn Query> {
    if let Some(bool_obj) = json.get("bool") {
        return Box::new(parse_bool(bool_obj));
    }
    if let Some(term_obj) = json.get("term") {
        if let Some((field, value)) = term_obj.as_object().and_then(|o| o.iter().next()) {
            return Box::new(TermQuery {
                field: field.clone(),
                value: value.clone(),
            });
        }
    }
    Box::new(MatchAllQuery)
}

fn parse_bool(json: &Value) -> BoolQuery {
    let mut must = Vec::new();
    let mut should = Vec::new();
    let mut must_not = Vec::new();

    if let Some(m) = json.get("must") {
        must = parse_list(m);
    }
    if let Some(s) = json.get("should") {
        should = parse_list(s);
    }
    if let Some(mn) = json.get("must_not") {
        must_not = parse_list(mn);
    }

    BoolQuery {
        must,
        should,
        must_not,
    }
}

fn parse_list(json: &Value) -> Vec<Box<dyn Query>> {
    match json {
        Value::Array(arr) => arr.iter().map(|v| parse_query_internal(v)).collect(),
        _ => vec![parse_query_internal(json)],
    }
}

pub fn parse_sort(json: &Value) -> Option<SortOptions> {
    let sort_value = json.get("sort")?;

    if let Some(arr) = sort_value.as_array() {
        if let Some(first) = arr.first() {
            return parse_single_sort(first);
        }
    } else {
        return parse_single_sort(sort_value);
    }

    None
}

fn parse_single_sort(json: &Value) -> Option<SortOptions> {
    if let Some(field) = json.as_str() {
        return Some(SortOptions {
            field: field.to_string(),
            order: SortOrder::Asc,
        });
    }

    if let Some(obj) = json.as_object() {
        if let Some((field, val)) = obj.iter().next() {
            let order = if val.get("order").and_then(|v| v.as_str()) == Some("desc") {
                SortOrder::Desc
            } else {
                SortOrder::Asc
            };
            return Some(SortOptions {
                field: field.clone(),
                order,
            });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn should_match_term_with_keyword_suffix() {
        let query = TermQuery {
            field: "status.keyword".to_string(),
            value: json!("active"),
        };
        let doc = json!({ "status": "active" });
        assert!(query.matches(&doc));
    }

    #[test]
    fn should_parse_simple_term_query() {
        let body = json!({
            "query": {
                "term": { "user_id": 1 }
            }
        });
        let query = parse_query(&body);
        let doc = json!({ "user_id": 1 });
        assert!(query.matches(&doc));
    }

    #[test]
    fn should_parse_bool_must_query() {
        let body = json!({
            "query": {
                "bool": {
                    "must": [
                        { "term": { "tags": "rust" } },
                        { "term": { "published": true } }
                    ]
                }
            }
        });
        let query = parse_query(&body);

        assert!(query.matches(&json!({ "tags": "rust", "published": true })));
        assert!(!query.matches(&json!({ "tags": "rust", "published": false })));
    }

    #[test]
    fn should_parse_bool_must_not_query() {
        let body = json!({
            "query": {
                "bool": {
                    "must_not": { "term": { "status": "deleted" } }
                }
            }
        });
        let query = parse_query(&body);

        assert!(query.matches(&json!({ "status": "active" })));
        assert!(!query.matches(&json!({ "status": "deleted" })));
    }

    #[test]
    fn should_parse_sort_string() {
        let body = json!({ "sort": ["created_at"] });
        let sort = parse_sort(&body).unwrap();
        assert_eq!(sort.field, "created_at");
        assert!(matches!(sort.order, SortOrder::Asc));
    }

    #[test]
    fn should_parse_sort_object_desc() {
        let body = json!({
            "sort": { "price": { "order": "desc" } }
        });
        let sort = parse_sort(&body).unwrap();
        assert_eq!(sort.field, "price");
        assert!(matches!(sort.order, SortOrder::Desc));
    }

    #[test]
    fn should_parse_pagination_parameters() {
        let body = json!({
            "from": 20,
            "size": 50
        });
        let (from, size) = parse_pagination(&body);
        assert_eq!(from, 20);
        assert_eq!(size, 50);
    }

    #[test]
    fn should_return_default_pagination_when_missing() {
        let body = json!({});
        let (from, size) = parse_pagination(&body);
        assert_eq!(from, 0);
        assert_eq!(size, 10);
    }

    #[test]
    fn should_parse_terms_aggregation() {
        let body = json!({
            "aggs": {
                "popular_colors": {
                    "terms": {
                        "field": "color.keyword"
                    }
                }
            }
        });
        let aggs = parse_aggregations(&body);
        assert_eq!(aggs.len(), 1);
        assert_eq!(aggs[0].name, "popular_colors");
        assert_eq!(aggs[0].field, "color.keyword");
    }

    #[test]
    fn should_match_nested_field_with_dot_notation() {
        let query = TermQuery {
            field: "brand.name".to_string(),
            value: json!("TestBrand"),
        };
        let doc = json!({
            "brand": {
                "name": "TestBrand"
            }
        });
        assert!(query.matches(&doc));
    }

    #[test]
    fn should_match_keyword_suffix_on_nested_field() {
        let query = TermQuery {
            field: "brand.name.keyword".to_string(),
            value: json!("TestBrand"),
        };
        let doc = json!({
            "brand": {
                "name": "TestBrand"
            }
        });
        assert!(query.matches(&doc));
    }

    #[test]
    fn should_match_inside_array_of_objects() {
        let query = TermQuery {
            field: "offers.url".to_string(),
            value: json!("https://test.com/1"),
        };
        let doc = json!({
            "offers": [
                { "url": "https://other.com", "price": 10 },
                { "url": "https://test.com/1", "price": 20 }
            ]
        });
        assert!(
            query.matches(&doc),
            "Powinien znaleźć URL wewnątrz tablicy obiektów"
        );
    }

    #[test]
    fn should_match_keyword_inside_array_of_objects() {
        let query = TermQuery {
            field: "offers.url.keyword".to_string(),
            value: json!("https://test.com/1"),
        };
        let doc = json!({
            "offers": [
                { "url": "https://test.com/1" }
            ]
        });
        assert!(
            query.matches(&doc),
            "Powinien obsłużyć .keyword wewnątrz tablicy"
        );
    }

    #[test]
    fn should_not_match_if_nested_value_differs() {
        let query = TermQuery {
            field: "brand.name".to_string(),
            value: json!("WrongBrand"),
        };
        let doc = json!({
            "brand": { "name": "RightBrand" }
        });
        assert!(!query.matches(&doc));
    }

    #[test]
    fn should_handle_deep_nesting() {
        let query = TermQuery {
            field: "a.b.c.d".to_string(),
            value: json!(42),
        };
        let doc = json!({
            "a": { "b": { "c": { "d": 42 } } }
        });
        assert!(query.matches(&doc));
    }
}
