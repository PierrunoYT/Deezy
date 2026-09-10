use super::*;

impl DeezerClient {
    pub async fn api_call(
        &self,
        method: &str,
        params: Option<Value>,
    ) -> Result<Value, String> {
        let token = if method == "deezer.getUserData" {
            "null".to_string()
        } else {
            self.token.clone()
        };

        let body = params.unwrap_or(serde_json::json!({}));

        let res = self
            .http
            .post(API_URL)
            .query(&[
                ("api_version", "1.0"),
                ("api_token", &token),
                ("input", "3"),
                ("method", method),
            ])
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("API call failed: {}", e.without_url()))?;

        let data: Value = response_json(res)
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        if let Some(error) = data.get("error") {
            if let Some(obj) = error.as_object() {
                if !obj.is_empty() {
                    let msg = obj
                        .values()
                        .next()
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown error");
                    return Err(format!("Deezer error: {}", msg));
                }
            }
        }

        Ok(data)
    }
}
