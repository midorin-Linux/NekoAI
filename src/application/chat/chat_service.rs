use crate::application::traits::ai_client::AIClient;

use anyhow::Result;

pub async fn proccess_message(client: &impl AIClient, prompt: String) -> Result<String> {
    client.generate(prompt).await
}
