use super::*;

impl DeezerClient {
    pub async fn new(arl: &str) -> Result<Self, String> {
        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
            ),
        );
        headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("en-US,en;q=0.9"));
        headers.insert(ACCEPT, HeaderValue::from_static("*/*"));
        headers.insert(CACHE_CONTROL, HeaderValue::from_static("max-age=0"));
        headers.insert(CONNECTION, HeaderValue::from_static("keep-alive"));

        let jar = Arc::new(Jar::default());
        let deezer_url = "https://www.deezer.com".parse().unwrap();
        jar.add_cookie_str(&format!("arl={}; Domain=.deezer.com; Path=/", arl), &deezer_url);

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .cookie_provider(jar)
            .redirect(reqwest::redirect::Policy::custom(|attempt| {
                if attempt.previous().len() >= 10 {
                    return attempt.error("too many Deezer redirects");
                }
                if is_allowed_deezer_url(attempt.url()) {
                    attempt.follow()
                } else {
                    attempt.error("redirected to a non-Deezer host")
                }
            }))
            .min_tls_version(reqwest::tls::Version::TLS_1_2)
            .https_only(true)
            .connect_timeout(Duration::from_secs(10))
            .read_timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| e.to_string())?;

        let mut client = Self {
            http,
            arl: arl.to_string(),
            token: String::new(),
            license_token: None,
            user: None,
        };

        client.login().await?;
        Ok(client)
    }

    async fn login(&mut self) -> Result<(), String> {
        // Make initial request to establish session and get SID cookie
        let _ = self.http
            .get(API_URL)
            .send()
            .await
            .map_err(|e: reqwest::Error| format!("Failed to get SID: {}", e))?;
        
        let data = self.api_call("deezer.getUserData", None).await?;
        let results = &data["results"];

        self.token = results["checkForm"]
            .as_str()
            .ok_or("Failed to get auth token from Deezer")?
            .to_string();

        self.license_token = results
            .get("USER")
            .and_then(|u| u.get("OPTIONS"))
            .and_then(|o| o.get("license_token"))
            .and_then(|t| t.as_str())
            .map(|s| s.to_string());

        let user_id = results["USER"]["USER_ID"]
            .as_u64()
            .or_else(|| {
                results["USER"]["USER_ID"]
                    .as_str()
                    .and_then(|s| s.parse().ok())
            })
            .ok_or("Invalid ARL token")?;

        if user_id == 0 {
            return Err("Invalid ARL token".into());
        }

        let name = results["USER"]["BLOG_NAME"]
            .as_str()
            .unwrap_or("Unknown")
            .to_string();

        let picture = results["USER"]["USER_PICTURE"]
            .as_str()
            .unwrap_or("");

        // Deezer uses a 32-char MD5 hash for USER_PICTURE.
        // An empty or all-zeros hash means the user has no custom picture.
        // Return None so the frontend shows the fallback avatar icon.
        let image = if picture.is_empty() || picture.chars().all(|c| c == '0') {
            None
        } else {
            Some(format!(
                "https://e-cdns-images.dzcdn.net/images/user/{}/250x250-000000-80-0-0.jpg",
                picture
            ))
        };

        let offer_name = results["OFFER_NAME"]
            .as_str()
            .or_else(|| results["USER"]["OFFER_NAME"].as_str())
            .or_else(|| results["USER"]["OPTIONS"]["offer_name"].as_str())
            .unwrap_or("")
            .to_lowercase();
        let has_ads = results["USER"]["OPTIONS"]["ads_audio"].as_bool().unwrap_or(false)
            || results["USER"]["OPTIONS"]["ads_display"].as_bool().unwrap_or(false);
        let is_free_account = offer_name.contains("free")
            || offer_name.contains("gratuit")
            || offer_name.contains("kostenlos")
            || (offer_name.is_empty() && has_ads);

        self.user = Some(UserInfo {
            id: user_id,
            name,
            image,
            is_free_account,
        });

        Ok(())
    }
}
