use std::error::Error;

use reqwest_middleware::{ClientBuilder, ClientWithMiddleware, RequestBuilder};
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
use serde::Deserialize;
use serde_json::{from_str, to_string};

fn normalize_response<T: for<'de> Deserialize<'de>>(data: Option<&str>) -> Option<T> {
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
        if let Ok(normalized_data) = from_str::<T>(&new_json_str) {
            return Some(normalized_data);
        }
    }
    None
}

fn normalize_request<T: for<'de> Deserialize<'de>>(data: &T) -> String {
    // Implement your normalization logic here (corresponding to TypeScript's normalizeRequest)
    // For example:
    let serialized_data = to_string(&data).unwrap();
    let deserialized_data: serde_json::Value = from_str(&serialized_data).unwrap();
    let mut serialized_data_map = deserialized_data.as_object().unwrap().clone();
    serialized_data_map.remove("type");
    to_string(&serialized_data_map).unwrap()
    // (Note: Replace `Data` with the appropriate data structure if needed)

    // Placeholder for this example
    // to_string(data).unwrap()
}

struct RetryOptions {
    retry: u32,
}

struct APIClient {
    api: ClientWithMiddleware,
    retry_options: RetryOptions,
}

impl APIClient {
    fn new(retry_options: RetryOptions) -> Self {
        let retry_policy =
            ExponentialBackoff::builder().build_with_max_retries(retry_options.retry);
        let api = ClientBuilder::new(reqwest::Client::new())
            .with(RetryTransientMiddleware::new_with_policy(retry_policy))
            .build();
        Self { api, retry_options }
    }

    async fn get<T: for<'de> Deserialize<'de>>(&self, url: &str) -> Result<T, Box<dyn Error>> {
        let response = self.wrap_request(self.api.get(url)).await?;
        if let json_str = response.text().await.unwrap().as_str() {
            if let Some(data) = normalize_response(Some(json_str)) {
                return Ok(data);
            }
        }
        Err("Error deserializing data".into())
    }

    async fn post<T: for<'de> Deserialize<'de>>(
        &self,
        url: &str,
        json: &Data,
    ) -> Result<T, Box<dyn Error>> {
        let normalized_json = normalize_request(json);
        let response = self
            .wrap_request(self.api.post(url).json(&normalized_json))
            .await?;
        if let json_str = response.text().await.unwrap().as_str() {
            if let Some(data) = normalize_response(Some(json_str)) {
                return Ok(data);
            }
        }
        Err("Error deserializing data".into())
    }

    async fn put<T: for<'de> Deserialize<'de>>(
        &self,
        url: &str,
        json: &T,
    ) -> Result<T, Box<dyn Error>> {
        let normalized_json = normalize_request(json);
        let response = self
            .wrap_request(self.api.put(url).json(&normalized_json))
            .await?;
        if let json_str = response.text().await.unwrap().as_str() {
            if let Some(data) = normalize_response(Some(json_str)) {
                return Ok(data);
            }
        }
        Err("Error deserializing data".into())
    }

    async fn delete(&self, url: &str) -> Result<(), Box<dyn Error>> {
        self.wrap_request(self.api.delete(url)).await?;
        Ok(())
    }

    async fn wrap_request(
        &self,
        request: RequestBuilder,
    ) -> Result<reqwest::Response, Box<dyn Error>> {
        let response = request.send().await.unwrap();

        Ok(response)
    }
}
