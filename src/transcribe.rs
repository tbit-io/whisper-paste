use reqwest::multipart;
use serde::Deserialize;

#[derive(Deserialize)]
struct WhisperResponse {
    text: String,
}

pub async fn transcribe(api_key: &str, model: &str, wav_data: Vec<u8>) -> Result<String, String> {
    let part = multipart::Part::bytes(wav_data)
        .file_name("audio.wav")
        .mime_str("audio/wav")
        .map_err(|e| e.to_string())?;

    let form = multipart::Form::new()
        .text("model", model.to_string())
        .part("file", part);

    let client = reqwest::Client::new();
    let resp = client
        .post("https://api.openai.com/v1/audio/transcriptions")
        .bearer_auth(api_key)
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("request failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("API error {status}: {body}"));
    }

    let result: WhisperResponse = resp.json().await.map_err(|e| format!("parse error: {e}"))?;
    Ok(result.text)
}
