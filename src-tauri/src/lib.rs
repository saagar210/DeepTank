mod simulation;

use simulation::SimulationState;
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
fn feed(state: tauri::State<'_, Mutex<SimulationState>>, x: f32, y: f32) {
    state.lock().unwrap().ecosystem.drop_food(x, y);
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
        _ => {}
    }
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

            app.manage(Mutex::new(state));
            app.manage(Mutex::new(conn));

            // Start simulation loop
            let app_handle = app.handle().clone();
            std::thread::spawn(move || {
                let tick_duration = Duration::from_micros(33_333); // 30Hz
                let mut last_save_tick: u64 = 0;
                let mut last_snapshot_tick: u64 = 0;
                let mut last_journal_tick: u64 = 0;
                let mut births_since_snapshot: u32 = 0;
                let mut deaths_since_snapshot: u32 = 0;
                let mut slow_accumulator: f32 = 0.0;

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
            select_fish,
            get_genome,
            get_all_genomes,
            get_species_list,
            get_fish_detail,
            update_tank_size,
            get_snapshots,
            get_journal_entries,
            get_config,
            update_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running DeepTank");
}
