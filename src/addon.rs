use jiff::Timestamp;
use serde::{Deserialize, Serialize, de::Deserializer};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Addon {
    pub id: i32,
    pub status: i32,
    pub name: String,
    pub slug: String,
    pub summary: String,
    pub authors: Vec<AddonAuthor>,
    pub main_file: AddonFile,
    pub date_modified: Timestamp,
}

impl<'de> Deserialize<'de> for Addon {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum AddonWrapper {
            Api(ApiResponse),
            Disk(DiskAddonWrapper),
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct DiskAddonWrapper {
            id: i32,
            status: i32,
            name: String,
            slug: String,
            summary: String,
            authors: Vec<AddonAuthor>,
            main_file: AddonFile,
            date_modified: Timestamp,
        }

        #[derive(Deserialize)]
        struct ApiResponse {
            data: ApiAddonWrapper,
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct ApiAddonWrapper {
            id: i32,
            name: String,
            status: i32,
            slug: String,
            summary: String,
            authors: Vec<AddonAuthor>,
            main_file_id: i32,
            latest_files: Vec<AddonFile>,
            date_modified: Timestamp,
        }

        let wrapper = AddonWrapper::deserialize(deserializer)?;

        match wrapper {
            AddonWrapper::Disk(inner) => Ok(Self {
                id: inner.id,
                name: inner.name,
                status: inner.status,
                slug: inner.slug,
                summary: inner.summary,
                authors: inner.authors,
                main_file: inner.main_file,
                date_modified: inner.date_modified,
            }),
            AddonWrapper::Api(inner) => {
                let inner = inner.data;
                let main_file = inner
                    .latest_files
                    .into_iter()
                    .find(|m| m.id == inner.main_file_id)
                    .expect("if the main file id isn't here then something has went wrong");

                Ok(Self {
                    id: inner.id,
                    name: inner.name,
                    status: inner.status,
                    slug: inner.slug,
                    summary: inner.summary,
                    authors: inner.authors,
                    main_file,
                    date_modified: inner.date_modified,
                })
            }
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct AddonAuthor {
    pub id: i32,
    pub name: String,
    pub url: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddonFile {
    pub id: i32,
    pub mod_id: i32,
    pub is_available: bool,
    pub display_name: Option<String>,
    pub file_name: String,
    pub hashes: Vec<String>,
    pub file_date: Timestamp,
    pub download_url: Option<String>,
    pub game_versions: Vec<String>,
    pub modules: Vec<String>,
}

impl<'de> Deserialize<'de> for AddonFile {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum HashWrapper {
            Disk(Vec<String>),
            Api(Vec<ApiHashWrapper>),
        }

        #[derive(Deserialize)]
        #[serde(untagged)]
        enum ModuleWrapper {
            Disk(Vec<String>),
            Api(Vec<AddonModule>),
        }

        #[derive(Deserialize)]
        struct ApiHashWrapper {
            value: String,
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct AddonFileWrapper {
            id: i32,
            mod_id: i32,
            is_available: bool,
            display_name: Option<String>,
            file_name: String,
            hashes: HashWrapper,
            modules: ModuleWrapper,
            file_date: Timestamp,
            download_url: Option<String>,
            game_versions: Vec<String>,
        }

        let wrapper = AddonFileWrapper::deserialize(deserializer)?;

        let hashes = match wrapper.hashes {
            HashWrapper::Api(api_hash) => api_hash.into_iter().map(|hash| hash.value).collect(),
            HashWrapper::Disk(disk_hash) => disk_hash,
        };

        let modules = match wrapper.modules {
            ModuleWrapper::Api(api_modules) => api_modules.into_iter().map(|m| m.name).collect(),
            ModuleWrapper::Disk(disk_modules) => disk_modules,
        };

        Ok(Self {
            id: wrapper.id,
            mod_id: wrapper.mod_id,
            is_available: wrapper.is_available,
            display_name: wrapper.display_name,
            file_name: wrapper.file_name,
            hashes,
            modules,
            file_date: wrapper.file_date,
            download_url: wrapper.download_url,
            game_versions: wrapper.game_versions,
        })
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AddonModule {
    pub name: String,
    pub fingerprint: usize,
}
