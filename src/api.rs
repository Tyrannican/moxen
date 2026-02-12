use crate::addon::Addon;
use anyhow::{Context, Result};
use reqwest::Client;

const BASE_URL: &str = "https://api.curseforge.com";

pub struct CurseClient {
    api: Client,
    key: String,
}

impl CurseClient {
    pub fn new(api_key: &str) -> Self {
        Self {
            api: Client::new(),
            key: api_key.to_string(),
        }
    }

    pub async fn get_addon(&self, addon_id: i32) -> Result<Addon> {
        let url = format!("{BASE_URL}/v1/mods/{addon_id}");
        self.api
            .get(&url)
            .header("Accept", "application/json")
            .header("x-api-key", &self.key)
            .send()
            .await
            .context("calling api")?
            .json::<Addon>()
            .await
            .context("converting to text")
    }

    pub async fn download_addon(&self, addon: &Addon) -> Result<Vec<u8>> {
        let url = if let Some(ref url) = addon.main_file.download_url {
            url
        } else {
            &format!(
                "https://edge.forgecdn.net/files/{}/{}/{}",
                addon.main_file.id / 1000,
                addon.main_file.id % 1000,
                addon.main_file.file_name
            )
        };

        let content = self
            .api
            .get(url)
            .send()
            .await
            .with_context(|| format!("calling download url for {}: {}", addon.name, url))?
            .bytes()
            .await
            .context("getting content bytes")?
            .to_vec();

        Ok(content)
    }
}
