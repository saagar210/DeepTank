pub mod achievements;
pub mod boids;
pub mod config;
pub mod ecosystem;
pub mod events;
pub mod fish;
pub mod genome;
pub mod ollama;
pub mod persistence;
pub mod scenarios;

use boids::BoidsEngine;
use config::SimulationConfig;
use ecosystem::{EcosystemManager, SimEvent};
use events::EventSystem;
use fish::Fish;
use genome::FishGenome;
use chrono::Timelike;
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Frame payload sent to React each tick
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameUpdate {
    pub tick: u64,
    pub fish: Vec<FishState>,
    pub food: Vec<FoodState>,
    pub bubbles: Vec<BubbleState>,
    pub eggs: Vec<EggState>,
    pub decorations: Vec<DecorationState>,
    pub events: Vec<SimEvent>,
    pub water_quality: f32,
    pub population: u32,
    pub max_generation: u32,
    pub species_count: u32,
    pub time_of_day: f32,
    pub active_event: Option<String>,
    pub genetic_diversity: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FishState {
    pub id: u32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub vx: f32,
    pub vy: f32,
    pub heading: f32,
    pub behavior: String,
    pub hunger: f32,
    pub health: f32,
    pub age_fraction: f32,
    pub genome_id: u32,
    pub energy: f32,
    pub is_infected: bool,
    pub is_juvenile: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub territory_cx: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub territory_cy: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub territory_r: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_name: Option<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub is_favorite: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EggState {
    pub id: u32,
    pub x: f32,
    pub y: f32,
    pub genome_id: u32,
    pub progress: f32, // 0.0 to 1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FoodState {
    pub x: f32,
    pub y: f32,
    pub food_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BubbleState {
    pub x: f32,
    pub y: f32,
    pub radius: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecorationState {
    pub id: u32,
    pub decoration_type: String,
    pub x: f32,
    pub y: f32,
    pub scale: f32,
    pub flip_x: bool,
}

/// Top-level simulation state managed by Tauri
pub struct SimulationState {
    pub tick: u64,
    pub paused: bool,
    pub speed_multiplier: f32,
    pub config: SimulationConfig,
    pub fish: Vec<Fish>,
    pub genomes: HashMap<u32, FishGenome>,
    pub boids: BoidsEngine,
    pub ecosystem: EcosystemManager,
    pub rng: StdRng,
    pub selected_fish_id: Option<u32>,
    pub time_of_day: f32, // 0.0-24.0
    pub event_system: EventSystem,
    pub genetic_diversity: f32,
    pub active_scenario_id: Option<String>,
}

impl SimulationState {
    pub fn new() -> Self {
        let config = SimulationConfig::default();
        let boids = BoidsEngine::new(&config);
        let mut rng = StdRng::from_entropy();

        let mut genomes = HashMap::new();
        let mut fish_list = Vec::new();

        // Seed initial population
        let initial_count = 18;
        for i in 0..initial_count {
            let genome = FishGenome::random_diverse(&mut rng, i, initial_count);
            let x = rng.gen_range(100.0..config.tank_width - 100.0);
            let y = rng.gen_range(100.0..config.tank_height - 100.0);
            let f = Fish::new(genome.id, x, y, &mut rng);
            genomes.insert(genome.id, genome);
            fish_list.push(f);
        }

        Self {
            tick: 0,
            paused: false,
            speed_multiplier: 1.0,
            config,
            fish: fish_list,
            genomes,
            boids,
            ecosystem: EcosystemManager::new(),
            rng,
            selected_fish_id: None,
            time_of_day: 12.0,
            event_system: EventSystem::new(),
            genetic_diversity: 1.0,
            active_scenario_id: None,
        }
    }

    pub fn step(&mut self) -> FrameUpdate {
        if self.paused {
            return self.build_frame(Vec::new());
        }

        self.tick += 1;

        // Advance day/night cycle
        if self.config.day_night_speed > 0.0 {
            // At speed=1: 1 sim-minute per real-second at 30Hz → 24h in 24 real-minutes
            self.time_of_day += (1.0 / 30.0 / 60.0) * self.config.day_night_speed;
            self.time_of_day = self.time_of_day.rem_euclid(24.0);
        } else {
            // Real-time clock mode
            let now = chrono::Local::now();
            self.time_of_day = now.hour() as f32 + now.minute() as f32 / 60.0;
        }

        // Environmental events
        if self.config.environmental_events_enabled {
            self.event_system.update(self.config.event_frequency, &mut self.rng);
        }

        // Spawn free food during plankton bloom
        if self.event_system.should_spawn_free_food(self.tick) {
            let x = self.rng.gen_range(50.0..self.config.tank_width - 50.0);
            self.ecosystem.food.push(ecosystem::FoodParticle::new(x, 5.0));
        }

        // Apply event modifiers to config temporarily
        let saved_current_strength = self.config.current_strength;
        let saved_hunger_rate = self.config.hunger_rate;
        if let Some(cs) = self.event_system.current_strength_override() {
            self.config.current_strength = cs;
        }
        self.config.hunger_rate *= self.event_system.metabolism_multiplier();

        // Boids physics (speed modifier applied per-fish through behavior_speed_multiplier)
        let food_positions = self.ecosystem.food_positions();
        let obstacles = self.ecosystem.obstacle_positions();
        self.boids.update(
            &mut self.fish,
            &self.genomes,
            &self.config,
            self.tick,
            &food_positions,
            &obstacles,
        );

        // Ecosystem (behavior, feeding, predation, reproduction, speciation)
        let events = self.ecosystem.update(
            &mut self.fish,
            &mut self.genomes,
            &self.config,
            self.tick,
            &mut self.rng,
            self.time_of_day,
            &self.event_system,
        );

        // Apply heatwave energy drain
        let energy_mult = self.event_system.energy_drain_multiplier();
        if energy_mult > 1.0 {
            for f in &mut self.fish {
                f.energy = (f.energy - 0.0002 * (energy_mult - 1.0)).max(0.0);
            }
        }

        // Restore config
        self.config.current_strength = saved_current_strength;
        self.config.hunger_rate = saved_hunger_rate;

        // Prune dead genomes every 500 ticks to prevent unbounded growth
        if self.tick % 500 == 0 {
            let living_genome_ids: std::collections::HashSet<u32> =
                self.fish.iter().map(|f| f.genome_id).collect();
            let species_genome_ids: std::collections::HashSet<u32> =
                self.ecosystem.species.iter()
                    .filter(|s| s.extinct_at_tick.is_none())
                    .flat_map(|s| s.member_genome_ids.iter().copied())
                    .collect();
            self.genomes.retain(|id, _| living_genome_ids.contains(id) || species_genome_ids.contains(id));
        }

        // Recompute genetic diversity periodically (every 60 ticks ≈ 2sec)
        if self.tick % 60 == 0 {
            self.genetic_diversity = Self::compute_diversity_index(&self.genomes, &self.fish);
        }

        self.build_frame(events)
    }

    fn compute_diversity_index(genomes: &HashMap<u32, FishGenome>, fish: &[Fish]) -> f32 {
        if fish.len() < 2 { return 0.0; }
        // Shannon-Wiener index on binned traits: hue(12 bins), speed(5), size(5), pattern(5)
        let mut bins: HashMap<(u8, u8, u8, u8), u32> = HashMap::new();
        for f in fish {
            if let Some(g) = genomes.get(&f.genome_id) {
                let hue_bin = (g.base_hue / 30.0).min(11.0) as u8;
                let speed_bin = ((g.speed - 0.5) / 0.3).clamp(0.0, 4.0) as u8;
                let size_bin = ((g.body_length - 0.6) / 0.28).clamp(0.0, 4.0) as u8;
                let pat_bin = match &g.pattern {
                    genome::PatternGene::Solid => 0u8,
                    genome::PatternGene::Striped { .. } => 1,
                    genome::PatternGene::Spotted { .. } => 2,
                    genome::PatternGene::Gradient { .. } => 3,
                    genome::PatternGene::Bicolor { .. } => 4,
                };
                *bins.entry((hue_bin, speed_bin, size_bin, pat_bin)).or_default() += 1;
            }
        }
        if bins.is_empty() { return 0.0; }
        let n = fish.len() as f32;
        let h: f32 = bins.values()
            .map(|&count| { let p = count as f32 / n; -p * p.ln() })
            .sum();
        let max_h = (bins.len() as f32).ln().max(0.001);
        (h / max_h).clamp(0.0, 1.0)
    }

    pub fn build_frame(&self, events: Vec<SimEvent>) -> FrameUpdate {
        let max_gen = self.genomes.values().map(|g| g.generation).max().unwrap_or(0);
        let species_count = self.ecosystem.species.iter().filter(|s| s.extinct_at_tick.is_none()).count() as u32;

        FrameUpdate {
            tick: self.tick,
            fish: self.fish.iter().map(|f| {
                let age_frac = self.genomes.get(&f.genome_id)
                    .map(|g| f.age_fraction(g, ecosystem::BASE_LIFESPAN))
                    .unwrap_or(0.0);
                FishState {
                    id: f.id,
                    x: f.x,
                    y: f.y,
                    z: f.z,
                    vx: f.vx,
                    vy: f.vy,
                    heading: f.heading,
                    behavior: f.behavior.as_str().to_string(),
                    hunger: f.hunger,
                    health: f.health,
                    age_fraction: age_frac,
                    genome_id: f.genome_id,
                    energy: f.energy,
                    is_infected: f.is_infected,
                    is_juvenile: f.is_juvenile,
                    territory_cx: f.territory_center.map(|(cx, _)| cx),
                    territory_cy: f.territory_center.map(|(_, cy)| cy),
                    territory_r: if f.territory_center.is_some() { Some(f.territory_radius) } else { None },
                    custom_name: f.custom_name.clone(),
                    is_favorite: f.is_favorite,
                }
            }).collect(),
            food: self.ecosystem.food.iter().map(|f| FoodState { x: f.x, y: f.y, food_type: f.food_type.as_str().to_string() }).collect(),
            bubbles: self.ecosystem.bubbles.iter().map(|b| BubbleState { x: b.x, y: b.y, radius: b.radius }).collect(),
            eggs: self.ecosystem.eggs.iter().map(|e| EggState {
                id: e.id,
                x: e.x,
                y: e.y,
                genome_id: e.genome_id,
                progress: if self.config.egg_hatch_time > 0 { e.age as f32 / self.config.egg_hatch_time as f32 } else { 1.0 },
            }).collect(),
            decorations: self.ecosystem.decorations.iter().map(|d| DecorationState {
                id: d.id,
                decoration_type: d.decoration_type.as_str().to_string(),
                x: d.x,
                y: d.y,
                scale: d.scale,
                flip_x: d.flip_x,
            }).collect(),
            events,
            water_quality: self.ecosystem.water_quality,
            population: self.fish.len() as u32,
            max_generation: max_gen,
            species_count,
            time_of_day: self.time_of_day,
            active_event: self.event_system.active_event_name().map(|s| s.to_string()),
            genetic_diversity: self.genetic_diversity,
        }
    }

    /// Get genome data for a specific fish (for frontend caching)
    pub fn get_genome(&self, genome_id: u32) -> Option<&FishGenome> {
        self.genomes.get(&genome_id)
    }

}
