use crate::simulation::ecosystem::Species;
use crate::simulation::fish::{BehaviorState, Fish};
use crate::simulation::genome::{FishGenome, PatternGene, Sex};
use rusqlite::{params, Connection, Result};
use std::collections::HashMap;
use std::path::Path;

pub fn open_db(path: &Path) -> Result<Connection> {
    let conn = Connection::open(path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
    Ok(conn)
}

pub fn init_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS schema_version (version INTEGER PRIMARY KEY);
        INSERT OR IGNORE INTO schema_version VALUES (1);

        CREATE TABLE IF NOT EXISTS aquarium (
            id INTEGER PRIMARY KEY DEFAULT 1,
            tick_count INTEGER NOT NULL DEFAULT 0,
            water_quality REAL NOT NULL DEFAULT 1.0,
            last_saved_at TEXT NOT NULL DEFAULT (datetime('now')),
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        INSERT OR IGNORE INTO aquarium (id) VALUES (1);

        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS genomes (
            id INTEGER PRIMARY KEY,
            generation INTEGER NOT NULL,
            parent_a INTEGER,
            parent_b INTEGER,
            sex TEXT NOT NULL,
            base_hue REAL NOT NULL,
            saturation REAL NOT NULL,
            lightness REAL NOT NULL,
            body_length REAL NOT NULL,
            body_width REAL NOT NULL,
            tail_size REAL NOT NULL,
            dorsal_fin_size REAL NOT NULL,
            pectoral_fin_size REAL NOT NULL,
            pattern_type TEXT NOT NULL,
            pattern_data TEXT,
            pattern_intensity REAL NOT NULL,
            pattern_color_offset REAL NOT NULL,
            eye_size REAL NOT NULL,
            speed REAL NOT NULL,
            aggression REAL NOT NULL,
            school_affinity REAL NOT NULL,
            curiosity REAL NOT NULL,
            boldness REAL NOT NULL,
            metabolism REAL NOT NULL,
            fertility REAL NOT NULL,
            lifespan_factor REAL NOT NULL,
            maturity_age REAL NOT NULL,
            born_at_tick INTEGER NOT NULL DEFAULT 0,
            species_id INTEGER
        );

        CREATE TABLE IF NOT EXISTS fish (
            id INTEGER PRIMARY KEY,
            genome_id INTEGER NOT NULL,
            position_x REAL NOT NULL,
            position_y REAL NOT NULL,
            position_z REAL NOT NULL,
            velocity_x REAL NOT NULL,
            velocity_y REAL NOT NULL,
            heading REAL NOT NULL,
            age INTEGER NOT NULL DEFAULT 0,
            hunger REAL NOT NULL DEFAULT 0.3,
            health REAL NOT NULL DEFAULT 1.0,
            energy REAL NOT NULL DEFAULT 1.0,
            behavior_state TEXT NOT NULL DEFAULT 'swimming',
            meals_eaten INTEGER NOT NULL DEFAULT 0,
            last_reproduced_tick INTEGER DEFAULT NULL,
            is_alive INTEGER NOT NULL DEFAULT 1
        );

        CREATE TABLE IF NOT EXISTS species (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT,
            description TEXT,
            discovered_at_tick INTEGER NOT NULL,
            extinct_at_tick INTEGER,
            centroid_hue REAL,
            centroid_speed REAL,
            centroid_size REAL,
            centroid_pattern TEXT,
            member_count_at_discovery INTEGER
        );

        CREATE TABLE IF NOT EXISTS population_snapshots (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            tick INTEGER NOT NULL,
            timestamp TEXT NOT NULL DEFAULT (datetime('now')),
            population INTEGER NOT NULL,
            species_count INTEGER NOT NULL,
            water_quality REAL NOT NULL,
            avg_hue REAL,
            avg_speed REAL,
            avg_size REAL,
            avg_aggression REAL,
            avg_metabolism REAL,
            births_since_last INTEGER NOT NULL DEFAULT 0,
            deaths_since_last INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            tick INTEGER NOT NULL,
            event_type TEXT NOT NULL,
            subject_fish_id INTEGER,
            subject_species_id INTEGER,
            description TEXT NOT NULL,
            timestamp TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS journal_entries (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            tick INTEGER NOT NULL,
            entry_text TEXT NOT NULL,
            timestamp TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS decorations (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            decoration_type TEXT NOT NULL,
            position_x REAL NOT NULL,
            position_y REAL NOT NULL,
            scale REAL NOT NULL DEFAULT 1.0,
            flip_x INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS species_snapshots (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            tick INTEGER NOT NULL,
            species_id INTEGER NOT NULL,
            species_name TEXT,
            population INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS achievements (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT NOT NULL,
            unlocked_at_tick INTEGER,
            unlocked_at TEXT
        );

        -- Migration: add disease_resistance column if missing
        -- SQLite doesn't have IF NOT EXISTS for ALTER TABLE, so check column existence via pragma
        ",
    )?;
    // Add disease_resistance column if not present
    let has_disease_col: bool = conn.prepare("SELECT disease_resistance FROM genomes LIMIT 0")
        .is_ok();
    if !has_disease_col {
        conn.execute_batch("ALTER TABLE genomes ADD COLUMN disease_resistance REAL NOT NULL DEFAULT 0.5;").ok();
    }
    conn.execute_batch("
        CREATE INDEX IF NOT EXISTS idx_genomes_generation ON genomes(generation);
        CREATE INDEX IF NOT EXISTS idx_snapshots_tick ON population_snapshots(tick);
        CREATE INDEX IF NOT EXISTS idx_events_type ON events(event_type);
        CREATE INDEX IF NOT EXISTS idx_events_tick ON events(tick);
        ",
    )?;
    Ok(())
}

pub fn save_state(
    conn: &Connection,
    tick: u64,
    water_quality: f32,
    fish: &[Fish],
    genomes: &HashMap<u32, FishGenome>,
    species: &[Species],
) -> Result<()> {
    let tx = conn.unchecked_transaction()?;

    // Update aquarium
    tx.execute(
        "UPDATE aquarium SET tick_count = ?1, water_quality = ?2, last_saved_at = datetime('now') WHERE id = 1",
        params![tick as i64, water_quality],
    )?;

    // Upsert genomes
    for g in genomes.values() {
        let sex_str = match g.sex { Sex::Male => "male", Sex::Female => "female" };
        let (pat_type, pat_data) = serialize_pattern(&g.pattern);
        tx.execute(
            "INSERT OR REPLACE INTO genomes (id, generation, parent_a, parent_b, sex,
                base_hue, saturation, lightness, body_length, body_width, tail_size,
                dorsal_fin_size, pectoral_fin_size, pattern_type, pattern_data,
                pattern_intensity, pattern_color_offset, eye_size, speed, aggression,
                school_affinity, curiosity, boldness, metabolism, fertility,
                lifespan_factor, maturity_age, born_at_tick, disease_resistance)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25,?26,?27,?28,?29)",
            params![
                g.id, g.generation, g.parent_a, g.parent_b, sex_str,
                g.base_hue, g.saturation, g.lightness, g.body_length, g.body_width, g.tail_size,
                g.dorsal_fin_size, g.pectoral_fin_size, pat_type, pat_data,
                g.pattern_intensity, g.pattern_color_offset, g.eye_size, g.speed, g.aggression,
                g.school_affinity, g.curiosity, g.boldness, g.metabolism, g.fertility,
                g.lifespan_factor, g.maturity_age, 0i64, g.disease_resistance,
            ],
        )?;
    }

    // Replace fish table
    tx.execute("DELETE FROM fish", [])?;
    for f in fish {
        tx.execute(
            "INSERT INTO fish (id, genome_id, position_x, position_y, position_z,
                velocity_x, velocity_y, heading, age, hunger, health, energy,
                behavior_state, meals_eaten, last_reproduced_tick, is_alive)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)",
            params![
                f.id, f.genome_id, f.x, f.y, f.z, f.vx, f.vy, f.heading,
                f.age, f.hunger, f.health, f.energy, f.behavior.as_str(),
                f.meals_eaten, f.last_reproduced_tick.map(|t| t as i64), f.is_alive as i32,
            ],
        )?;
    }

    // Upsert species
    for s in species {
        tx.execute(
            "INSERT OR REPLACE INTO species (id, name, description, discovered_at_tick,
                extinct_at_tick, centroid_hue, centroid_speed, centroid_size,
                centroid_pattern, member_count_at_discovery)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![
                s.id, s.name, s.description, s.discovered_at_tick as i64,
                s.extinct_at_tick.map(|t| t as i64), s.centroid_hue, s.centroid_speed,
                s.centroid_size, s.centroid_pattern, s.member_count,
            ],
        )?;
    }

    tx.commit()?;
    Ok(())
}

/// Returns (tick, water_quality, fish, genomes, species, max_species_id)
pub fn load_state(
    conn: &Connection,
) -> Result<Option<(u64, f32, Vec<Fish>, HashMap<u32, FishGenome>, Vec<Species>, u32)>> {
    // Check if there's saved state
    let tick: i64 = conn.query_row("SELECT tick_count FROM aquarium WHERE id = 1", [], |row| row.get(0))?;
    if tick == 0 {
        return Ok(None);
    }

    let water_quality: f64 = conn.query_row("SELECT water_quality FROM aquarium WHERE id = 1", [], |row| row.get(0))?;

    // Load genomes
    let mut genomes = HashMap::new();
    let mut stmt = conn.prepare(
        "SELECT id, generation, parent_a, parent_b, sex, base_hue, saturation, lightness,
                body_length, body_width, tail_size, dorsal_fin_size, pectoral_fin_size,
                pattern_type, pattern_data, pattern_intensity, pattern_color_offset, eye_size,
                speed, aggression, school_affinity, curiosity, boldness, metabolism, fertility,
                lifespan_factor, maturity_age, disease_resistance FROM genomes"
    )?;
    let genome_rows = stmt.query_map([], |row| {
        let sex_str: String = row.get(4)?;
        let pat_type: String = row.get(13)?;
        let pat_data: Option<String> = row.get(14)?;
        Ok(FishGenome {
            id: row.get(0)?,
            generation: row.get(1)?,
            parent_a: row.get(2)?,
            parent_b: row.get(3)?,
            sex: if sex_str == "male" { Sex::Male } else { Sex::Female },
            base_hue: row.get(5)?,
            saturation: row.get(6)?,
            lightness: row.get(7)?,
            body_length: row.get(8)?,
            body_width: row.get(9)?,
            tail_size: row.get(10)?,
            dorsal_fin_size: row.get(11)?,
            pectoral_fin_size: row.get(12)?,
            pattern: deserialize_pattern(&pat_type, pat_data.as_deref()),
            pattern_intensity: row.get(15)?,
            pattern_color_offset: row.get(16)?,
            eye_size: row.get(17)?,
            speed: row.get(18)?,
            aggression: row.get(19)?,
            school_affinity: row.get(20)?,
            curiosity: row.get(21)?,
            boldness: row.get(22)?,
            metabolism: row.get(23)?,
            fertility: row.get(24)?,
            lifespan_factor: row.get(25)?,
            maturity_age: row.get(26)?,
            disease_resistance: row.get::<_, f64>(27).unwrap_or(0.5) as f32,
        })
    })?;
    for g in genome_rows {
        let genome = g?;
        genomes.insert(genome.id, genome);
    }

    // Load fish
    let mut fish = Vec::new();
    let mut stmt = conn.prepare(
        "SELECT id, genome_id, position_x, position_y, position_z, velocity_x, velocity_y,
                heading, age, hunger, health, energy, behavior_state, meals_eaten,
                last_reproduced_tick, is_alive FROM fish WHERE is_alive = 1"
    )?;
    let fish_rows = stmt.query_map([], |row| {
        let beh_str: String = row.get(12)?;
        let last_repro: Option<i64> = row.get(14)?;
        Ok(Fish {
            id: row.get(0)?,
            genome_id: row.get(1)?,
            x: row.get(2)?,
            y: row.get(3)?,
            z: row.get(4)?,
            vx: row.get(5)?,
            vy: row.get(6)?,
            heading: row.get(7)?,
            age: row.get(8)?,
            hunger: row.get(9)?,
            health: row.get(10)?,
            energy: row.get(11)?,
            behavior: match beh_str.as_str() {
                "foraging" => BehaviorState::Foraging,
                "fleeing" => BehaviorState::Fleeing,
                "satiated" => BehaviorState::Satiated,
                "courting" => BehaviorState::Courting,
                "resting" => BehaviorState::Resting,
                "dying" => BehaviorState::Dying,
                _ => BehaviorState::Swimming,
            },
            meals_eaten: row.get(13)?,
            last_reproduced_tick: last_repro.map(|t| t as u64),
            is_alive: true,
            prev_force_x: 0.0,
            prev_force_y: 0.0,
            satiated_timer: 0,
            courting_partner: None,
            courting_timer: 0,
            dying_timer: 0,
            starvation_ticks: 0,
            fleeing_from: None,
            killed_by_predator: false,
            is_infected: false,
            infection_timer: 0,
            recovery_timer: 0,
        })
    })?;
    for f in fish_rows {
        fish.push(f?);
    }

    // Load species
    let mut species = Vec::new();
    let mut stmt = conn.prepare(
        "SELECT id, name, description, discovered_at_tick, extinct_at_tick,
                centroid_hue, centroid_speed, centroid_size, centroid_pattern,
                member_count_at_discovery FROM species"
    )?;
    let species_rows = stmt.query_map([], |row| {
        let extinct: Option<i64> = row.get(4)?;
        Ok(Species {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            discovered_at_tick: row.get::<_, i64>(3)? as u64,
            extinct_at_tick: extinct.map(|t| t as u64),
            centroid_hue: row.get(5)?,
            centroid_speed: row.get(6)?,
            centroid_size: row.get(7)?,
            centroid_pattern: row.get(8)?,
            member_count: row.get::<_, u32>(9).unwrap_or(0),
            member_genome_ids: Vec::new(),
        })
    })?;
    for s in species_rows {
        species.push(s?);
    }

    // Update ID counters
    let max_genome_id: u32 = genomes.keys().copied().max().unwrap_or(0);
    let max_fish_id: u32 = fish.iter().map(|f| f.id).max().unwrap_or(0);
    let max_species_id: u32 = species.iter().map(|s| s.id).max().unwrap_or(0);
    crate::simulation::genome::set_genome_id_counter(max_genome_id + 1);
    crate::simulation::fish::set_fish_id_counter(max_fish_id + 1);

    // Rebuild member_genome_ids from living fish
    let fish_genome_ids: Vec<(u32, u32)> = fish.iter().map(|f| (f.id, f.genome_id)).collect();
    for sp in &mut species {
        if sp.extinct_at_tick.is_some() { continue; }
        sp.member_genome_ids = fish_genome_ids.iter()
            .filter_map(|&(_fid, gid)| {
                if let Some(g) = genomes.get(&gid) {
                    // Check if genome is close enough to this species centroid
                    let hue_diff = (g.base_hue - sp.centroid_hue).abs().min(360.0 - (g.base_hue - sp.centroid_hue).abs());
                    let speed_diff = (g.speed - sp.centroid_speed).abs();
                    let size_diff = (g.body_length - sp.centroid_size).abs();
                    if hue_diff < 30.0 && speed_diff < 0.5 && size_diff < 0.5 {
                        return Some(gid);
                    }
                }
                None
            })
            .collect();
        sp.member_count = sp.member_genome_ids.len() as u32;
    }

    Ok(Some((tick as u64, water_quality as f32, fish, genomes, species, max_species_id)))
}

pub fn save_snapshot(
    conn: &Connection,
    tick: u64,
    population: u32,
    species_count: u32,
    water_quality: f32,
    genomes: &HashMap<u32, FishGenome>,
    fish: &[Fish],
    births: u32,
    deaths: u32,
) -> Result<()> {
    let (avg_hue, avg_speed, avg_size, avg_aggression, avg_metabolism) = if !fish.is_empty() {
        let mut sin_sum = 0.0_f32; let mut cos_sum = 0.0_f32;
        let mut sp = 0.0_f32; let mut sz = 0.0_f32; let mut ag = 0.0_f32; let mut met = 0.0_f32;
        let mut count = 0_u32;
        for f in fish {
            if let Some(g) = genomes.get(&f.genome_id) {
                let rad = g.base_hue.to_radians();
                sin_sum += rad.sin();
                cos_sum += rad.cos();
                sp += g.speed; sz += g.body_length; ag += g.aggression; met += g.metabolism;
                count += 1;
            }
        }
        let n = count.max(1) as f32;
        let avg_h = sin_sum.atan2(cos_sum).to_degrees().rem_euclid(360.0);
        (avg_h, sp/n, sz/n, ag/n, met/n)
    } else {
        (0.0, 0.0, 0.0, 0.0, 0.0)
    };

    conn.execute(
        "INSERT INTO population_snapshots (tick, population, species_count, water_quality,
            avg_hue, avg_speed, avg_size, avg_aggression, avg_metabolism,
            births_since_last, deaths_since_last)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
        params![
            tick as i64, population, species_count, water_quality,
            avg_hue, avg_speed, avg_size, avg_aggression, avg_metabolism,
            births, deaths,
        ],
    )?;
    Ok(())
}

pub fn save_species_snapshot(conn: &Connection, tick: u64, species: &[Species]) -> Result<()> {
    for s in species {
        if s.extinct_at_tick.is_some() { continue; }
        conn.execute(
            "INSERT INTO species_snapshots (tick, species_id, species_name, population) VALUES (?1,?2,?3,?4)",
            params![tick as i64, s.id, s.name.as_deref().unwrap_or("unnamed"), s.member_count],
        )?;
    }
    Ok(())
}

pub fn get_species_snapshots(conn: &Connection) -> Vec<(i64, u32, String, u32)> {
    let mut results = Vec::new();
    if let Ok(mut stmt) = conn.prepare(
        "SELECT tick, species_id, species_name, population FROM species_snapshots ORDER BY tick ASC LIMIT 5000"
    ) {
        if let Ok(rows) = stmt.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, u32>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, u32>(3)?,
            ))
        }) {
            for r in rows.flatten() { results.push(r); }
        }
    }
    results
}

fn serialize_pattern(p: &PatternGene) -> (String, Option<String>) {
    match p {
        PatternGene::Solid => ("solid".to_string(), None),
        PatternGene::Striped { angle } => ("striped".to_string(), Some(format!("{{\"angle\":{}}}", angle))),
        PatternGene::Spotted { density } => ("spotted".to_string(), Some(format!("{{\"density\":{}}}", density))),
        PatternGene::Gradient { direction } => ("gradient".to_string(), Some(format!("{{\"direction\":{}}}", direction))),
        PatternGene::Bicolor { split } => ("bicolor".to_string(), Some(format!("{{\"split\":{}}}", split))),
    }
}

fn deserialize_pattern(type_str: &str, data: Option<&str>) -> PatternGene {
    match type_str {
        "striped" => {
            let angle = extract_f32(data, "angle").unwrap_or(0.0);
            PatternGene::Striped { angle }
        }
        "spotted" => {
            let density = extract_f32(data, "density").unwrap_or(0.5);
            PatternGene::Spotted { density }
        }
        "gradient" => {
            let direction = extract_f32(data, "direction").unwrap_or(0.0);
            PatternGene::Gradient { direction }
        }
        "bicolor" => {
            let split = extract_f32(data, "split").unwrap_or(0.5);
            PatternGene::Bicolor { split }
        }
        _ => PatternGene::Solid,
    }
}

fn extract_f32(json: Option<&str>, key: &str) -> Option<f32> {
    let s = json?;
    let v: serde_json::Value = serde_json::from_str(s).ok()?;
    v.get(key)?.as_f64().map(|f| f as f32)
}
