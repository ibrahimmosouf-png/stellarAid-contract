use anyhow::Result;
use serde::Deserialize;
use std::env;
use reqwest::Client;

#[derive(Debug, Deserialize)]
struct Event {
    #[serde(rename = "type")]
    event_type: String,
    paging_token: String,
    // Add other event fields as needed
}

#[tokio::main]
async fn main() -> Result<()> {
    let contract_id = env::var("CONTRACT_ID").expect("CONTRACT_ID must be set");
    let webhook_url = env::var("WEBHOOK_URL").ok();

    let client = Client::new();
    let horizon_url = format!("https://horizon-testnet.stellar.org/contracts/{}/events", contract_id);

    println!("Monitoring events for contract: {}", contract_id);

    let mut last_paging_token = String::new();

    loop {
        let url = if last_paging_token.is_empty() {
            horizon_url.clone()
        } else {
            format!("{}?cursor={}", horizon_url, last_paging_token)
        };

        let response = client.get(&url).send().await?;
        let text = response.text().await?;

        for line in text.lines() {
            if line.starts_with("data: ") {
                let data = &line[6..];
                if let Ok(event) = serde_json::from_str::<Event>(data) {
                    println!("Received event: {:?}", event);
                    last_paging_token = event.paging_token.clone();

                    // Check for anomalous events
                    if let Some(ref url) = webhook_url {
                        if is_anomalous(&event) {
                            send_alert(url, &event).await?;
                        }
                    }
                }
            }
        }
    }
}

fn is_anomalous(event: &Event) -> bool {
    // Implement logic to detect anomalous events
    // For example, check for high-volume donations, unusual fund release patterns, or contract freeze events.
    match event.event_type.as_str() {
        "contract_frozen" => true,
        _ => false,
    }
}

async fn send_alert(webhook_url: &str, event: &Event) -> Result<()> {
    let client = Client::new();
    let message = format!("Anomalous event detected: {:?}", event);
    let payload = serde_json::json!({ "text": message });

    client.post(webhook_url).json(&payload).send().await?;

    println!("Sent alert for event: {:?}", event);

    Ok(())
}