//! Hugging Face account and metadata API. No credentials in public DTOs.
use serde::{Deserialize, Serialize};

use serde_json::{json, Value};

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Account {
    pub name: String,
    pub token: String,
    pub client_id: String,
}

#[derive(Clone, Default)]
pub struct State(
    pub Arc<Mutex<Account>>,
    pub Arc<std::sync::atomic::AtomicU64>,
);

impl State {
    pub fn load() -> Self {
        let a = crate::storage::load_optional(&path(), 65536)
            .ok()
            .flatten()
            .unwrap_or_default();

        Self(Arc::new(Mutex::new(a)), Arc::default())
    }

    pub fn token(&self) -> String {
        self.0.lock().map(|a| a.token.clone()).unwrap_or_default()
    }

    pub fn save(&self, a: Account) -> Result<(), String> {
        self.save_at(a, None)
    }

    fn save_at(&self, a: Account, generation: Option<u64>) -> Result<(), String> {
        let mut current = self.0.lock().map_err(|e| e.to_string())?;

        if let Some(g) = generation {
            if self.1.load(std::sync::atomic::Ordering::SeqCst) != g {
                return Err("Sign-in cancelled or superseded by another account request".into());
            }
        }

        crate::storage::write_private_atomic(
            &path(),
            &serde_json::to_vec(&a).map_err(|e| e.to_string())?,
        )?;

        if generation.is_none() {
            self.1.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }

        *current = a;

        Ok(())
    }
}

fn path() -> std::path::PathBuf {
    crate::storage::profile_directory().join("hugging-face.bin")
}

pub fn client() -> Result<reqwest::Client, String> {
    static CLIENT: std::sync::OnceLock<Result<reqwest::Client, String>> =
        std::sync::OnceLock::new();
    CLIENT
        .get_or_init(|| {
            reqwest::Client::builder()
                .connect_timeout(Duration::from_secs(10))
                .timeout(Duration::from_secs(30))
                .redirect(reqwest::redirect::Policy::none())
                .user_agent("LLamaLauncher/1.0")
                .build()
                .map_err(|_| "Cannot create network client".into())
        })
        .clone()
}

pub async fn decode(response: reqwest::Response) -> Result<Value, String> {
    let code = response.status().as_u16();

    if code == 401 {
        return Err("Hugging Face authentication expired or invalid. Sign in again.".into());
    }

    if code == 403 {
        return Err(
            "Access denied. Open the model page to accept its license or request gated access."
                .into(),
        );
    }

    if code == 429 {
        return Err(format!(
            "Hugging Face rate limit. Retry after {} seconds.",
            response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("60")
        ));
    }

    if !(200..300).contains(&code) {
        return Err(format!("Hugging Face returned HTTP {code}"));
    }

    let mut response = response;

    let mut bytes = Vec::new();

    while let Some(c) = response
        .chunk()
        .await
        .map_err(|_| "Hugging Face response interrupted")?
    {
        if bytes.len() + c.len() > 8 * 1024 * 1024 {
            return Err("Hub metadata exceeds 8 MiB".into());
        }

        bytes.extend(c);
    }

    serde_json::from_slice(&bytes).map_err(|_| "Invalid Hugging Face response".into())
}

pub async fn api(state: &State, path: &str, query: &[(&str, &str)]) -> Result<Value, String> {
    let url = format!("https://huggingface.co{path}");

    let mut request = client()?.get(url).query(query);

    let token = state.token();

    if !token.is_empty() {
        request = request.bearer_auth(token);
    }

    decode(
        request
            .send()
            .await
            .map_err(|_| "Cannot reach Hugging Face")?,
    )
    .await
}

pub fn segments(value: &str) -> Result<Vec<&str>, String> {
    let parts: Vec<_> = value.split('/').collect();

    if parts.is_empty()
        || parts.iter().any(|p| {
            p.is_empty()
                || *p == "."
                || *p == ".."
                || p.contains(['\\', ':', '\0'])
                || p.chars().any(char::is_control)
        })
    {
        return Err("Invalid repository or file path".into());
    }

    Ok(parts)
}

pub fn url_path(parts: &[&str]) -> Result<String, String> {
    let mut url = reqwest::Url::parse("https://huggingface.co").map_err(|_| "Invalid Hub URL")?;

    url.path_segments_mut()
        .map_err(|_| "Invalid URL")?
        .extend(parts);

    Ok(url.path().into())
}

pub async fn connect(state: &State, token: String) -> Result<Value, String> {
    connect_at(state, token, None, None).await
}

async fn connect_at(
    state: &State,
    token: String,
    client_id_override: Option<String>,
    generation: Option<u64>,
) -> Result<Value, String> {
    if token.is_empty() || token.len() > 8192 || token.contains(['\r', '\n']) {
        return Err("Enter a valid read token".into());
    }

    let generation =
        generation.unwrap_or_else(|| state.1.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1);

    let v = decode(
        client()?
            .get("https://huggingface.co/api/whoami-v2")
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|_| "Cannot test Hugging Face account")?,
    )
    .await?;

    let name = v["name"].as_str().ok_or("Missing account name")?.to_owned();

    let client_id = match client_id_override {
        Some(id) => id,
        None => state.0.lock().map_err(|e| e.to_string())?.client_id.clone(),
    };

    state.save_at(
        Account {
            name: name.clone(),
            token,
            client_id,
        },
        Some(generation),
    )?;

    Ok(json!({
    "connected":true,"name":name}
    ))
}

pub async fn search(state: &State, search: &str, cursor: &str) -> Result<Value, String> {
    if search.len() > 256 {
        return Err("Search too long".into());
    }

    let mut url =
        reqwest::Url::parse("https://huggingface.co/api/models").map_err(|_| "Invalid Hub URL")?;

    url.query_pairs_mut()
        .append_pair("filter", "gguf")
        .append_pair("search", search)
        .append_pair("limit", "30")
        .append_pair("full", "true")
        .append_pair("sort", "downloads")
        .append_pair("direction", "-1");

    if !cursor.is_empty() {
        url.query_pairs_mut().append_pair("cursor", cursor);
    }

    let mut r = client()?.get(url);

    let token = state.token();

    if !token.is_empty() {
        r = r.bearer_auth(token);
    }

    let response = r.send().await.map_err(|_| "Model search failed")?;

    let next = response
        .headers()
        .get("link")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').find(|p| p.contains("rel=\"next\"")))
        .and_then(|v| v.split('<').nth(1))
        .and_then(|v| v.split('>').next())
        .and_then(|u| reqwest::Url::parse(u).ok())
        .and_then(|u| {
            u.query_pairs()
                .find(|(k, _)| k == "cursor")
                .map(|(_, v)| v.to_string())
        })
        .unwrap_or_default();

    let models = decode(response).await?;

    let results: Vec<_> = models
        .as_array()
        .ok_or("Invalid model search")?
        .iter()
        .filter(|m| {
            matches!(
                m["pipeline_tag"].as_str(),
                Some("text-generation" | "image-text-to-text" | "text2text-generation")
            ) || m["pipeline_tag"].is_null()
        })
        .cloned()
        .collect();

    Ok(json!({
    "models":results,"cursor":next}
    ))
}

pub async fn details(state: &State, repo: &str) -> Result<Value, String> {
    let parts = segments(repo)?;

    if parts.len() != 2 {
        return Err("Use an owner/repository ID".into());
    }

    let mut p = vec!["api", "models"];

    p.extend(parts);

    let v = api(state, &url_path(&p)?, &[("blobs", "true")]).await?;

    let sha = v["sha"].as_str().ok_or("Repository has no commit")?;

    let files: Vec<_> = v["siblings"]
        .as_array()
        .ok_or("Repository has no files")?
        .iter()
        .filter(|f| {
            f["rfilename"]
                .as_str()
                .is_some_and(|n| n.ends_with(".gguf"))
        })
        .cloned()
        .collect();

    Ok(json!({
    "id":repo,"sha":sha,"files":files,"card":v["cardData"],"gated":v["gated"],"pipeline":v["pipeline_tag"],"model_page":format!("https://huggingface.co/{repo}")}
    ))
}
pub async fn read_card(state: &State, repo: &str, revision: &str) -> Result<String, String> {
    if revision.len() != 40 || !revision.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("Invalid model revision".into());
    }
    let mut parts = segments(repo)?;
    parts.extend(["resolve", revision, "README.md"]);
    let url = format!("https://huggingface.co{}", url_path(&parts)?);
    let mut request = client()?.get(url);
    let token = state.token();
    if !token.is_empty() {
        request = request.bearer_auth(token);
    }
    let mut response = request.send().await.map_err(|_| "Cannot read model card")?;
    if !response.status().is_success() {
        return Ok(String::new());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "Model card interrupted")?
    {
        if bytes.len() + chunk.len() > 512 * 1024 {
            return Err("Model card exceeds 512 KiB".into());
        }
        bytes.extend(chunk);
    }
    String::from_utf8(bytes).map_err(|_| "Invalid model card text".into())
}

pub async fn oauth(
    app: tauri::AppHandle,
    state: State,
    jobs: crate::jobs::State,
    client_id: String,
) -> Result<String, String> {
    if client_id.is_empty() || client_id.len() > 2048 {
        return Err("Browser sign-in needs a registered public Hugging Face OAuth client ID. Add it in Account settings or use a read token.".into());
    }

    let generation = state.1.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;

    let response = client()?
        .post("https://huggingface.co/oauth/device")
        .form(&[
            ("client_id", client_id.as_str()),
            ("scope", "openid profile read-repos gated-repos"),
        ])
        .send()
        .await
        .map_err(|_| "Cannot start browser sign-in")?;

    let device = decode(response).await?;

    let code = device["device_code"]
        .as_str()
        .ok_or("No device code")?
        .to_owned();

    let mut interval = device["interval"].as_u64().unwrap_or(5).clamp(1, 60);

    let expiry = device["expires_in"].as_u64().unwrap_or(600).min(1800);

    let url = device["verification_uri"]
        .as_str()
        .ok_or("No verification URL")?
        .to_owned();

    let parsed = reqwest::Url::parse(&url).map_err(|_| "Invalid authorization URL")?;

    if parsed.scheme() != "https" || parsed.host_str() != Some("huggingface.co") {
        return Err("Untrusted authorization URL".into());
    }

    let (id, cancel) = jobs.begin("sign-in")?;

    jobs.update(&app, &id, |r| {
        r.result = json!({
        "url":url,"code":device["user_code"]}
        );

        r.message = "Open the browser and authorize the displayed code.".into();
    });

    let job_id = id.clone();

    tauri::async_runtime::spawn(async move {
        let result=async{
let until=std::time::Instant::now()+Duration::from_secs(expiry);
loop{
if state.1.load(std::sync::atomic::Ordering::SeqCst)!=generation{return Err("Sign-in cancelled or superseded".into());}
tokio::select!{
_=crate::session::cancelled(&cancel)=>return Err("Sign-in cancelled".into()),_=tokio::time::sleep(Duration::from_secs(interval))=>{
}
}
if std::time::Instant::now()>until{
return Err("Sign-in code expired. Try again.".into());
}
let r=client()?.post("https://huggingface.co/oauth/token").form(&[("client_id",client_id.as_str()),("device_code",code.as_str()),("grant_type","urn:ietf:params:oauth:grant-type:device_code")]).send().await.map_err(|_|"Sign-in connection failed")?;
let success=r.status().is_success();
let v:Value=r.json().await.map_err(|_|"Invalid sign-in response")?;
if success{
let token=v["access_token"].as_str().ok_or("Missing access token")?.to_owned();
crate::jobs::check(&cancel)?;
return connect_at(&state,token,Some(client_id.clone()),Some(generation)).await;
}
match v["error"].as_str(){
Some("authorization_pending")=>{
}
,Some("slow_down")=>interval=(interval+5).min(120),Some("access_denied")=>return Err("Authorization denied".into()),_=>return Err("Authorization expired or failed. Try signing in again.".into())}
}
}
.await;

        jobs.finish(&app, &job_id, result);
    });

    Ok(id)
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn repository_paths_are_encoded_and_contained() {
        assert!(segments("owner/../escape").is_err());

        assert!(segments("owner\\repo").is_err());

        assert_eq!(
            url_path(&["owner", "repo", "resolve", "sha", "a b.gguf"]).unwrap(),
            "/owner/repo/resolve/sha/a%20b.gguf"
        );
    }
}
