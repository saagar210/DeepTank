mod simulation;

use simulation::SimulationState;
use simulation::achievements::{self, Achievement};
use simulation::genome::FishGenome;
use simulation::persistence;
use simulation::ollama;
use std::sync::Mutex;
use std::time::Duration;
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
        "genome": genome,
        "species_name": species_name,
    }))
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
        "SELECT tick, population, species_count, water_quality, avg_hue, avg_speed, avg_size, avg_aggression
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
        "SELECT tick, population, species_count, water_quality, avg_hue, avg_speed, avg_size, avg_aggression
         FROM population_snapshots ORDER BY tick ASC"
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
        "bubble_rate" => if let Some(v) = value.as_f64() { c.bubble_rate = v as f32; },
        "current_strength" => if let Some(v) = value.as_f64() { c.current_strength = v as f32; },
        "auto_feed_enabled" => if let Some(v) = value.as_bool() { c.auto_feed_enabled = v; },
        "auto_feed_interval" => if let Some(v) = value.as_f64() { c.auto_feed_interval = v as u32; },
        "auto_feed_amount" => if let Some(v) = value.as_f64() { c.auto_feed_amount = v as u32; },
        "ollama_enabled" => if let Some(v) = value.as_bool() { c.ollama_enabled = v; },
        "ollama_url" => if let Some(v) = value.as_str() { c.ollama_url = v.to_string(); },
        "ollama_model" => if let Some(v) = value.as_str() { c.ollama_model = v.to_string(); },
        "master_volume" => if let Some(v) = value.as_f64() { c.master_volume = v as f32; },
        "ambient_enabled" => if let Some(v) = value.as_bool() { c.ambient_enabled = v; },
        "event_sounds_enabled" => if let Some(v) = value.as_bool() { c.event_sounds_enabled = v; },
        "theme" => if let Some(v) = value.as_str() { c.theme = v.to_string(); },
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
    let query = if let Some(ref etype) = event_type {
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
        results
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
        results
    };
    query
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
            persistence::save_state(conn, sim.tick, sim.ecosystem.water_quality, &sim.fish, &sim.genomes, &sim.ecosystem.species)
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

fn get_db_path() -> std::path::PathBuf {
    let mut path = dirs::data_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    path.push("DeepTank");
    std::fs::create_dir_all(&path).ok();
    path.push("deeptank.db");
    path
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
                    Ok(Some((tick, wq, fish, genomes, species, max_species_id))) => {
                        log::info!("Loaded saved state: tick={}, fish={}", tick, fish.len());
                        let mut s = SimulationState::new();
                        s.tick = tick;
                        s.ecosystem.water_quality = wq;
                        s.fish = fish;
                        s.genomes = genomes;
                        s.ecosystem.species = species;
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

            // Start simulation loop
            let app_handle = app.handle().clone();
            std::thread::spawn(move || {
                let tick_duration = Duration::from_micros(33_333); // 30Hz
                let mut last_save_tick: u64 = 0;
                let mut last_snapshot_tick: u64 = 0;
                let mut last_journal_tick: u64 = 0;
                let mut last_achievement_tick: u64 = 0;
                let mut births_since_snapshot: u32 = 0;
                let mut deaths_since_snapshot: u32 = 0;
                let mut slow_accumulator: f32 = 0.0;
                let mut high_wq_streak: u32 = 0;

                loop {
                    let start = std::time::Instant::now();

                    let (frame, tick, should_save, should_snapshot, should_name_species, should_journal) = {
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
                        // Count births/deaths from events
                        for ev in &f.events {
                            if let simulation::ecosystem::SimEvent::Birth { .. } = ev {
                                births_since_snapshot += 1;
                            }
                            if let simulation::ecosystem::SimEvent::Death { .. } = ev {
                                deaths_since_snapshot += 1;
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

                        (frame, tick, save, snap, unnamed, journal)
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
                                        simulation::ecosystem::SimEvent::Death { fish_id, genome_id, cause } => {
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

                        // Gather stats
                        let had_birth = frame.as_ref().map_or(false, |f| f.events.iter().any(|e| matches!(e, simulation::ecosystem::SimEvent::Birth { .. })));
                        let had_speciation = frame.as_ref().map_or(false, |f| f.events.iter().any(|e| matches!(e, simulation::ecosystem::SimEvent::NewSpecies { .. })));
                        let had_extinction = frame.as_ref().map_or(false, |f| f.events.iter().any(|e| matches!(e, simulation::ecosystem::SimEvent::Extinction { .. })));
                        let had_predation = frame.as_ref().map_or(false, |f| f.events.iter().any(|e| matches!(e, simulation::ecosystem::SimEvent::Predation { .. })));

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
                    if should_save {
                        last_save_tick = tick;
                        let db_state = app_handle.state::<Mutex<Option<rusqlite::Connection>>>();
                        let sim_state = app_handle.state::<Mutex<SimulationState>>();
                        let db = db_state.lock().unwrap();
                        let sim = sim_state.lock().unwrap();
                        if let Some(ref conn) = *db {
                            if let Err(e) = persistence::save_state(
                                conn, sim.tick, sim.ecosystem.water_quality,
                                &sim.fish, &sim.genomes, &sim.ecosystem.species,
                            ) {
                                log::error!("Auto-save failed: {}", e);
                            }
                        }
                    }

                    // Population snapshot
                    if should_snapshot {
                        last_snapshot_tick = tick;
                        let db_state = app_handle.state::<Mutex<Option<rusqlite::Connection>>>();
                        let sim_state = app_handle.state::<Mutex<SimulationState>>();
                        let db = db_state.lock().unwrap();
                        let sim = sim_state.lock().unwrap();
                        if let Some(ref conn) = *db {
                            let sc = sim.ecosystem.species.iter().filter(|s| s.extinct_at_tick.is_none()).count() as u32;
                            persistence::save_snapshot(
                                conn, sim.tick, sim.fish.len() as u32, sc,
                                sim.ecosystem.water_quality, &sim.genomes, &sim.fish,
                                births_since_snapshot, deaths_since_snapshot,
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
            get_genome,
            get_all_genomes,
            get_species_list,
            get_species_history,
            get_fish_detail,
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running DeepTank");
}
