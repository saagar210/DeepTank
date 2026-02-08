mod simulation;

use simulation::SimulationState;
use simulation::achievements::{self, Achievement};
use simulation::genome::FishGenome;
use simulation::persistence;
use simulation::ollama;
use std::sync::Mutex;
use std::time::Duration;
use rand::Rng;
use tauri::{Emitter, Manager};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;

#[tauri::command]
fn pause(state: tauri::State<'_, Mutex<SimulationState>>) {
    state.lock().unwrap().paused = true;
}

#[tauri::command]
fn resume(state: tauri::State<'_, Mutex<SimulationState>>) {
    state.lock().unwrap().paused = false;
}

#[tauri::command]
fn set_speed(state: tauri::State<'_, Mutex<SimulationState>>, multiplier: f32) {
    state.lock().unwrap().speed_multiplier = multiplier.clamp(0.25, 4.0);
}

#[tauri::command]
fn feed(state: tauri::State<'_, Mutex<SimulationState>>, x: f32, y: f32, food_type: Option<String>) {
    let mut sim = state.lock().unwrap();
    if let Some(ft) = food_type {
        sim.ecosystem.drop_food_typed(x, y, simulation::ecosystem::FoodType::from_str(&ft));
    } else {
        sim.ecosystem.drop_food(x, y);
    }
}

#[tauri::command]
fn step_forward(state: tauri::State<'_, Mutex<SimulationState>>) -> simulation::FrameUpdate {
    let mut sim = state.lock().unwrap();
    let was_paused = sim.paused;
    sim.paused = false;
    let frame = sim.step();
    sim.paused = was_paused;
    frame
}

#[tauri::command]
fn select_fish(state: tauri::State<'_, Mutex<SimulationState>>, id: Option<u32>) {
    state.lock().unwrap().selected_fish_id = id;
}

#[tauri::command]
fn tap_glass(state: tauri::State<'_, Mutex<SimulationState>>, x: f32, y: f32) {
    let mut sim = state.lock().unwrap();
    // Collect boldness values to avoid borrow conflict
    let boldness_map: std::collections::HashMap<u32, f32> = sim.genomes.iter()
        .map(|(&id, g)| (id, g.boldness))
        .collect();
    simulation::ecosystem::EcosystemManager::apply_glass_tap(
        &mut sim.fish,
        &boldness_map,
        x,
        y,
    );
}

#[tauri::command]
fn trigger_event(state: tauri::State<'_, Mutex<SimulationState>>, event_type: String) -> Result<(), String> {
    let event = simulation::events::EnvironmentalEvent::from_str(&event_type)
        .ok_or_else(|| format!("Unknown event type: {}", event_type))?;
    let mut sim = state.lock().unwrap();
    sim.event_system.trigger(event);
    Ok(())
}

#[tauri::command]
fn breed_fish(state: tauri::State<'_, Mutex<SimulationState>>, fish_a_id: u32, fish_b_id: u32) -> Result<u32, String> {
    let mut sim = state.lock().unwrap();
    let tick = sim.tick;
    let config = sim.config.clone();
    let SimulationState { ref mut ecosystem, ref mut fish, ref mut genomes, ref mut rng, .. } = *sim;
    ecosystem.force_breed(fish, genomes, &config, tick, rng, fish_a_id, fish_b_id)
}

#[tauri::command]
fn get_breed_preview(state: tauri::State<'_, Mutex<SimulationState>>, genome_a_id: u32, genome_b_id: u32) -> Result<serde_json::Value, String> {
    let sim = state.lock().unwrap();
    let ga = sim.genomes.get(&genome_a_id).ok_or("Genome A not found")?;
    let gb = sim.genomes.get(&genome_b_id).ok_or("Genome B not found")?;
    // Predict offspring trait ranges (midpoint +/- 10% variance)
    let mid = |a: f32, b: f32| -> (f32, f32, f32) {
        let m = (a + b) / 2.0;
        (m * 0.9, m, m * 1.1)
    };
    let (spd_min, spd_mid, spd_max) = mid(ga.speed, gb.speed);
    let (sz_min, sz_mid, sz_max) = mid(ga.body_length, gb.body_length);
    let (ag_min, ag_mid, ag_max) = mid(ga.aggression, gb.aggression);
    let (bold_min, bold_mid, bold_max) = mid(ga.boldness, gb.boldness);
    let (school_min, school_mid, school_max) = mid(ga.school_affinity, gb.school_affinity);
    let (meta_min, meta_mid, meta_max) = mid(ga.metabolism, gb.metabolism);
    Ok(serde_json::json!({
        "speed": { "min": spd_min, "mid": spd_mid, "max": spd_max, "parent_a": ga.speed, "parent_b": gb.speed },
        "size": { "min": sz_min, "mid": sz_mid, "max": sz_max, "parent_a": ga.body_length, "parent_b": gb.body_length },
        "aggression": { "min": ag_min, "mid": ag_mid, "max": ag_max, "parent_a": ga.aggression, "parent_b": gb.aggression },
        "boldness": { "min": bold_min, "mid": bold_mid, "max": bold_max, "parent_a": ga.boldness, "parent_b": gb.boldness },
        "school_affinity": { "min": school_min, "mid": school_mid, "max": school_max, "parent_a": ga.school_affinity, "parent_b": gb.school_affinity },
        "metabolism": { "min": meta_min, "mid": meta_mid, "max": meta_max, "parent_a": ga.metabolism, "parent_b": gb.metabolism },
    }))
}

#[tauri::command]
fn get_genome(state: tauri::State<'_, Mutex<SimulationState>>, genome_id: u32) -> Option<FishGenome> {
    state.lock().unwrap().get_genome(genome_id).cloned()
}

#[tauri::command]
fn get_all_genomes(state: tauri::State<'_, Mutex<SimulationState>>) -> Vec<FishGenome> {
    state.lock().unwrap().genomes.values().cloned().collect()
}

#[tauri::command]
fn get_species_list(state: tauri::State<'_, Mutex<SimulationState>>) -> Vec<simulation::ecosystem::Species> {
    state.lock().unwrap().ecosystem.species.clone()
}

#[tauri::command]
fn get_species_history(state: tauri::State<'_, Mutex<SimulationState>>) -> Vec<serde_json::Value> {
    let sim = state.lock().unwrap();
    sim.ecosystem.species.iter().map(|s| {
        // Find a representative genome for this species
        let rep_genome_id = s.member_genome_ids.first().copied();
        serde_json::json!({
            "id": s.id,
            "name": s.name,
            "description": s.description,
            "discovered_at_tick": s.discovered_at_tick,
            "extinct_at_tick": s.extinct_at_tick,
            "centroid_hue": s.centroid_hue,
            "centroid_speed": s.centroid_speed,
            "centroid_size": s.centroid_size,
            "centroid_pattern": s.centroid_pattern,
            "member_count": s.member_count,
            "representative_genome_id": rep_genome_id,
        })
    }).collect()
}

#[tauri::command]
fn get_fish_detail(state: tauri::State<'_, Mutex<SimulationState>>, fish_id: u32) -> Option<serde_json::Value> {
    let sim = state.lock().unwrap();
    let fish = sim.fish.iter().find(|f| f.id == fish_id)?;
    let genome = sim.genomes.get(&fish.genome_id)?;
    let species_name = sim.ecosystem.species.iter()
        .find(|s| s.extinct_at_tick.is_none() && s.member_genome_ids.contains(&fish.genome_id))
        .and_then(|s| s.name.clone());

    Some(serde_json::json!({
        "id": fish.id,
        "genome_id": fish.genome_id,
        "x": fish.x,
        "y": fish.y,
        "z": fish.z,
        "heading": fish.heading,
        "age": fish.age,
        "hunger": fish.hunger,
        "health": fish.health,
        "energy": fish.energy,
        "behavior": fish.behavior.as_str(),
        "meals_eaten": fish.meals_eaten,
        "is_alive": fish.is_alive,
        "is_infected": fish.is_infected,
        "custom_name": fish.custom_name,
        "is_favorite": fish.is_favorite,
        "genome": genome,
        "species_name": species_name,
    }))
}

#[tauri::command]
fn name_fish(state: tauri::State<'_, Mutex<SimulationState>>, fish_id: u32, name: String) -> Result<(), String> {
    let mut sim = state.lock().unwrap();
    let fish = sim.fish.iter_mut().find(|f| f.id == fish_id && f.is_alive)
        .ok_or("Fish not found")?;
    let trimmed = name.trim();
    fish.custom_name = if trimmed.is_empty() { None } else { Some(trimmed.chars().take(20).collect()) };
    Ok(())
}

#[tauri::command]
fn toggle_favorite(state: tauri::State<'_, Mutex<SimulationState>>, fish_id: u32) -> Result<bool, String> {
    let mut sim = state.lock().unwrap();
    let fish = sim.fish.iter_mut().find(|f| f.id == fish_id && f.is_alive)
        .ok_or("Fish not found")?;
    fish.is_favorite = !fish.is_favorite;
    Ok(fish.is_favorite)
}

#[tauri::command]
fn get_favorites(state: tauri::State<'_, Mutex<SimulationState>>) -> Vec<serde_json::Value> {
    let sim = state.lock().unwrap();
    sim.fish.iter()
        .filter(|f| f.is_favorite)
        .filter_map(|f| {
            let genome = sim.genomes.get(&f.genome_id)?;
            let species_name = sim.ecosystem.species.iter()
                .find(|s| s.extinct_at_tick.is_none() && s.member_genome_ids.contains(&f.genome_id))
                .and_then(|s| s.name.clone());
            Some(serde_json::json!({
                "id": f.id,
                "custom_name": f.custom_name,
                "species_name": species_name,
                "is_alive": f.is_alive,
                "age": f.age,
                "genome_id": f.genome_id,
                "hue": genome.base_hue,
            }))
        })
        .collect()
}

#[tauri::command]
fn update_tank_size(state: tauri::State<'_, Mutex<SimulationState>>, width: f32, height: f32) {
    let mut sim = state.lock().unwrap();
    sim.config.tank_width = width;
    sim.config.tank_height = height;
    sim.boids.grid = simulation::boids::SpatialGrid::new(width, height, sim.config.cohesion_radius);
}

#[tauri::command]
fn get_snapshots(db: tauri::State<'_, Mutex<Option<rusqlite::Connection>>>) -> Vec<serde_json::Value> {
    let guard = db.lock().unwrap();
    let conn = match guard.as_ref() {
        Some(c) => c,
        None => return Vec::new(),
    };
    let mut stmt = match conn.prepare(
        "SELECT tick, population, species_count, water_quality, avg_hue, avg_speed, avg_size, avg_aggression,
                avg_boldness, avg_school_affinity, avg_disease_resistance, min_speed, max_speed, min_size, max_size,
                genetic_diversity
         FROM population_snapshots ORDER BY tick DESC LIMIT 200"
    ) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let mut results = Vec::new();
    if let Ok(rows) = stmt.query_map([], |row| {
        Ok(serde_json::json!({
            "tick": row.get::<_, i64>(0).unwrap_or(0),
            "population": row.get::<_, i32>(1).unwrap_or(0),
            "species_count": row.get::<_, i32>(2).unwrap_or(0),
            "water_quality": row.get::<_, f64>(3).unwrap_or(0.0),
            "avg_hue": row.get::<_, f64>(4).unwrap_or(0.0),
            "avg_speed": row.get::<_, f64>(5).unwrap_or(0.0),
            "avg_size": row.get::<_, f64>(6).unwrap_or(0.0),
            "avg_aggression": row.get::<_, f64>(7).unwrap_or(0.0),
            "avg_boldness": row.get::<_, f64>(8).unwrap_or(0.5),
            "avg_school_affinity": row.get::<_, f64>(9).unwrap_or(0.5),
            "avg_disease_resistance": row.get::<_, f64>(10).unwrap_or(0.5),
            "min_speed": row.get::<_, f64>(11).unwrap_or(0.5),
            "max_speed": row.get::<_, f64>(12).unwrap_or(2.0),
            "min_size": row.get::<_, f64>(13).unwrap_or(0.6),
            "max_size": row.get::<_, f64>(14).unwrap_or(2.0),
            "genetic_diversity": row.get::<_, f64>(15).unwrap_or(0.5),
        }))
    }) {
        for r in rows.flatten() {
            results.push(r);
        }
    }
    results.reverse();
    results
}

#[tauri::command]
fn get_all_snapshots(db: tauri::State<'_, Mutex<Option<rusqlite::Connection>>>) -> Vec<serde_json::Value> {
    let guard = db.lock().unwrap();
    let conn = match guard.as_ref() {
        Some(c) => c,
        None => return Vec::new(),
    };
    let mut stmt = match conn.prepare(
        "SELECT tick, population, species_count, water_quality, avg_hue, avg_speed, avg_size, avg_aggression,
                avg_boldness, avg_school_affinity, avg_disease_resistance, min_speed, max_speed, min_size, max_size,
                genetic_diversity
         FROM population_snapshots ORDER BY tick ASC LIMIT 10000"
    ) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let mut results = Vec::new();
    if let Ok(rows) = stmt.query_map([], |row| {
        Ok(serde_json::json!({
            "tick": row.get::<_, i64>(0).unwrap_or(0),
            "population": row.get::<_, i32>(1).unwrap_or(0),
            "species_count": row.get::<_, i32>(2).unwrap_or(0),
            "water_quality": row.get::<_, f64>(3).unwrap_or(0.0),
            "avg_hue": row.get::<_, f64>(4).unwrap_or(0.0),
            "avg_speed": row.get::<_, f64>(5).unwrap_or(0.0),
            "avg_size": row.get::<_, f64>(6).unwrap_or(0.0),
            "avg_aggression": row.get::<_, f64>(7).unwrap_or(0.0),
            "avg_boldness": row.get::<_, f64>(8).unwrap_or(0.5),
            "avg_school_affinity": row.get::<_, f64>(9).unwrap_or(0.5),
            "avg_disease_resistance": row.get::<_, f64>(10).unwrap_or(0.5),
            "min_speed": row.get::<_, f64>(11).unwrap_or(0.5),
            "max_speed": row.get::<_, f64>(12).unwrap_or(2.0),
            "min_size": row.get::<_, f64>(13).unwrap_or(0.6),
            "max_size": row.get::<_, f64>(14).unwrap_or(2.0),
            "genetic_diversity": row.get::<_, f64>(15).unwrap_or(0.5),
        }))
    }) {
        for r in rows.flatten() {
            results.push(r);
        }
    }
    results
}

#[tauri::command]
fn get_config(state: tauri::State<'_, Mutex<SimulationState>>) -> serde_json::Value {
    let sim = state.lock().unwrap();
    serde_json::to_value(&sim.config).unwrap_or_default()
}

#[tauri::command]
fn update_config(state: tauri::State<'_, Mutex<SimulationState>>, key: String, value: serde_json::Value) {
    let mut sim = state.lock().unwrap();
    let c = &mut sim.config;
    match key.as_str() {
        "separation_weight" => if let Some(v) = value.as_f64() { c.separation_weight = v as f32; },
        "alignment_weight" => if let Some(v) = value.as_f64() { c.alignment_weight = v as f32; },
        "cohesion_weight" => if let Some(v) = value.as_f64() { c.cohesion_weight = v as f32; },
        "wander_strength" => if let Some(v) = value.as_f64() { c.wander_strength = v as f32; },
        "hunger_rate" => if let Some(v) = value.as_f64() { c.hunger_rate = v as f32; },
        "mutation_rate_small" => if let Some(v) = value.as_f64() { c.mutation_rate_small = v as f32; },
        "mutation_rate_large" => if let Some(v) = value.as_f64() { c.mutation_rate_large = v as f32; },
        "species_threshold" => if let Some(v) = value.as_f64() { c.species_threshold = v as f32; },
        "day_night_cycle" => if let Some(v) = value.as_bool() { c.day_night_cycle = v; },
        "day_night_speed" => if let Some(v) = value.as_f64() { c.day_night_speed = v as f32; },
        "bubble_rate" => if let Some(v) = value.as_f64() { c.bubble_rate = v as f32; },
        "current_strength" => if let Some(v) = value.as_f64() { c.current_strength = v as f32; },
        "auto_feed_enabled" => if let Some(v) = value.as_bool() { c.auto_feed_enabled = v; },
        "auto_feed_interval" => if let Some(v) = value.as_f64() { c.auto_feed_interval = v as u32; },
        "auto_feed_amount" => if let Some(v) = value.as_f64() { c.auto_feed_amount = v as u32; },
        "ollama_enabled" => if let Some(v) = value.as_bool() { c.ollama_enabled = v; },
        "ollama_url" => if let Some(v) = value.as_str() {
            // Basic URL validation: must start with http:// or https://
            if v.starts_with("http://") || v.starts_with("https://") {
                c.ollama_url = v.to_string();
            }
        },
        "ollama_model" => if let Some(v) = value.as_str() { c.ollama_model = v.to_string(); },
        "master_volume" => if let Some(v) = value.as_f64() { c.master_volume = v as f32; },
        "ambient_enabled" => if let Some(v) = value.as_bool() { c.ambient_enabled = v; },
        "event_sounds_enabled" => if let Some(v) = value.as_bool() { c.event_sounds_enabled = v; },
        "theme" => if let Some(v) = value.as_str() { c.theme = v.to_string(); },
        "environmental_events_enabled" => if let Some(v) = value.as_bool() { c.environmental_events_enabled = v; },
        "event_frequency" => if let Some(v) = value.as_f64() { c.event_frequency = v as f32; },
        "territory_enabled" => if let Some(v) = value.as_bool() { c.territory_enabled = v; },
        "territory_claim_radius" => if let Some(v) = value.as_f64() { c.territory_claim_radius = v as f32; },
        "disease_enabled" => if let Some(v) = value.as_bool() { c.disease_enabled = v; },
        "disease_infection_chance" => if let Some(v) = value.as_f64() { c.disease_infection_chance = v as f32; },
        "disease_spontaneous_chance" => if let Some(v) = value.as_f64() { c.disease_spontaneous_chance = v as f32; },
        "disease_duration" => if let Some(v) = value.as_u64() { c.disease_duration = v as u32; },
        "disease_damage" => if let Some(v) = value.as_f64() { c.disease_damage = v as f32; },
        "disease_spread_radius" => if let Some(v) = value.as_f64() { c.disease_spread_radius = v as f32; },
        _ => {}
    }
}

#[tauri::command]
fn get_species_snapshots(db: tauri::State<'_, Mutex<Option<rusqlite::Connection>>>) -> Vec<serde_json::Value> {
    let guard = db.lock().unwrap();
    let conn = match guard.as_ref() {
        Some(c) => c,
        None => return Vec::new(),
    };
    let data = persistence::get_species_snapshots(conn);
    data.into_iter().map(|(tick, sp_id, name, pop)| {
        serde_json::json!({ "tick": tick, "species_id": sp_id, "species_name": name, "population": pop })
    }).collect()
}

#[tauri::command]
fn get_events(db: tauri::State<'_, Mutex<Option<rusqlite::Connection>>>, event_type: Option<String>, limit: Option<u32>) -> Vec<serde_json::Value> {
    let guard = db.lock().unwrap();
    let conn = match guard.as_ref() {
        Some(c) => c,
        None => return Vec::new(),
    };
    let lim = limit.unwrap_or(100) as i64;
    let mut results = Vec::new();
    if let Some(ref etype) = event_type {
        let mut stmt = match conn.prepare(
            "SELECT tick, event_type, subject_fish_id, subject_species_id, description, timestamp FROM events WHERE event_type = ?1 ORDER BY id DESC LIMIT ?2"
        ) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        let rows = stmt.query_map(rusqlite::params![etype, lim], |row| {
            Ok(serde_json::json!({
                "tick": row.get::<_, i64>(0).unwrap_or(0),
                "event_type": row.get::<_, String>(1).unwrap_or_default(),
                "fish_id": row.get::<_, Option<i64>>(2).unwrap_or(None),
                "species_id": row.get::<_, Option<i64>>(3).unwrap_or(None),
                "description": row.get::<_, String>(4).unwrap_or_default(),
                "timestamp": row.get::<_, String>(5).unwrap_or_default(),
            }))
        }).ok();
        if let Some(rows) = rows {
            for r in rows.flatten() { results.push(r); }
        }
    } else {
        let mut stmt = match conn.prepare(
            "SELECT tick, event_type, subject_fish_id, subject_species_id, description, timestamp FROM events ORDER BY id DESC LIMIT ?1"
        ) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        let rows = stmt.query_map(rusqlite::params![lim], |row| {
            Ok(serde_json::json!({
                "tick": row.get::<_, i64>(0).unwrap_or(0),
                "event_type": row.get::<_, String>(1).unwrap_or_default(),
                "fish_id": row.get::<_, Option<i64>>(2).unwrap_or(None),
                "species_id": row.get::<_, Option<i64>>(3).unwrap_or(None),
                "description": row.get::<_, String>(4).unwrap_or_default(),
                "timestamp": row.get::<_, String>(5).unwrap_or_default(),
            }))
        }).ok();
        if let Some(rows) = rows {
            for r in rows.flatten() { results.push(r); }
        }
    }
    results
}

#[tauri::command]
fn get_journal_entries(db: tauri::State<'_, Mutex<Option<rusqlite::Connection>>>) -> Vec<serde_json::Value> {
    let guard = db.lock().unwrap();
    let conn = match guard.as_ref() {
        Some(c) => c,
        None => return Vec::new(),
    };
    let mut results = Vec::new();
    if let Ok(mut stmt) = conn.prepare("SELECT tick, entry_text, timestamp FROM journal_entries ORDER BY tick DESC LIMIT 50") {
        if let Ok(rows) = stmt.query_map([], |row| {
            Ok(serde_json::json!({
                "tick": row.get::<_, i64>(0).unwrap_or(0),
                "text": row.get::<_, String>(1).unwrap_or_default(),
                "timestamp": row.get::<_, String>(2).unwrap_or_default(),
            }))
        }) {
            for r in rows.flatten() {
                results.push(r);
            }
        }
    }
    results
}

#[tauri::command]
fn add_decoration(
    state: tauri::State<'_, Mutex<SimulationState>>,
    db: tauri::State<'_, Mutex<Option<rusqlite::Connection>>>,
    decoration_type: String,
    x: f32,
    y: f32,
    scale: f32,
    flip_x: bool,
) -> serde_json::Value {
    let dtype = simulation::ecosystem::DecorationType::from_str(&decoration_type);
    let mut sim = state.lock().unwrap();
    let d = sim.ecosystem.add_decoration(dtype, x, y, scale, flip_x);
    // Persist to DB
    let guard = db.lock().unwrap();
    if let Some(ref conn) = *guard {
        conn.execute(
            "INSERT INTO decorations (id, decoration_type, position_x, position_y, scale, flip_x) VALUES (?1,?2,?3,?4,?5,?6)",
            rusqlite::params![d.id, d.decoration_type.as_str(), d.x, d.y, d.scale, d.flip_x as i32],
        ).ok();
    }
    serde_json::json!({ "id": d.id, "decoration_type": d.decoration_type.as_str(), "x": d.x, "y": d.y, "scale": d.scale, "flip_x": d.flip_x })
}

#[tauri::command]
fn remove_decoration(
    state: tauri::State<'_, Mutex<SimulationState>>,
    db: tauri::State<'_, Mutex<Option<rusqlite::Connection>>>,
    id: u32,
) -> bool {
    let mut sim = state.lock().unwrap();
    let removed = sim.ecosystem.remove_decoration(id);
    if removed {
        let guard = db.lock().unwrap();
        if let Some(ref conn) = *guard {
            conn.execute("DELETE FROM decorations WHERE id = ?1", rusqlite::params![id]).ok();
        }
    }
    removed
}

#[tauri::command]
fn get_decorations(state: tauri::State<'_, Mutex<SimulationState>>) -> Vec<serde_json::Value> {
    let sim = state.lock().unwrap();
    sim.ecosystem.decorations.iter().map(|d| {
        serde_json::json!({
            "id": d.id,
            "decoration_type": d.decoration_type.as_str(),
            "x": d.x, "y": d.y,
            "scale": d.scale,
            "flip_x": d.flip_x,
        })
    }).collect()
}

#[tauri::command]
fn get_achievements(state: tauri::State<'_, Mutex<Vec<Achievement>>>) -> Vec<Achievement> {
    state.lock().unwrap().clone()
}

#[tauri::command]
fn get_lineage(
    state: tauri::State<'_, Mutex<SimulationState>>,
    db: tauri::State<'_, Mutex<Option<rusqlite::Connection>>>,
    genome_id: u32,
    depth: Option<u32>,
) -> Vec<serde_json::Value> {
    let max_depth = depth.unwrap_or(5);
    let sim = state.lock().unwrap();
    let db_guard = db.lock().unwrap();

    let mut result = Vec::new();
    let mut queue: Vec<(u32, u32)> = vec![(genome_id, 0)]; // (genome_id, depth)
    let mut visited = std::collections::HashSet::new();
    visited.insert(genome_id);

    while let Some((gid, d)) = queue.pop() {
        // Try memory first, then DB
        let genome = sim.genomes.get(&gid).cloned().or_else(|| {
            if let Some(ref conn) = *db_guard {
                let mut stmt = conn.prepare(
                    "SELECT id, generation, parent_a, parent_b, sex, base_hue, saturation, lightness,
                     body_length, speed, aggression FROM genomes WHERE id = ?1"
                ).ok()?;
                stmt.query_row(rusqlite::params![gid], |row| {
                    Ok(FishGenome {
                        id: row.get(0)?,
                        generation: row.get(1)?,
                        parent_a: row.get(2)?,
                        parent_b: row.get(3)?,
                        sex: if row.get::<_, String>(4)? == "male" {
                            simulation::genome::Sex::Male
                        } else {
                            simulation::genome::Sex::Female
                        },
                        base_hue: row.get(5)?,
                        saturation: row.get(6)?,
                        lightness: row.get(7)?,
                        body_length: row.get(8)?,
                        speed: row.get(9)?,
                        aggression: row.get(10)?,
                        // Defaults for fields we don't need for lineage display
                        body_width: 0.5,
                        tail_size: 0.5,
                        dorsal_fin_size: 0.5,
                        pectoral_fin_size: 0.5,
                        pattern: simulation::genome::PatternGene::Solid,
                        pattern_intensity: 0.5,
                        pattern_color_offset: 0.0,
                        eye_size: 0.5,
                        school_affinity: 0.5,
                        curiosity: 0.5,
                        boldness: 0.5,
                        metabolism: 1.0,
                        fertility: 0.5,
                        lifespan_factor: 1.0,
                        maturity_age: 0.2,
                        disease_resistance: 0.5,
                    })
                }).ok()
            } else {
                None
            }
        });

        if let Some(g) = genome {
            let is_alive = sim.fish.iter().any(|f| f.genome_id == gid && f.is_alive);
            result.push(serde_json::json!({
                "genome_id": g.id,
                "generation": g.generation,
                "parent_a": g.parent_a,
                "parent_b": g.parent_b,
                "base_hue": g.base_hue,
                "speed": g.speed,
                "body_length": g.body_length,
                "depth": d,
                "is_alive": is_alive,
            }));

            if d < max_depth {
                if let Some(pa) = g.parent_a {
                    if visited.insert(pa) {
                        queue.push((pa, d + 1));
                    }
                }
                if let Some(pb) = g.parent_b {
                    if visited.insert(pb) {
                        queue.push((pb, d + 1));
                    }
                }
            }
        }
    }

    result
}

#[tauri::command]
async fn export_tank(
    state: tauri::State<'_, Mutex<SimulationState>>,
    db: tauri::State<'_, Mutex<Option<rusqlite::Connection>>>,
    app: tauri::AppHandle,
) -> Result<String, String> {
    use tauri_plugin_dialog::DialogExt;

    // Force save first
    {
        let sim = state.lock().unwrap();
        let db_guard = db.lock().unwrap();
        if let Some(ref conn) = *db_guard {
            persistence::save_state(conn, sim.tick, sim.ecosystem.water_quality, &sim.fish, &sim.genomes, &sim.ecosystem.species, &sim.ecosystem.eggs)
                .map_err(|e| e.to_string())?;
        }
    }

    let db_path = get_db_path();
    let dialog = tauri_plugin_dialog::FileDialogBuilder::new(app.dialog().clone())
        .add_filter("DeepTank Save", &["deeptank"])
        .set_file_name("my_aquarium.deeptank")
        .set_title("Export Tank");

    let path = dialog.blocking_save_file();
    match path {
        Some(p) => {
            let dest = p.as_path().ok_or("Invalid path")?;
            std::fs::copy(&db_path, dest).map_err(|e| e.to_string())?;
            Ok(dest.display().to_string())
        }
        None => Err("Cancelled".to_string()),
    }
}

#[tauri::command]
async fn import_tank(
    db: tauri::State<'_, Mutex<Option<rusqlite::Connection>>>,
    app: tauri::AppHandle,
) -> Result<String, String> {
    use tauri_plugin_dialog::DialogExt;

    let dialog = tauri_plugin_dialog::FileDialogBuilder::new(app.dialog().clone())
        .add_filter("DeepTank Save", &["deeptank"])
        .set_title("Import Tank");

    let path = dialog.blocking_pick_file();
    match path {
        Some(p) => {
            let src = p.as_path().ok_or("Invalid path")?;
            let db_path = get_db_path();

            // Close current DB connection
            {
                let mut db_guard = db.lock().unwrap();
                *db_guard = None;
            }

            // Copy imported file over the DB
            std::fs::copy(src, &db_path).map_err(|e| e.to_string())?;

            // Reopen DB
            {
                let mut db_guard = db.lock().unwrap();
                *db_guard = persistence::open_db(&db_path).ok();
            }

            // Reload will be triggered by frontend
            // Use webview to reload
            if let Some(w) = app.get_webview_window("main") {
                w.eval("window.location.reload()").ok();
            }

            Ok(src.display().to_string())
        }
        None => Err("Cancelled".to_string()),
    }
}

fn get_db_dir() -> std::path::PathBuf {
    let mut path = dirs::data_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    path.push("DeepTank");
    std::fs::create_dir_all(&path).ok();
    path
}

fn get_db_path() -> std::path::PathBuf {
    let mut path = get_db_dir();
    path.push("deeptank.db");
    path
}

fn tank_name_to_filename(name: &str) -> String {
    let mut slug = String::new();
    let mut last_was_dash = false;
    for c in name.to_lowercase().chars() {
        if c.is_alphanumeric() {
            slug.push(c);
            last_was_dash = false;
        } else if !last_was_dash {
            slug.push('-');
            last_was_dash = true;
        }
    }
    let slug = slug.trim_matches('-');
    format!("deeptank_{}.db", slug)
}

fn get_tank_db_path(name: &str) -> std::path::PathBuf {
    let mut path = get_db_dir();
    path.push(tank_name_to_filename(name));
    path
}

/// Save current simulation state to the currently open DB connection.
fn save_current_state(
    sim: &SimulationState,
    conn: &rusqlite::Connection,
) {
    persistence::save_state(
        conn,
        sim.tick,
        sim.ecosystem.water_quality,
        &sim.fish,
        &sim.genomes,
        &sim.ecosystem.species,
        &sim.ecosystem.eggs,
    ).ok();
}

/// Load a tank from a DB path into the SimulationState, returning the new connection.
fn load_tank_from_db(db_path: &std::path::Path) -> Result<(SimulationState, rusqlite::Connection), String> {
    let conn = persistence::open_db(db_path).map_err(|e| format!("Failed to open DB: {}", e))?;
    persistence::init_schema(&conn).map_err(|e| format!("Schema init failed: {}", e))?;

    let state = match persistence::load_state(&conn) {
        Ok(Some((tick, wq, fish, genomes, species, eggs, max_species_id))) => {
            let mut s = SimulationState::new();
            s.tick = tick;
            s.ecosystem.water_quality = wq;
            s.fish = fish;
            s.genomes = genomes;
            s.ecosystem.species = species;
            s.ecosystem.eggs = eggs;
            s.ecosystem.restore_species_counter(max_species_id + 1);
            s.ecosystem.restore_speciation_tick(tick);
            // Load decorations
            if let Ok(mut stmt) = conn.prepare("SELECT id, decoration_type, position_x, position_y, scale, flip_x FROM decorations") {
                if let Ok(rows) = stmt.query_map([], |row| {
                    Ok(simulation::ecosystem::Decoration {
                        id: row.get(0)?,
                        decoration_type: simulation::ecosystem::DecorationType::from_str(&row.get::<_, String>(1)?),
                        x: row.get(2)?,
                        y: row.get(3)?,
                        scale: row.get::<_, f64>(4)? as f32,
                        flip_x: row.get::<_, i32>(5)? != 0,
                    })
                }) {
                    for r in rows.flatten() {
                        s.ecosystem.decorations.push(r);
                    }
                    let max_dec_id = s.ecosystem.decorations.iter().map(|d| d.id).max().unwrap_or(0);
                    s.ecosystem.restore_decoration_counter(max_dec_id + 1);
                    s.ecosystem.recompute_plant_count();
                }
            }
            let max_fish_id = s.fish.iter().map(|f| f.id).max().unwrap_or(0);
            simulation::fish::set_fish_id_counter(max_fish_id + 1);
            let max_egg_id = s.ecosystem.eggs.iter().map(|e| e.id).max().unwrap_or(0);
            simulation::ecosystem::set_egg_id_counter(max_egg_id + 1);
            s
        }
        _ => SimulationState::new(),
    };
    Ok((state, conn))
}

#[tauri::command]
fn list_tanks(active_tank: tauri::State<'_, Mutex<String>>) -> Vec<serde_json::Value> {
    let dir = get_db_dir();
    let active = active_tank.lock().unwrap().clone();
    let mut tanks = Vec::new();

    // The default tank (deeptank.db)
    let default_path = dir.join("deeptank.db");
    if default_path.exists() {
        tanks.push(serde_json::json!({
            "name": "My Aquarium",
            "active": active == "My Aquarium",
        }));
    }

    // Named tanks (deeptank_*.db)
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let fname = entry.file_name().to_string_lossy().to_string();
            if fname.starts_with("deeptank_") && fname.ends_with(".db") && fname != "deeptank.db" {
                // Extract name from filename
                let slug = &fname["deeptank_".len()..fname.len() - 3];
                // Reconstruct name: replace dashes with spaces and capitalize each word
                let name: String = slug.split('-')
                    .filter(|w| !w.is_empty())
                    .map(|w| {
                        let mut chars = w.chars();
                        match chars.next() {
                            Some(c) => format!("{}{}", c.to_uppercase(), chars.as_str()),
                            None => String::new(),
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                // Skip if it matches the default already added
                if name.to_lowercase() == "my aquarium" { continue; }
                tanks.push(serde_json::json!({
                    "name": name,
                    "active": active == name,
                }));
            }
        }
    }
    tanks
}

#[tauri::command]
fn create_tank(
    app: tauri::AppHandle,
    state: tauri::State<'_, Mutex<SimulationState>>,
    db: tauri::State<'_, Mutex<Option<rusqlite::Connection>>>,
    active_tank: tauri::State<'_, Mutex<String>>,
    name: String,
) -> Result<(), String> {
    let name = name.trim().to_string();
    if name.is_empty() || name.len() > 20 { return Err("Name must be 1-20 characters".to_string()); }

    let new_path = get_tank_db_path(&name);
    if new_path.exists() { return Err("Tank already exists".to_string()); }

    // Save current tank
    {
        let sim = state.lock().unwrap();
        let db_guard = db.lock().unwrap();
        if let Some(ref conn) = *db_guard {
            save_current_state(&sim, conn);
        }
    }

    // Create new tank DB
    let new_conn = persistence::open_db(&new_path).map_err(|e| e.to_string())?;
    persistence::init_schema(&new_conn).map_err(|e| e.to_string())?;

    // Switch to new fresh state atomically
    {
        let mut sim = state.lock().unwrap();
        let mut db_guard = db.lock().unwrap();
        let mut active = active_tank.lock().unwrap();
        *sim = SimulationState::new();
        *db_guard = Some(new_conn);
        *active = name;
    }

    // Reload frontend
    if let Some(w) = app.get_webview_window("main") {
        w.eval("window.location.reload()").ok();
    }
    Ok(())
}

#[tauri::command]
fn switch_tank(
    app: tauri::AppHandle,
    state: tauri::State<'_, Mutex<SimulationState>>,
    db: tauri::State<'_, Mutex<Option<rusqlite::Connection>>>,
    active_tank: tauri::State<'_, Mutex<String>>,
    name: String,
) -> Result<(), String> {
    let current_name = active_tank.lock().unwrap().clone();
    if current_name == name { return Ok(()); }

    // Save current tank
    {
        let sim = state.lock().unwrap();
        let db_guard = db.lock().unwrap();
        if let Some(ref conn) = *db_guard {
            save_current_state(&sim, conn);
        }
    }

    // Determine DB path for target tank
    let target_path = if name == "My Aquarium" {
        get_db_path()
    } else {
        get_tank_db_path(&name)
    };

    if !target_path.exists() {
        return Err(format!("Tank '{}' not found", name));
    }

    // Load new tank
    let (new_state, new_conn) = load_tank_from_db(&target_path)?;

    // Swap all state atomically (hold all locks simultaneously to prevent
    // the sim loop from saving new state to the old DB connection)
    {
        let mut sim = state.lock().unwrap();
        let mut db_guard = db.lock().unwrap();
        let mut active = active_tank.lock().unwrap();
        *sim = new_state;
        *db_guard = Some(new_conn);
        *active = name;
    }

    // Reload frontend
    if let Some(w) = app.get_webview_window("main") {
        w.eval("window.location.reload()").ok();
    }
    Ok(())
}

#[tauri::command]
fn delete_tank(
    active_tank: tauri::State<'_, Mutex<String>>,
    name: String,
) -> Result<(), String> {
    let active = active_tank.lock().unwrap().clone();
    if active == name { return Err("Cannot delete the active tank".to_string()); }
    if name == "My Aquarium" { return Err("Cannot delete the default tank".to_string()); }

    let path = get_tank_db_path(&name);
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn get_active_tank(active_tank: tauri::State<'_, Mutex<String>>) -> String {
    active_tank.lock().unwrap().clone()
}

#[tauri::command]
fn toggle_widget_mode(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("main") {
        if enabled {
            w.set_always_on_top(true).map_err(|e| e.to_string())?;
            w.set_decorations(false).map_err(|e| e.to_string())?;
            w.set_size(tauri::LogicalSize::new(400.0, 300.0)).map_err(|e| e.to_string())?;
            w.set_min_size(None::<tauri::LogicalSize<f64>>).map_err(|e| e.to_string())?;
        } else {
            w.set_always_on_top(false).map_err(|e| e.to_string())?;
            w.set_decorations(true).map_err(|e| e.to_string())?;
            w.set_min_size(Some(tauri::LogicalSize::new(800.0, 600.0))).map_err(|e| e.to_string())?;
            w.set_size(tauri::LogicalSize::new(1200.0, 800.0)).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[tauri::command]
fn get_scenarios() -> Vec<serde_json::Value> {
    simulation::scenarios::all_scenarios().iter().map(|s| {
        serde_json::json!({
            "id": s.id,
            "name": s.name,
            "description": s.description,
            "goals": s.goals.iter().map(|g| g.description()).collect::<Vec<_>>(),
        })
    }).collect()
}

#[tauri::command]
fn start_scenario(
    app: tauri::AppHandle,
    state: tauri::State<'_, Mutex<SimulationState>>,
    db: tauri::State<'_, Mutex<Option<rusqlite::Connection>>>,
    active_tank: tauri::State<'_, Mutex<String>>,
    scenario_id: String,
) -> Result<(), String> {
    let scenarios = simulation::scenarios::all_scenarios();
    let scenario = scenarios.iter().find(|s| s.id == scenario_id)
        .ok_or("Scenario not found")?;

    let tank_name = format!("Scenario: {}", scenario.name);

    // Save current tank first
    {
        let sim = state.lock().unwrap();
        let db_guard = db.lock().unwrap();
        if let Some(ref conn) = *db_guard {
            save_current_state(&sim, conn);
        }
    }

    // Create or switch to scenario tank
    let tank_path = get_tank_db_path(&tank_name);
    let new_conn = persistence::open_db(&tank_path).map_err(|e| e.to_string())?;
    persistence::init_schema(&new_conn).map_err(|e| e.to_string())?;

    // Create fresh state with scenario config overrides
    let mut new_state = SimulationState::new();

    // Apply config overrides
    for (key, val) in &scenario.config_overrides {
        match *key {
            "hunger_rate" => new_state.config.hunger_rate = *val,
            "auto_feed_enabled" => new_state.config.auto_feed_enabled = *val > 0.0,
            "mutation_rate_large" => new_state.config.mutation_rate_large = *val,
            "mutation_rate_small" => new_state.config.mutation_rate_small = *val,
            _ => {}
        }
    }

    // Spawn initial fish
    let (w, h) = (new_state.config.tank_width, new_state.config.tank_height);
    for _ in 0..scenario.initial_fish_count {
        let x = new_state.rng.gen_range(50.0..w - 50.0);
        let y = new_state.rng.gen_range(80.0..h - 80.0);
        let genome = simulation::genome::FishGenome::random(&mut new_state.rng);
        let gid = genome.id;
        new_state.genomes.insert(gid, genome);
        let fish = simulation::fish::Fish::new(gid, x, y, &mut new_state.rng);
        new_state.fish.push(fish);
    }

    // Store active scenario ID in state
    new_state.active_scenario_id = Some(scenario_id);

    {
        let mut sim = state.lock().unwrap();
        let mut db_guard = db.lock().unwrap();
        let mut active = active_tank.lock().unwrap();
        *sim = new_state;
        *db_guard = Some(new_conn);
        *active = tank_name;
    }

    if let Some(w) = app.get_webview_window("main") {
        w.eval("window.location.reload()").ok();
    }
    Ok(())
}

#[tauri::command]
fn get_scenario_progress(state: tauri::State<'_, Mutex<SimulationState>>) -> Option<serde_json::Value> {
    let sim = state.lock().unwrap();
    let scenario_id = sim.active_scenario_id.as_ref()?;
    let scenarios = simulation::scenarios::all_scenarios();
    let scenario = scenarios.iter().find(|s| s.id == *scenario_id)?;

    let population = sim.fish.len() as u32;
    let max_gen = sim.genomes.values().map(|g| g.generation).max().unwrap_or(0);
    let species_count = sim.ecosystem.species.iter().filter(|s| s.extinct_at_tick.is_none()).count() as u32;

    let goal_status = simulation::scenarios::check_goals(
        scenario, population, max_gen, species_count,
        sim.tick, sim.genetic_diversity, &sim.genomes, &sim.fish,
    );

    let all_complete = goal_status.iter().all(|(_, met)| *met);

    Some(serde_json::json!({
        "scenario_id": scenario_id,
        "scenario_name": scenario.name,
        "goals": scenario.goals.iter().enumerate().map(|(i, g)| {
            serde_json::json!({
                "description": g.description(),
                "complete": goal_status.iter().find(|(gi, _)| *gi == i).map(|(_, m)| *m).unwrap_or(false),
            })
        }).collect::<Vec<_>>(),
        "all_complete": all_complete,
    }))
}

#[tauri::command]
fn abandon_scenario(
    state: tauri::State<'_, Mutex<SimulationState>>,
) -> Result<(), String> {
    let mut sim = state.lock().unwrap();
    sim.active_scenario_id = None;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // Open/create database
            let db_path = get_db_path();
            log::info!("Database path: {:?}", db_path);
            let conn = persistence::open_db(&db_path).ok();

            if let Some(ref c) = conn {
                persistence::init_schema(c).ok();
            }

            // Try to load saved state
            let state = if let Some(ref c) = conn {
                match persistence::load_state(c) {
                    Ok(Some((tick, wq, fish, genomes, species, eggs, max_species_id))) => {
                        log::info!("Loaded saved state: tick={}, fish={}, eggs={}", tick, fish.len(), eggs.len());
                        let mut s = SimulationState::new();
                        s.tick = tick;
                        s.ecosystem.water_quality = wq;
                        s.fish = fish;
                        s.genomes = genomes;
                        s.ecosystem.species = species;
                        s.ecosystem.eggs = eggs;
                        s.ecosystem.restore_species_counter(max_species_id + 1);
                        s.ecosystem.restore_speciation_tick(tick);
                        // Load decorations
                        if let Ok(mut stmt) = c.prepare("SELECT id, decoration_type, position_x, position_y, scale, flip_x FROM decorations") {
                            if let Ok(rows) = stmt.query_map([], |row| {
                                Ok(simulation::ecosystem::Decoration {
                                    id: row.get(0)?,
                                    decoration_type: simulation::ecosystem::DecorationType::from_str(&row.get::<_, String>(1)?),
                                    x: row.get(2)?,
                                    y: row.get(3)?,
                                    scale: row.get::<_, f64>(4)? as f32,
                                    flip_x: row.get::<_, i32>(5)? != 0,
                                })
                            }) {
                                for r in rows.flatten() {
                                    s.ecosystem.decorations.push(r);
                                }
                                let max_dec_id = s.ecosystem.decorations.iter().map(|d| d.id).max().unwrap_or(0);
                                s.ecosystem.restore_decoration_counter(max_dec_id + 1);
                                s.ecosystem.recompute_plant_count();
                            }
                        }
                        // Restore ID counters so new IDs don't collide with loaded ones
                        let max_fish_id = s.fish.iter().map(|f| f.id).max().unwrap_or(0);
                        simulation::fish::set_fish_id_counter(max_fish_id + 1);
                        let max_egg_id = s.ecosystem.eggs.iter().map(|e| e.id).max().unwrap_or(0);
                        simulation::ecosystem::set_egg_id_counter(max_egg_id + 1);
                        s
                    }
                    _ => {
                        log::info!("No saved state, starting fresh");
                        SimulationState::new()
                    }
                }
            } else {
                SimulationState::new()
            };

            // Load or init achievements
            let mut achievement_list = achievements::default_achievements();
            if let Some(ref c) = conn {
                // Load unlocked states from DB
                if let Ok(mut stmt) = c.prepare("SELECT id, unlocked_at_tick FROM achievements WHERE unlocked_at_tick IS NOT NULL") {
                    if let Ok(rows) = stmt.query_map([], |row| {
                        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
                    }) {
                        for r in rows.flatten() {
                            if let Some(a) = achievement_list.iter_mut().find(|a| a.id == r.0) {
                                a.unlocked_at_tick = Some(r.1 as u64);
                            }
                        }
                    }
                }
            }

            app.manage(Mutex::new(state));
            app.manage(Mutex::new(conn));
            app.manage(Mutex::new(achievement_list));
            app.manage(Mutex::new("My Aquarium".to_string())); // active tank name

            // Start simulation loop
            let app_handle = app.handle().clone();
            std::thread::spawn(move || {
                let tick_duration = Duration::from_micros(33_333); // 30Hz
                let mut last_save_tick: u64 = 0;
                let mut last_snapshot_tick: u64 = 0;
                let mut last_journal_tick: u64 = 0;
                let mut last_narration_tick: u64 = 0;
                let mut last_achievement_tick: u64 = 0;
                let mut births_since_snapshot: u32 = 0;
                let mut deaths_since_snapshot: u32 = 0;
                let mut slow_accumulator: f32 = 0.0;
                let mut high_wq_streak: u32 = 0;
                // Track events between achievement checks so we don't miss any
                let mut had_birth_since_check = false;
                let mut had_speciation_since_check = false;
                let mut had_extinction_since_check = false;
                let mut had_predation_since_check = false;

                loop {
                    let start = std::time::Instant::now();

                    let (frame, tick, should_save, should_snapshot, should_name_species, should_journal, should_narrate) = {
                        let state = app_handle.state::<Mutex<SimulationState>>();
                        let mut sim = state.lock().unwrap();
                        let multiplier = sim.speed_multiplier;
                        let steps = if multiplier >= 1.0 {
                            multiplier as u32
                        } else {
                            // Slow-motion: accumulate fractional steps
                            slow_accumulator += multiplier;
                            if slow_accumulator >= 1.0 {
                                slow_accumulator -= 1.0;
                                1
                            } else {
                                0
                            }
                        };

                        let mut accumulated_events = Vec::new();
                        let frame = if steps == 0 {
                            // Slow-motion skip: emit current state without stepping
                            Some(sim.build_frame(Vec::new()))
                        } else {
                            let mut last_frame = None;
                            for _ in 0..steps {
                                let f = sim.step();
                                accumulated_events.extend(f.events.clone());
                                last_frame = Some(f);
                            }
                            if let Some(ref mut f) = last_frame {
                                f.events = accumulated_events.clone();
                            }
                            last_frame
                        };

                        let f = frame.as_ref().unwrap();
                        // Count births/deaths from events and track for achievement checks
                        for ev in &f.events {
                            match ev {
                                simulation::ecosystem::SimEvent::Birth { .. } => {
                                    births_since_snapshot += 1;
                                    had_birth_since_check = true;
                                }
                                simulation::ecosystem::SimEvent::Death { .. } => {
                                    deaths_since_snapshot += 1;
                                }
                                simulation::ecosystem::SimEvent::NewSpecies { .. } => {
                                    had_speciation_since_check = true;
                                }
                                simulation::ecosystem::SimEvent::Extinction { .. } => {
                                    had_extinction_since_check = true;
                                }
                                simulation::ecosystem::SimEvent::Predation { .. } => {
                                    had_predation_since_check = true;
                                }
                                _ => {}
                            }
                        }

                        let tick = sim.tick;
                        let save = tick - last_save_tick >= sim.config.auto_save_interval as u64;
                        let snap = tick - last_snapshot_tick >= sim.config.snapshot_interval as u64;
                        let unnamed: Vec<_> = sim.ecosystem.species.iter()
                            .filter(|s| s.name.is_none() && s.extinct_at_tick.is_none())
                            .map(|s| (s.id, s.centroid_hue, s.centroid_speed, s.centroid_size, s.centroid_pattern.clone(), s.member_count))
                            .collect();
                        let journal = tick - last_journal_tick >= 3000 && sim.config.ollama_enabled;
                        let narrate = tick - last_narration_tick >= 1500 && sim.config.ollama_enabled;

                        (frame, tick, save, snap, unnamed, journal, narrate)
                    };

                    if let Some(ref frame) = frame {
                        let _ = app_handle.emit("frame-update", frame);

                        // Persist non-FeedingDrop events to DB
                        if !frame.events.is_empty() {
                            let db_state = app_handle.state::<Mutex<Option<rusqlite::Connection>>>();
                            let db = db_state.lock().unwrap();
                            if let Some(ref conn) = *db {
                                for ev in &frame.events {
                                    let (etype, fish_id, species_id, desc) = match ev {
                                        simulation::ecosystem::SimEvent::Birth { fish_id, genome_id, parent_a, parent_b } => {
                                            ("birth", Some(*fish_id as i64), None::<i64>, format!("Fish #{} born (genome {}) from parents #{}, #{}", fish_id, genome_id, parent_a, parent_b))
                                        }
                                        simulation::ecosystem::SimEvent::Death { fish_id, genome_id, cause, .. } => {
                                            ("death", Some(*fish_id as i64), None, format!("Fish #{} (genome {}) died: {:?}", fish_id, genome_id, cause))
                                        }
                                        simulation::ecosystem::SimEvent::Predation { predator_id, prey_id } => {
                                            ("predation", Some(*prey_id as i64), None, format!("Fish #{} eaten by #{}", prey_id, predator_id))
                                        }
                                        simulation::ecosystem::SimEvent::NewSpecies { species_id } => {
                                            ("new_species", None, Some(*species_id as i64), format!("New species #{} discovered", species_id))
                                        }
                                        simulation::ecosystem::SimEvent::Extinction { species_id } => {
                                            ("extinction", None, Some(*species_id as i64), format!("Species #{} went extinct", species_id))
                                        }
                                        simulation::ecosystem::SimEvent::FeedingDrop { .. } => continue,
                                    };
                                    conn.execute(
                                        "INSERT INTO events (tick, event_type, subject_fish_id, subject_species_id, description) VALUES (?1,?2,?3,?4,?5)",
                                        rusqlite::params![tick as i64, etype, fish_id, species_id, desc],
                                    ).ok();
                                }
                            }
                        }
                    }

                    // Track water quality streak
                    if let Some(ref f) = frame {
                        if f.water_quality > 0.95 {
                            high_wq_streak += 1;
                        } else {
                            high_wq_streak = 0;
                        }
                    }

                    // Achievement checking every 300 ticks
                    if tick - last_achievement_tick >= 300 {
                        last_achievement_tick = tick;
                        let sim_state = app_handle.state::<Mutex<SimulationState>>();
                        let sim = sim_state.lock().unwrap();
                        let ach_state = app_handle.state::<Mutex<Vec<Achievement>>>();
                        let mut achs = ach_state.lock().unwrap();

                        // Use accumulated event flags (not just current frame)
                        let had_birth = had_birth_since_check;
                        let had_speciation = had_speciation_since_check;
                        let had_extinction = had_extinction_since_check;
                        let had_predation = had_predation_since_check;
                        // Reset flags for next check period
                        had_birth_since_check = false;
                        had_speciation_since_check = false;
                        had_extinction_since_check = false;
                        had_predation_since_check = false;

                        let max_gen = sim.genomes.values().map(|g| g.generation).max().unwrap_or(0);
                        let species_count = sim.ecosystem.species.iter().filter(|s| s.extinct_at_tick.is_none()).count() as u32;
                        let population = sim.fish.len() as u32;
                        let wq = sim.ecosystem.water_quality;
                        let carrying_capacity = sim.config.base_carrying_capacity;

                        let (max_aggression, max_speed, max_meals, min_body, max_body) = {
                            let mut ma = 0.0_f32; let mut ms = 0.0_f32; let mut mm = 0_u32;
                            let mut min_b = f32::MAX; let mut max_b = 0.0_f32;
                            for f in &sim.fish {
                                if let Some(g) = sim.genomes.get(&f.genome_id) {
                                    if g.aggression > ma { ma = g.aggression; }
                                    if g.speed > ms { ms = g.speed; }
                                    if g.body_length < min_b { min_b = g.body_length; }
                                    if g.body_length > max_b { max_b = g.body_length; }
                                }
                                if f.meals_eaten > mm { mm = f.meals_eaten; }
                            }
                            (ma, ms, mm, min_b, max_b)
                        };

                        let newly_unlocked = achievements::check_achievements(
                            &mut achs, tick, population, max_gen, species_count,
                            wq, high_wq_streak, had_birth, had_speciation,
                            had_extinction, had_predation, max_aggression, max_speed,
                            max_meals, min_body, max_body, carrying_capacity,
                        );

                        // Persist + emit toasts for new achievements
                        if !newly_unlocked.is_empty() {
                            let db_state = app_handle.state::<Mutex<Option<rusqlite::Connection>>>();
                            let db = db_state.lock().unwrap();
                            if let Some(ref conn) = *db {
                                for a in achs.iter() {
                                    if newly_unlocked.contains(&a.name) {
                                        conn.execute(
                                            "INSERT OR REPLACE INTO achievements (id, name, description, unlocked_at_tick, unlocked_at) VALUES (?1,?2,?3,?4,datetime('now'))",
                                            rusqlite::params![a.id, a.name, a.description, a.unlocked_at_tick.unwrap_or(0) as i64],
                                        ).ok();
                                    }
                                }
                            }
                            // Emit achievement events to frontend
                            for name in &newly_unlocked {
                                let _ = app_handle.emit("achievement-unlocked", name.clone());
                            }
                        }
                    }

                    // Auto-save
                    // Lock order: always sim before db (matches Tauri command handlers)
                    if should_save {
                        last_save_tick = tick;
                        let sim_state = app_handle.state::<Mutex<SimulationState>>();
                        let db_state = app_handle.state::<Mutex<Option<rusqlite::Connection>>>();
                        let sim = sim_state.lock().unwrap();
                        let db = db_state.lock().unwrap();
                        if let Some(ref conn) = *db {
                            if let Err(e) = persistence::save_state(
                                conn, sim.tick, sim.ecosystem.water_quality,
                                &sim.fish, &sim.genomes, &sim.ecosystem.species, &sim.ecosystem.eggs,
                            ) {
                                log::error!("Auto-save failed: {}", e);
                            }
                        }
                    }

                    // Population snapshot
                    if should_snapshot {
                        last_snapshot_tick = tick;
                        let sim_state = app_handle.state::<Mutex<SimulationState>>();
                        let db_state = app_handle.state::<Mutex<Option<rusqlite::Connection>>>();
                        let sim = sim_state.lock().unwrap();
                        let db = db_state.lock().unwrap();
                        if let Some(ref conn) = *db {
                            let sc = sim.ecosystem.species.iter().filter(|s| s.extinct_at_tick.is_none()).count() as u32;
                            persistence::save_snapshot(
                                conn, sim.tick, sim.fish.len() as u32, sc,
                                sim.ecosystem.water_quality, &sim.genomes, &sim.fish,
                                births_since_snapshot, deaths_since_snapshot,
                                sim.genetic_diversity,
                            ).ok();
                            births_since_snapshot = 0;
                            deaths_since_snapshot = 0;
                            // Also save per-species snapshot
                            persistence::save_species_snapshot(conn, sim.tick, &sim.ecosystem.species).ok();
                        }
                    }

                    // Ollama species naming (async, non-blocking)
                    if !should_name_species.is_empty() {
                        let sim_state = app_handle.state::<Mutex<SimulationState>>();
                        let sim = sim_state.lock().unwrap();
                        let ollama_url = sim.config.ollama_url.clone();
                        let ollama_model = sim.config.ollama_model.clone();
                        let ollama_enabled = sim.config.ollama_enabled;
                        drop(sim);

                        for (sp_id, hue, speed, size, pattern, count) in should_name_species {
                            let url = ollama_url.clone();
                            let model = ollama_model.clone();
                            let app_h = app_handle.clone();

                            if ollama_enabled {
                                tokio::spawn(async move {
                                    let result = ollama::name_species(&url, &model, hue, speed, size, &pattern, count, 0).await;
                                    let (name, desc) = result.unwrap_or_else(|| {
                                        (ollama::fallback_species_name(hue, speed, &pattern, size), String::new())
                                    });
                                    let app_h2 = app_h.clone();
                                    let _ = tokio::task::spawn_blocking(move || {
                                        let state = app_h2.state::<Mutex<SimulationState>>();
                                        let mut sim = state.lock().unwrap();
                                        if let Some(sp) = sim.ecosystem.species.iter_mut().find(|s| s.id == sp_id) {
                                            sp.name = Some(name);
                                            sp.description = Some(desc);
                                        }
                                    }).await;
                                });
                            } else {
                                // Fallback naming
                                let state = app_handle.state::<Mutex<SimulationState>>();
                                let mut sim = state.lock().unwrap();
                                if let Some(sp) = sim.ecosystem.species.iter_mut().find(|s| s.id == sp_id) {
                                    sp.name = Some(ollama::fallback_species_name(hue, speed, &pattern, size));
                                }
                            }
                        }
                    }

                    // Narration generation (shorter, more frequent than journal)
                    if should_narrate {
                        last_narration_tick = tick;
                        let sim_state = app_handle.state::<Mutex<SimulationState>>();
                        let sim = sim_state.lock().unwrap();
                        let url = sim.config.ollama_url.clone();
                        let model = sim.config.ollama_model.clone();
                        let pop = sim.fish.len() as u32;
                        let species_count = sim.ecosystem.species.iter().filter(|s| s.extinct_at_tick.is_none()).count() as u32;
                        let wq = sim.ecosystem.water_quality;
                        let latest_event = sim.ecosystem.species.iter()
                            .filter(|s| s.extinct_at_tick.is_none())
                            .last()
                            .map(|s| format!("Species '{}' has {} members", s.name.as_deref().unwrap_or("unnamed"), s.member_count))
                            .unwrap_or_else(|| "Life continues in the tank".to_string());
                        drop(sim);

                        let app_h = app_handle.clone();
                        tokio::spawn(async move {
                            if let Some(text) = ollama::generate_narration(&url, &model, pop, species_count, wq, &latest_event).await {
                                let _ = app_h.emit("narration", text);
                            }
                        });
                    }

                    // Journal entry generation
                    if should_journal {
                        last_journal_tick = tick;
                        let sim_state = app_handle.state::<Mutex<SimulationState>>();
                        let sim = sim_state.lock().unwrap();
                        let url = sim.config.ollama_url.clone();
                        let model = sim.config.ollama_model.clone();
                        let pop = sim.fish.len() as u32;
                        let wq = sim.ecosystem.water_quality;
                        let species_summary: String = sim.ecosystem.species.iter()
                            .filter(|s| s.extinct_at_tick.is_none())
                            .map(|s| format!("\"{}\" ({} members)", s.name.as_deref().unwrap_or("unnamed"), s.member_count))
                            .collect::<Vec<_>>()
                            .join(", ");
                        let current_tick = sim.tick;
                        drop(sim);

                        let app_h = app_handle.clone();
                        tokio::spawn(async move {
                            if let Some(entry) = ollama::generate_journal_entry(&url, &model, current_tick, pop, wq, &species_summary).await {
                                let app_h2 = app_h.clone();
                                let _ = tokio::task::spawn_blocking(move || {
                                    let db_state = app_h2.state::<Mutex<Option<rusqlite::Connection>>>();
                                    let db = db_state.lock().unwrap();
                                    if let Some(ref conn) = *db {
                                        conn.execute(
                                            "INSERT INTO journal_entries (tick, entry_text) VALUES (?1, ?2)",
                                            rusqlite::params![current_tick as i64, entry],
                                        ).ok();
                                    }
                                }).await;
                            }
                        });
                    }

                    let elapsed = start.elapsed();
                    if elapsed < tick_duration {
                        std::thread::sleep(tick_duration - elapsed);
                    }
                }
            });

            // System tray
            let show_i = MenuItem::with_id(app, "show", "Show DeepTank", true, None::<&str>)?;
            let pause_i = MenuItem::with_id(app, "pause_toggle", "Pause", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &pause_i, &quit_i])?;

            let tray_icon = app.default_window_icon().cloned()
                .unwrap_or_else(|| tauri::image::Image::new(&[], 1, 1));
            let _tray = TrayIconBuilder::new()
                .icon(tray_icon)
                .tooltip("DeepTank")
                .menu(&menu)
                .on_menu_event(move |app, event| {
                    match event.id.as_ref() {
                        "show" => {
                            if let Some(w) = app.get_webview_window("main") {
                                w.show().ok();
                                w.set_focus().ok();
                            }
                        }
                        "pause_toggle" => {
                            let state = app.state::<Mutex<SimulationState>>();
                            let mut sim = state.lock().unwrap();
                            sim.paused = !sim.paused;
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .build(app)?;

            log::info!("DeepTank initialized with simulation loop");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            pause,
            resume,
            set_speed,
            feed,
            step_forward,
            select_fish,
            tap_glass,
            trigger_event,
            breed_fish,
            get_breed_preview,
            get_genome,
            get_all_genomes,
            get_species_list,
            get_species_history,
            get_fish_detail,
            name_fish,
            toggle_favorite,
            get_favorites,
            update_tank_size,
            get_snapshots,
            get_all_snapshots,
            get_species_snapshots,
            get_events,
            get_journal_entries,
            get_config,
            update_config,
            add_decoration,
            remove_decoration,
            get_decorations,
            get_achievements,
            get_lineage,
            export_tank,
            import_tank,
            list_tanks,
            create_tank,
            switch_tank,
            delete_tank,
            get_active_tank,
            get_scenarios,
            start_scenario,
            get_scenario_progress,
            abandon_scenario,
            toggle_widget_mode,
        ])
        .run(tauri::generate_context!())
        .expect("error while running DeepTank");
}
