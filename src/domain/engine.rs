use crate::domain::query::{Query, TermsAggregation};
use serde_json::Value;
use std::cmp::Ordering;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum SortOrder {
    Asc,
    Desc,
}

#[derive(Debug, Clone)]
pub struct SortOptions {
    pub field: String,
    pub order: SortOrder,
}

#[derive(Debug, Clone)]
pub struct AggregationResult {
    pub name: String,
    pub buckets: Vec<Bucket>,
}

#[derive(Debug, Clone)]
pub struct Bucket {
    pub key: Value,
    pub doc_count: usize,
}

pub struct SearchEngine;

impl SearchEngine {
    pub fn search(
        documents: &[Value],
        query: &dyn Query,
        sort: Option<SortOptions>,
        from: usize,
        size: usize,
    ) -> Vec<Value> {
        let mut results: Vec<Value> = documents
            .iter()
            .filter(|doc| query.matches(doc))
            .cloned()
            .collect();

        if let Some(options) = sort {
            let field_name = options.field.strip_suffix(".keyword").unwrap_or(&options.field);
            
            results.sort_by(|a, b| {
                let val_a = a.get(field_name);
                let val_b = b.get(field_name);

                let cmp = match (val_a, val_b) {
                    (Some(v1), Some(v2)) => Self::compare_values(v1, v2),
                    (Some(_), None) => Ordering::Greater,
                    (None, Some(_)) => Ordering::Less,
                    (None, None) => Ordering::Equal,
                };

                match options.order {
                    SortOrder::Asc => cmp,
                    SortOrder::Desc => cmp.reverse(),
                }
            });
        }

        results.into_iter().skip(from).take(size).collect()
    }

    pub fn aggregate(
        filtered_documents: &[Value],
        aggregations: &[TermsAggregation],
    ) -> Vec<AggregationResult> {
        let mut results = Vec::new();

        for agg in aggregations {
            let field_name = agg.field.strip_suffix(".keyword").unwrap_or(&agg.field);
            let mut counts: HashMap<String, (Value, usize)> = HashMap::new();

            for doc in filtered_documents {
                if let Some(val) = doc.get(field_name) {
                    let key_str = match val {
                        Value::String(s) => s.clone(),
                        Value::Number(n) => n.to_string(),
                        Value::Bool(b) => b.to_string(),
                        _ => continue,
                    };

                    let entry = counts.entry(key_str).or_insert((val.clone(), 0));
                    entry.1 += 1;
                }
            }

            let mut buckets: Vec<Bucket> = counts
                .into_values()
                .map(|(key, doc_count)| Bucket { key, doc_count })
                .collect();

            buckets.sort_by(|a, b| b.doc_count.cmp(&a.doc_count).then_with(|| {
                let key_a = a.key.as_str().unwrap_or("");
                let key_b = b.key.as_str().unwrap_or("");
                key_a.cmp(key_b)
            }));

            results.push(AggregationResult {
                name: agg.name.clone(),
                buckets,
            });
        }

        results
    }

    fn compare_values(a: &Value, b: &Value) -> Ordering {
        if let (Some(f1), Some(f2)) = (a.as_f64(), b.as_f64()) {
            return f1.partial_cmp(&f2).unwrap_or(Ordering::Equal);
        }
        if let (Some(s1), Some(s2)) = (a.as_str(), b.as_str()) {
            return s1.cmp(s2);
        }
        if let (Some(b1), Some(b2)) = (a.as_bool(), b.as_bool()) {
            return b1.cmp(&b2);
        }
        Ordering::Equal
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::query::MatchAllQuery;
    use serde_json::json;

    #[derive(Debug)]
    struct MockKeywordQuery {
        field: String,
        value: Value,
    }

    impl Query for MockKeywordQuery {
        fn matches(&self, doc: &Value) -> bool {
            let field_name = self.field.strip_suffix(".keyword").unwrap_or(&self.field);
            doc.get(field_name) == Some(&self.value)
        }
    }

    #[test]
    fn should_sort_documents_ascending() {
        let docs = vec![
            json!({"id": 2, "val": 20}),
            json!({"id": 1, "val": 10}),
            json!({"id": 3, "val": 30}),
        ];
        let sort = Some(SortOptions {
            field: "val".to_string(),
            order: SortOrder::Asc,
        });

        let results = SearchEngine::search(&docs, &MatchAllQuery, sort, 0, 10);
        
        assert_eq!(results[0]["id"], 1);
        assert_eq!(results[2]["id"], 3);
    }

    #[test]
    fn should_sort_documents_descending() {
        let docs = vec![
            json!({"id": 1, "val": 10}),
            json!({"id": 2, "val": 20}),
        ];
        let sort = Some(SortOptions {
            field: "val".to_string(),
            order: SortOrder::Desc,
        });

        let results = SearchEngine::search(&docs, &MatchAllQuery, sort, 0, 10);
        
        assert_eq!(results[0]["id"], 2);
    }

    #[test]
    fn should_handle_keyword_suffix_in_sort() {
        let docs = vec![
            json!({"name": "B"}),
            json!({"name": "A"}),
        ];
        let sort = Some(SortOptions {
            field: "name.keyword".to_string(),
            order: SortOrder::Asc,
        });

        let results = SearchEngine::search(&docs, &MatchAllQuery, sort, 0, 10);
        assert_eq!(results[0]["name"], "A");
    }

    #[test]
    fn should_handle_keyword_suffix_in_query() {
        let docs = vec![json!({"filename": "test.json"})];
        let query = MockKeywordQuery {
            field: "filename.keyword".to_string(),
            value: json!("test.json"),
        };

        let results = SearchEngine::search(&docs, &query, None, 0, 10);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn should_apply_pagination_from_and_size() {
        let docs = vec![
            json!({"id": 1}),
            json!({"id": 2}),
            json!({"id": 3}),
            json!({"id": 4}),
        ];

        let results = SearchEngine::search(&docs, &MatchAllQuery, None, 1, 2);
        
        assert_eq!(results.len(), 2);
        assert_eq!(results[0]["id"], 2);
        assert_eq!(results[1]["id"], 3);
    }

    #[test]
    fn should_aggregate_terms_correctly() {
        let docs = vec![
            json!({"color": "red"}),
            json!({"color": "blue"}),
            json!({"color": "red"}),
            json!({"color": "green"}),
        ];
        let aggs = vec![TermsAggregation {
            name: "colors".to_string(),
            field: "color.keyword".to_string(),
        }];

        let results = SearchEngine::aggregate(&docs, &aggs);
        
        assert_eq!(results.len(), 1);
        let agg_res = &results[0];
        assert_eq!(agg_res.name, "colors");
        
        let red_bucket = agg_res.buckets.iter().find(|b| b.key == json!("red")).unwrap();
        assert_eq!(red_bucket.doc_count, 2);
        
        let blue_bucket = agg_res.buckets.iter().find(|b| b.key == json!("blue")).unwrap();
        assert_eq!(blue_bucket.doc_count, 1);
    }
}