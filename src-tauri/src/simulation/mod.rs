pub mod achievements;
pub mod boids;
pub mod config;
pub mod ecosystem;
pub mod fish;
pub mod genome;
pub mod ollama;
pub mod persistence;

use boids::BoidsEngine;
use config::SimulationConfig;
use ecosystem::{EcosystemManager, SimEvent};
use fish::Fish;
use genome::FishGenome;
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
    pub decorations: Vec<DecorationState>,
    pub events: Vec<SimEvent>,
    pub water_quality: f32,
    pub population: u32,
    pub max_generation: u32,
    pub species_count: u32,
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
        }
    }

    pub fn step(&mut self) -> FrameUpdate {
        if self.paused {
            return self.build_frame(Vec::new());
        }

        self.tick += 1;

        // Boids physics
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
        );

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

        self.build_frame(events)
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
                }
            }).collect(),
            food: self.ecosystem.food.iter().map(|f| FoodState { x: f.x, y: f.y, food_type: f.food_type.as_str().to_string() }).collect(),
            bubbles: self.ecosystem.bubbles.iter().map(|b| BubbleState { x: b.x, y: b.y, radius: b.radius }).collect(),
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
        }
    }

    /// Get genome data for a specific fish (for frontend caching)
    pub fn get_genome(&self, genome_id: u32) -> Option<&FishGenome> {
        self.genomes.get(&genome_id)
    }

}
