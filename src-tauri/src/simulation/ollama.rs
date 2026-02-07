use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct OllamaResponse {
    response: String,
}

#[derive(Debug, Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    system: String,
    stream: bool,
}

pub async fn name_species(
    url: &str,
    model: &str,
    hue: f32,
    speed: f32,
    size: f32,
    pattern: &str,
    population: u32,
    generation: u32,
) -> Option<(String, String)> {
    let color = hue_to_color_name(hue);
    let size_desc = if size < 0.9 { "small" } else if size > 1.4 { "large" } else { "medium-sized" };
    let speed_desc = if speed < 0.8 { "slow" } else if speed > 1.4 { "fast" } else { "moderate-speed" };

    let prompt = format!(
        "A new fish species has been detected with these traits:\n\
         - Color: {} (hue ~{}°)\n\
         - Body: {} ({}x)\n\
         - Pattern: {}\n\
         - Speed: {} ({}x)\n\
         - Population: {} members, generation {}\n\n\
         Respond ONLY in JSON: {{\"name\": \"Latin-style Genus species\", \"description\": \"One sentence description\"}}",
        color, hue as u32, size_desc, size, pattern, speed_desc, speed, population, generation
    );

    let req = OllamaRequest {
        model: model.to_string(),
        prompt,
        system: "You are a marine biologist. Generate a species name (Latin-style genus + species) and a one-sentence description. Keep it concise and scientific but with personality. Respond ONLY in JSON.".to_string(),
        stream: false,
    };

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/api/generate", url))
        .json(&req)
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .ok()?;

    let body: OllamaResponse = resp.json().await.ok()?;

    // Parse JSON from response
    let text = body.response.trim();
    // Find JSON in the response
    let start = text.find('{')?;
    let end = text.rfind('}')? + 1;
    let json_str = &text[start..end];
    let parsed: serde_json::Value = serde_json::from_str(json_str).ok()?;

    let name = parsed.get("name")?.as_str()?.to_string();
    let desc = parsed.get("description")?.as_str()?.to_string();
    Some((name, desc))
}

pub async fn generate_journal_entry(
    url: &str,
    model: &str,
    tick: u64,
    population: u32,
    water_quality: f32,
    species_summary: &str,
) -> Option<String> {
    let day = tick / 1800; // ~1 minute = 1 day

    let prompt = format!(
        "Current aquarium state:\n\
         - Day: {}\n\
         - Population: {} fish\n\
         - Water quality: {:.0}%\n\
         - Species: {}\n\n\
         Write a brief 2-3 sentence field note.",
        day, population, water_quality * 100.0, species_summary
    );

    let req = OllamaRequest {
        model: model.to_string(),
        prompt,
        system: "You are a marine biologist keeping a daily log of a digital aquarium that evolves through genetic algorithms. Write brief observational field notes in first person, present tense.".to_string(),
        stream: false,
    };

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/api/generate", url))
        .json(&req)
        .timeout(std::time::Duration::from_secs(20))
        .send()
        .await
        .ok()?;

    let body: OllamaResponse = resp.json().await.ok()?;
    Some(body.response.trim().to_string())
}

fn hue_to_color_name(hue: f32) -> &'static str {
    match hue as u32 {
        0..=15 | 346..=360 => "red",
        16..=45 => "orange",
        46..=70 => "yellow",
        71..=150 => "green",
        151..=200 => "cyan",
        201..=260 => "blue",
        261..=290 => "purple",
        291..=345 => "pink",
        _ => "colorful",
    }
}

/// Fallback species name when Ollama is unavailable
pub fn fallback_species_name(hue: f32, speed: f32, pattern: &str, size: f32) -> String {
    let color = hue_to_color_name(hue);
    let color = format!("{}{}", &color[..1].to_uppercase(), &color[1..]);
    let pattern_word = match pattern.split('{').next().unwrap_or("Solid").trim() {
        "Striped" => "Striped",
        "Spotted" => "Spotted",
        "Gradient" => "Shaded",
        "Bicolor" => "Two-tone",
        _ => "",
    };
    let behavior = if speed > 1.4 { "Darters" } else if size > 1.4 { "Giants" } else { "Drifters" };
    if pattern_word.is_empty() {
        format!("{} {}", color, behavior)
    } else {
        format!("{} {} {}", color, pattern_word, behavior)
    }
}
