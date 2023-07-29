use serde::{Deserialize, Serialize};
use serde_json::{from_str, to_string};
use std::error::Error;

// Define your data structures here if needed (corresponding to deserialise/serialise in TypeScript)

#[derive(Debug, Deserialize, Serialize)]
struct Data {
    // Define your fields here (matching the expected response from the API)
}

// NormalizeResponse function
fn normalize_response(data: Option<&str>) -> Option<Data> {
    if let Some(json_str) = data {
        let new_json_str = if let Ok(parsed_data) = from_str::<serde_json::Value>(json_str) {
            if parsed_data.get("included").is_none() {
                let mut new_data = serde_json::Map::new();
                new_data.insert("included".to_string(), serde_json::Value::Array(vec![]));
                let normalized_json =
                    serde_json::json!({ "data": parsed_data, "included": vec![] });
                to_string(&normalized_json).unwrap()
            } else {
                json_str.to_string()
            }
        } else {
            json_str.to_string()
        };
        if let Ok(normalized_data) = from_str::<Data>(&new_json_str) {
            return Some(normalized_data);
        }
    }
    None
}

// NormalizeRequest function
fn normalize_request(data: &Data) -> String {
    // Implement your normalization logic here (corresponding to TypeScript's normalizeRequest)
    // For example:
    // let serialized_data = to_string(&data).unwrap();
    // let deserialized_data: serde_json::Value = from_str(&serialized_data).unwrap();
    // let mut serialized_data_map = deserialized_data.as_object().unwrap().clone();
    // serialized_data_map.remove("type");
    // to_string(&serialized_data_map).unwrap()
    // (Note: Replace `Data` with the appropriate data structure if needed)

    // Placeholder for this example
    to_string(data).unwrap()
}

// RetryOptions struct
struct RetryOptions {
    retry: u32,
    validate_retryable: Option<Box<dyn Fn(&reqwest::Error) -> bool>>,
    on_retry: Option<Box<dyn Fn(&reqwest::Error, u32) -> ()>>,
}

// APIClient struct
struct APIClient {
    api: reqwest::Client,
    retry_options: RetryOptions,
}

impl APIClient {
    fn new(options: reqwest::ClientBuilder, retry_options: RetryOptions) -> Self {
        let api = options.build().unwrap();
        Self { api, retry_options }
    }

    // Get function
    async fn get<T: for<'de> Deserialize<'de>>(
        &self,
        url: &str,
        options: Option<reqwest::RequestBuilder>,
    ) -> Result<T, Box<dyn Error>> {
        let response = self.wrap_request(self.api.get(url), options).await?;
        if let Some(json_str) = response.text().await.unwrap().as_str() {
            if let Some(data) = normalize_response(Some(json_str)) {
                return Ok(data);
            }
        }
        Err("Error deserializing data".into())
    }

    // Post function
    async fn post<T: for<'de> Deserialize<'de>>(
        &self,
        url: &str,
        json: &Data,
        options: Option<reqwest::RequestBuilder>,
    ) -> Result<T, Box<dyn Error>> {
        let normalized_json = normalize_request(json);
        let response = self
            .wrap_request(self.api.post(url).json(&normalized_json), options)
            .await?;
        if let Some(json_str) = response.text().await.unwrap().as_str() {
            if let Some(data) = normalize_response(Some(json_str)) {
                return Ok(data);
            }
        }
        Err("Error deserializing data".into())
    }

    // Put function
    async fn put<T: for<'de> Deserialize<'de>>(
        &self,
        url: &str,
        json: &Data,
        options: Option<reqwest::RequestBuilder>,
    ) -> Result<T, Box<dyn Error>> {
        let normalized_json = normalize_request(json);
        let response = self
            .wrap_request(self.api.put(url).json(&normalized_json), options)
            .await?;
        if let Some(json_str) = response.text().await.unwrap().as_str() {
            if let Some(data) = normalize_response(Some(json_str)) {
                return Ok(data);
            }
        }
        Err("Error deserializing data".into())
    }

    // Delete function
    async fn delete(
        &self,
        url: &str,
        options: Option<reqwest::RequestBuilder>,
    ) -> Result<(), Box<dyn Error>> {
        self.wrap_request(self.api.delete(url), options).await?;
        Ok(())
    }

    // WrapRequest function
    async fn wrap_request(
        &self,
        request: reqwest::RequestBuilder,
        options: Option<reqwest::RequestBuilder>,
    ) -> Result<reqwest::Response, Box<dyn Error>> {
        let retry = self.retry_options.retry;
        let validate_retryable = self.retry_options.validate_retryable.as_ref().cloned();
        let on_retry = self.retry_options.on_retry.as_ref().cloned();
        let retryable_status_codes = vec![408, 413, 429, 500, 502, 503, 504];

        retry(retry, |retry_count| async {
            let mut response = request.try_clone().unwrap().send().await?;

            if retry_count == retry {
                // On last retry, always bail on any error
                response.error_for_status_ref()?;
            } else if let Some(status) = response.status().as_u16() {
                if retryable_status_codes.contains(&status) {
                    return Err(response.error_for_status_ref().err().unwrap());
                }
            }

            Ok(response)
        })
        .await
    }
}
