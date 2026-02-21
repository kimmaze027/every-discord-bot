use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::de::DeserializeOwned;
use serde_json::Value;
use tokio::sync::RwLock;

use super::models::{GraphQLError, GraphQLResponse};

const API_URL: &str = "https://api.tarkov.dev/graphql";
const CACHE_TTL: Duration = Duration::from_secs(300); // 5 minutes

pub type Cache = Arc<RwLock<HashMap<String, (Instant, Value)>>>;

pub fn new_cache() -> Cache {
    Arc::new(RwLock::new(HashMap::new()))
}

/// Errors that can occur during a GraphQL query.
#[derive(Debug)]
pub enum QueryError {
    /// Network or HTTP-level error from reqwest.
    Network(reqwest::Error),
    /// The GraphQL API returned one or more errors in the response body.
    GraphQL(Vec<GraphQLError>),
    /// The response JSON could not be deserialized into the expected type.
    Deserialize(String),
    /// The response contained no `data` field and no `errors` field.
    EmptyResponse,
}

impl std::fmt::Display for QueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueryError::Network(e) => write!(f, "API 서버에 연결할 수 없습니다: {e}"),
            QueryError::GraphQL(errors) => {
                let msgs: Vec<&str> = errors.iter().map(|e| e.message.as_str()).collect();
                write!(f, "API 오류: {}", msgs.join(", "))
            }
            QueryError::Deserialize(msg) => {
                write!(f, "응답 처리 중 오류가 발생했습니다: {msg}")
            }
            QueryError::EmptyResponse => write!(f, "API로부터 빈 응답을 받았습니다"),
        }
    }
}

impl std::error::Error for QueryError {}

/// Execute a GraphQL query against the tarkov.dev API with caching.
///
/// - `client`: The shared reqwest client.
/// - `cache`: The shared cache.
/// - `query`: The GraphQL query string.
/// - `variables`: The variables to pass to the query.
///
/// Returns the deserialized `data` field of the GraphQL response.
/// On cache hit (within TTL), returns the cached value without making a network request.
/// Expired cache entries are pruned on cache miss.
pub async fn query<T: DeserializeOwned>(
    client: &reqwest::Client,
    cache: &Cache,
    query_str: &str,
    variables: &Value,
) -> Result<T, QueryError> {
    let cache_key = build_cache_key(query_str, variables);

    // Check cache
    {
        let cache_read = cache.read().await;
        if let Some((inserted_at, cached_value)) = cache_read.get(&cache_key) {
            if inserted_at.elapsed() < CACHE_TTL {
                let result: T = serde_json::from_value(cached_value.clone())
                    .map_err(|e| QueryError::Deserialize(e.to_string()))?;
                return Ok(result);
            }
        }
    }

    // Cache miss: execute query
    let body = serde_json::json!({
        "query": query_str,
        "variables": variables,
    });

    let response = client
        .post(API_URL)
        .json(&body)
        .send()
        .await
        .map_err(QueryError::Network)?;

    let response_text = response.text().await.map_err(QueryError::Network)?;

    let graphql_response: GraphQLResponse<Value> =
        serde_json::from_str(&response_text).map_err(|e| QueryError::Deserialize(e.to_string()))?;

    // Check for GraphQL errors
    if let Some(errors) = graphql_response.errors {
        if !errors.is_empty() {
            return Err(QueryError::GraphQL(errors));
        }
    }

    let data = graphql_response.data.ok_or(QueryError::EmptyResponse)?;

    // Store in cache and prune expired entries
    {
        let mut cache_write = cache.write().await;
        cache_write.insert(cache_key, (Instant::now(), data.clone()));

        // Prune expired entries to prevent memory growth
        cache_write.retain(|_, (inserted_at, _)| inserted_at.elapsed() < CACHE_TTL);
    }

    // Deserialize into target type
    serde_json::from_value(data).map_err(|e| QueryError::Deserialize(e.to_string()))
}

fn build_cache_key(query_str: &str, variables: &Value) -> String {
    format!("{}:{}", query_str.trim(), variables)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_cache_key() {
        let vars = serde_json::json!({"name": "LEDX", "lang": "ko"});
        let key = build_cache_key("query { items }", &vars);
        assert!(key.starts_with("query { items }:"));
        assert!(key.contains("LEDX"));
    }

    #[test]
    fn test_build_cache_key_deterministic() {
        let vars = serde_json::json!({"lang": "ko"});
        let key1 = build_cache_key("query { items }", &vars);
        let key2 = build_cache_key("query { items }", &vars);
        assert_eq!(key1, key2);
    }

    #[test]
    fn test_build_cache_key_different_variables() {
        let vars1 = serde_json::json!({"name": "LEDX"});
        let vars2 = serde_json::json!({"name": "GPU"});
        let key1 = build_cache_key("query { items }", &vars1);
        let key2 = build_cache_key("query { items }", &vars2);
        assert_ne!(key1, key2);
    }

    #[tokio::test]
    async fn test_cache_hit() {
        let cache = new_cache();
        let key = "test_query:{\"lang\":\"ko\"}".to_string();
        let value = serde_json::json!({"items": [{"name": "Test"}]});

        {
            let mut cache_write = cache.write().await;
            cache_write.insert(key.clone(), (Instant::now(), value.clone()));
        }

        // Verify cache contains the entry
        {
            let cache_read = cache.read().await;
            let (inserted_at, cached) = cache_read.get(&key).unwrap();
            assert!(inserted_at.elapsed() < CACHE_TTL);
            assert_eq!(cached, &value);
        }
    }

    #[tokio::test]
    async fn test_cache_miss_expired() {
        let cache = new_cache();
        let key = "expired_query:{}".to_string();
        let value = serde_json::json!({"items": []});

        {
            let mut cache_write = cache.write().await;
            // Insert with a timestamp far in the past
            let expired_time = Instant::now() - Duration::from_secs(600);
            cache_write.insert(key.clone(), (expired_time, value));
        }

        // Verify the entry is expired
        {
            let cache_read = cache.read().await;
            let (inserted_at, _) = cache_read.get(&key).unwrap();
            assert!(inserted_at.elapsed() >= CACHE_TTL);
        }
    }

    #[tokio::test]
    async fn test_new_cache_is_empty() {
        let cache = new_cache();
        let cache_read = cache.read().await;
        assert!(cache_read.is_empty());
    }

    #[test]
    fn test_query_error_display_network() {
        // Verify Display impl doesn't panic for each variant
        let err = QueryError::EmptyResponse;
        let msg = format!("{err}");
        assert!(msg.contains("빈 응답"));
    }

    #[test]
    fn test_query_error_display_graphql() {
        let err = QueryError::GraphQL(vec![GraphQLError {
            message: "field error".to_string(),
        }]);
        let msg = format!("{err}");
        assert!(msg.contains("field error"));
    }

    #[test]
    fn test_query_error_display_deserialize() {
        let err = QueryError::Deserialize("missing field".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("missing field"));
    }
}
