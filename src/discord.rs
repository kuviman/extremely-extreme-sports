use super::*;

pub fn send_activity(text: &str) {
    log::info!("{}", text);
    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Ok(url) = std::env::var("DISCORD_ACTIVITY_WEBHOOK") {
            let text = text.to_owned();
            std::thread::spawn(move || {
                let client = reqwest::blocking::Client::new();
                #[derive(Serialize)]
                struct Data {
                    content: String,
                }
                let data = Data { content: text };
                if let Err(e) = client.post(url).json(&data).send() {
                    log::error!("{}", e);
                }
            });
        }
    }
}
