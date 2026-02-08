use crate::simulation::config::SimulationConfig;
use crate::simulation::fish::{BehaviorState, Fish};
use crate::simulation::genome::{genome_distance, FishGenome, Sex};
use rand::prelude::*;
use serde::{Deserialize, Serialize};

pub const BASE_LIFESPAN: u32 = 20_000;

// ─── Food ───

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FoodType {
    Flake,
    Pellet,
    LiveFood,
}

impl FoodType {
    pub fn nutrition(&self) -> f32 {
        match self {
            FoodType::Flake => 0.2,
            FoodType::Pellet => 0.3,
            FoodType::LiveFood => 0.5,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            FoodType::Flake => "flake",
            FoodType::Pellet => "pellet",
            FoodType::LiveFood => "live",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "flake" => FoodType::Flake,
            "live" => FoodType::LiveFood,
            _ => FoodType::Pellet,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FoodParticle {
    pub x: f32,
    pub y: f32,
    pub age: u32,
    pub on_floor: bool,
    pub food_type: FoodType,
}

impl FoodParticle {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y, age: 0, on_floor: false, food_type: FoodType::Pellet }
    }

    pub fn new_typed(x: f32, y: f32, food_type: FoodType) -> Self {
        Self { x, y, age: 0, on_floor: false, food_type }
    }

    pub fn update(&mut self, config: &SimulationConfig, tick: u64) {
        self.age += 1;
        match self.food_type {
            FoodType::Flake => {
                if !self.on_floor {
                    self.y += 0.1; // slow sink
                    self.x += (tick as f32 * 0.05 + self.x * 0.1).sin() * 0.8; // horizontal drift
                    if self.y >= config.tank_height - 30.0 {
                        self.on_floor = true;
                        self.y = config.tank_height - 30.0;
                    }
                }
            }
            FoodType::Pellet => {
                if !self.on_floor {
                    self.y += 0.5;
                    self.x += (tick as f32 * 0.05 + self.x * 0.1).sin() * 0.3;
                    if self.y >= config.tank_height - 30.0 {
                        self.on_floor = true;
                        self.y = config.tank_height - 30.0;
                    }
                }
            }
            FoodType::LiveFood => {
                // Never settles; wanders via sine movement
                self.x += (tick as f32 * 0.02 + self.y * 0.05).sin() * 0.6;
                self.y += (tick as f32 * 0.015 + self.x * 0.03).cos() * 0.4;
                self.x = self.x.clamp(10.0, config.tank_width - 10.0);
                self.y = self.y.clamp(10.0, config.tank_height - 40.0);
            }
        }
    }

    pub fn is_expired(&self, config: &SimulationConfig) -> bool {
        self.age >= config.food_decay_ticks
    }
}

// ─── Eggs ───

static NEXT_EGG_ID: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);

pub fn next_egg_id() -> u32 {
    NEXT_EGG_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

pub fn set_egg_id_counter(val: u32) {
    NEXT_EGG_ID.store(val, std::sync::atomic::Ordering::Relaxed);
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Egg {
    pub id: u32,
    pub genome_id: u32,
    pub x: f32,
    pub y: f32,
    pub age: u32,
    pub parent_a_genome: u32,
    pub parent_b_genome: u32,
}

// ─── Simulation Events ───

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SimEvent {
    Birth { fish_id: u32, genome_id: u32, parent_a: u32, parent_b: u32 },
    Death { fish_id: u32, genome_id: u32, cause: DeathCause, custom_name: Option<String>, is_favorite: bool },
    FeedingDrop { x: f32, y: f32 },
    Predation { predator_id: u32, prey_id: u32 },
    NewSpecies { species_id: u32 },
    Extinction { species_id: u32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeathCause {
    OldAge,
    Starvation,
    PoorWater,
    Predation,
}

// ─── Species ───

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Species {
    pub id: u32,
    pub name: Option<String>,
    pub description: Option<String>,
    pub discovered_at_tick: u64,
    pub extinct_at_tick: Option<u64>,
    pub centroid_hue: f32,
    pub centroid_speed: f32,
    pub centroid_size: f32,
    pub centroid_pattern: String,
    pub member_count: u32,
    pub member_genome_ids: Vec<u32>,
}

// ─── Decorations ───

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DecorationType {
    Rock,
    TallPlant,
    ShortPlant,
    Coral,
}

impl DecorationType {
    pub fn is_plant(&self) -> bool {
        matches!(self, DecorationType::TallPlant | DecorationType::ShortPlant)
    }

    pub fn obstacle_radius(&self) -> f32 {
        match self {
            DecorationType::Rock => 25.0,
            DecorationType::TallPlant => 12.0,
            DecorationType::ShortPlant => 8.0,
            DecorationType::Coral => 18.0,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            DecorationType::Rock => "rock",
            DecorationType::TallPlant => "tall_plant",
            DecorationType::ShortPlant => "short_plant",
            DecorationType::Coral => "coral",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "tall_plant" => DecorationType::TallPlant,
            "short_plant" => DecorationType::ShortPlant,
            "coral" => DecorationType::Coral,
            _ => DecorationType::Rock,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decoration {
    pub id: u32,
    pub decoration_type: DecorationType,
    pub x: f32,
    pub y: f32,
    pub scale: f32,
    pub flip_x: bool,
}

// ─── Bubble ───

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bubble {
    pub x: f32,
    pub y: f32,
    pub radius: f32,
    pub age: u32,
}

impl Bubble {
    pub fn new(x: f32, y: f32, rng: &mut impl Rng) -> Self {
        Self {
            x,
            y,
            radius: rng.gen_range(1.5..4.0),
            age: 0,
        }
    }

    pub fn update(&mut self, tick: u64) {
        self.age += 1;
        self.y -= 1.0 + self.radius * 0.2;
        self.x += ((tick as f32 + self.y) * 0.03).sin() * 0.4;
    }

    pub fn is_popped(&self) -> bool {
        self.y < 10.0
    }
}

// ─── Ecosystem Manager ───

pub struct EcosystemManager {
    pub food: Vec<FoodParticle>,
    pub bubbles: Vec<Bubble>,
    pub eggs: Vec<Egg>,
    pub water_quality: f32,
    pub species: Vec<Species>,
    pub events: Vec<SimEvent>,
    pub plant_count: u32,
    pub decorations: Vec<Decoration>,
    next_species_id: u32,
    next_decoration_id: u32,
    last_speciation_tick: u64,
    auto_feed_timer: u32,
}

impl EcosystemManager {
    pub fn new() -> Self {
        Self {
            food: Vec::new(),
            bubbles: Vec::new(),
            eggs: Vec::new(),
            water_quality: 1.0,
            species: Vec::new(),
            events: Vec::new(),
            plant_count: 0,
            decorations: Vec::new(),
            next_species_id: 1,
            next_decoration_id: 1,
            last_speciation_tick: 0,
            auto_feed_timer: 0,
        }
    }

    pub fn recompute_plant_count(&mut self) {
        self.plant_count = self.decorations.iter()
            .filter(|d| d.decoration_type.is_plant())
            .count() as u32;
    }

    pub fn add_decoration(&mut self, dtype: DecorationType, x: f32, y: f32, scale: f32, flip_x: bool) -> Decoration {
        let d = Decoration {
            id: self.next_decoration_id,
            decoration_type: dtype,
            x,
            y,
            scale,
            flip_x,
        };
        self.next_decoration_id += 1;
        self.decorations.push(d.clone());
        self.recompute_plant_count();
        d
    }

    pub fn remove_decoration(&mut self, id: u32) -> bool {
        let len = self.decorations.len();
        self.decorations.retain(|d| d.id != id);
        let removed = self.decorations.len() < len;
        if removed { self.recompute_plant_count(); }
        removed
    }

    pub fn obstacle_positions(&self) -> Vec<(f32, f32, f32)> {
        self.decorations.iter()
            .map(|d| (d.x, d.y, d.decoration_type.obstacle_radius() * d.scale))
            .collect()
    }

    pub fn restore_species_counter(&mut self, val: u32) {
        self.next_species_id = val;
    }

    pub fn restore_decoration_counter(&mut self, val: u32) {
        self.next_decoration_id = val;
    }

    pub fn restore_speciation_tick(&mut self, tick: u64) {
        self.last_speciation_tick = tick;
    }

    pub fn drop_food(&mut self, x: f32, y: f32) {
        self.food.push(FoodParticle::new(x, y.max(5.0).min(50.0)));
        self.events.push(SimEvent::FeedingDrop { x, y });
    }

    pub fn drop_food_typed(&mut self, x: f32, y: f32, food_type: FoodType) {
        self.food.push(FoodParticle::new_typed(x, y.max(5.0).min(50.0), food_type));
        self.events.push(SimEvent::FeedingDrop { x, y });
    }

    pub fn apply_glass_tap(
        fish: &mut [Fish],
        boldness_map: &std::collections::HashMap<u32, f32>,
        tap_x: f32,
        tap_y: f32,
    ) {
        for f in fish.iter_mut() {
            if !f.is_alive || f.behavior == BehaviorState::Dying {
                continue;
            }
            let dx = f.x - tap_x;
            let dy = f.y - tap_y;
            let dist = (dx * dx + dy * dy).sqrt();

            // Bold fish (>0.7) only flee within 80px, others within 150px
            let boldness = boldness_map.get(&f.genome_id).copied().unwrap_or(0.5);
            let flee_radius = if boldness > 0.7 { 80.0 } else { 150.0 };

            if dist < flee_radius {
                f.behavior = BehaviorState::Fleeing;
                f.tap_flee_timer = 60;
                f.stress = (f.stress + 0.05).min(1.0);

                // Push velocity away from tap point
                if dist > 0.1 {
                    let push = 3.0 * (1.0 - dist / flee_radius);
                    f.vx += (dx / dist) * push;
                    f.vy += (dy / dist) * push;
                }
            }
        }
    }

    pub fn update(
        &mut self,
        fish: &mut Vec<Fish>,
        genomes: &mut std::collections::HashMap<u32, FishGenome>,
        config: &SimulationConfig,
        tick: u64,
        rng: &mut impl Rng,
        time_of_day: f32,
        event_system: &crate::simulation::events::EventSystem,
    ) -> Vec<SimEvent> {
        // Drain inter-tick events (e.g. FeedingDrop from user clicks) instead of clearing
        let mut carried_events: Vec<SimEvent> = self.events.drain(..).collect();

        // Auto-feeder
        if config.auto_feed_enabled {
            self.auto_feed_timer += 1;
            if self.auto_feed_timer >= config.auto_feed_interval {
                self.auto_feed_timer = 0;
                for _ in 0..config.auto_feed_amount {
                    let x = rng.gen_range(50.0..config.tank_width - 50.0);
                    self.food.push(FoodParticle::new(x, 5.0));
                }
            }
        }

        // Update food
        for food in &mut self.food {
            food.update(config, tick);
        }

        // Update water quality (with environmental event extra degradation)
        self.update_water_quality(fish.len(), config);
        self.water_quality = (self.water_quality - event_system.extra_water_degradation()).clamp(0.0, 1.0);

        // Update bubbles
        self.spawn_bubbles(config, tick, rng);
        for b in &mut self.bubbles {
            b.update(tick);
        }
        self.bubbles.retain(|b| !b.is_popped());

        // Feeding - fish eat nearby food
        self.process_feeding(fish, config);

        // Predation
        self.process_predation(fish, genomes, config, tick, rng);

        // Behavior updates
        self.update_fish_behavior(fish, genomes, config, tick, time_of_day);

        // Reproduction (creates eggs, not fish directly)
        self.process_reproduction(fish, genomes, config, tick, rng);

        // Hatch eggs → juvenile fish
        self.process_eggs(fish, genomes, config, rng);

        // Egg predation — aggressive large fish eat nearby eggs
        self.process_egg_predation(fish, genomes);

        // Territory claiming & defense
        if config.territory_enabled {
            Self::process_territories(fish, genomes, config);
        }

        // Disease processing
        if config.disease_enabled {
            self.process_disease(fish, genomes, config, rng);
        }

        // Remove expired food
        let wq = &mut self.water_quality;
        self.food.retain(|f| {
            if f.is_expired(config) {
                // Decayed food degrades water
                *wq = (*wq - 0.001).max(0.0);
                false
            } else {
                true
            }
        });

        // Remove dead fish
        let events = &mut self.events;
        fish.retain(|f| {
            if !f.is_alive {
                events.push(SimEvent::Death {
                    fish_id: f.id,
                    genome_id: f.genome_id,
                    cause: if f.killed_by_predator {
                        DeathCause::Predation
                    } else if f.starvation_ticks >= 200 {
                        DeathCause::Starvation
                    } else if f.health <= 0.0 {
                        DeathCause::PoorWater
                    } else {
                        DeathCause::OldAge
                    },
                    custom_name: f.custom_name.clone(),
                    is_favorite: f.is_favorite,
                });
                false
            } else {
                true
            }
        });

        // Speciation detection (every 300 ticks)
        if tick - self.last_speciation_tick >= 300 && fish.len() >= 3 {
            self.detect_species(fish, genomes, config, tick);
            self.last_speciation_tick = tick;
        }

        carried_events.extend(self.events.drain(..));
        carried_events
    }

    fn update_water_quality(&mut self, fish_count: usize, config: &SimulationConfig) {
        // Degradation from fish
        let fish_degradation = fish_count as f32 * config.water_degradation_per_fish;
        // Degradation from uneaten food
        let food_degradation = self.food.len() as f32 * 0.0001;
        // Recovery
        let recovery = config.water_recovery_rate + self.plant_count as f32 * config.plant_recovery_bonus;

        self.water_quality = (self.water_quality - fish_degradation - food_degradation + recovery)
            .clamp(0.0, 1.0);
    }

    fn process_feeding(&mut self, fish: &mut [Fish], _config: &SimulationConfig) {
        let eating_radius_sq = 8.0 * 8.0;

        let mut eaten_food = std::collections::HashSet::new();
        let mut nutrition_map: Vec<(usize, f32)> = Vec::new(); // fish_idx -> nutrition
        for (fi, f) in fish.iter_mut().enumerate() {
            if !f.is_alive || (f.behavior != BehaviorState::Foraging && f.behavior != BehaviorState::Swimming) {
                continue;
            }
            if f.hunger < 0.2 {
                continue;
            }
            for (food_idx, food) in self.food.iter().enumerate() {
                let dx = f.x - food.x;
                let dy = f.y - food.y;
                if dx * dx + dy * dy < eating_radius_sq && !eaten_food.contains(&food_idx) {
                    eaten_food.insert(food_idx);
                    nutrition_map.push((fi, food.food_type.nutrition()));
                    f.eat();
                    break;
                }
            }
        }

        // Apply nutrition bonuses for non-pellet food
        for (fi, nutrition) in &nutrition_map {
            let f = &mut fish[*fi];
            // eat() reduces hunger by 0.3; adjust based on nutrition
            // pellet=0.3 is baseline; flake=0.2 less satisfying, live=0.5 more
            let adjustment = nutrition - 0.3;
            f.hunger = (f.hunger - adjustment).clamp(0.0, 1.0);
        }

        // Remove eaten food (reverse order to preserve indices via swap_remove on sorted-reverse)
        let mut eaten_sorted: Vec<_> = eaten_food.into_iter().collect();
        eaten_sorted.sort_unstable();
        for idx in eaten_sorted.into_iter().rev() {
            self.food.swap_remove(idx);
        }
    }

    fn process_predation(
        &mut self,
        fish: &mut [Fish],
        genomes: &std::collections::HashMap<u32, FishGenome>,
        config: &SimulationConfig,
        _tick: u64,
        rng: &mut impl Rng,
    ) {
        let scan_radius = 80.0;
        let scan_radius_sq = scan_radius * scan_radius;
        let strike_radius = 12.0;
        let strike_radius_sq = strike_radius * strike_radius;
        let max_chase_ticks: u32 = 150;

        // Snapshot for read-only queries while mutating
        let snap: Vec<(u32, f32, f32, u32, bool, BehaviorState, Option<u32>)> = fish
            .iter()
            .map(|f| (f.id, f.x, f.y, f.genome_id, f.is_alive, f.behavior, f.hunting_target))
            .collect();

        let mut kills: std::collections::HashSet<usize> = std::collections::HashSet::new();
        let mut fed_predators: Vec<(usize, f32)> = Vec::new(); // (idx, hunger_reduction)

        for i in 0..fish.len() {
            let (fid, fx, fy, gid, alive, beh, _htarget) = snap[i];
            if !alive || kills.contains(&i) { continue; }

            let genome = match genomes.get(&gid) {
                Some(g) => g,
                None => continue,
            };

            // === Phase 1: Target acquisition ===
            // Predators (aggression > 0.6) that are Swimming/Foraging/Satiated can start hunting
            if genome.aggression > 0.6
                && beh != BehaviorState::Hunting
                && beh != BehaviorState::Fleeing
                && beh != BehaviorState::Resting
                && beh != BehaviorState::Dying
                && beh != BehaviorState::Courting
            {
                // Scan for prey
                let mut best_prey: Option<(usize, f32)> = None;
                for j in 0..fish.len() {
                    if j == i || kills.contains(&j) { continue; }
                    let (_, px, py, pgid, palive, pbeh, _) = snap[j];
                    if !palive || pbeh == BehaviorState::Dying { continue; }
                    let prey_genome = match genomes.get(&pgid) {
                        Some(g) => g,
                        None => continue,
                    };
                    // Must be smaller
                    if prey_genome.body_length >= genome.body_length * config.predation_size_ratio {
                        continue;
                    }
                    let dx = fx - px;
                    let dy = fy - py;
                    let dist_sq = dx * dx + dy * dy;
                    if dist_sq < scan_radius_sq {
                        if best_prey.is_none() || dist_sq < best_prey.unwrap().1 {
                            best_prey = Some((j, dist_sq));
                        }
                    }
                }
                // Start hunting if prey found and hunger is relevant or aggression is high
                if let Some((prey_idx, _)) = best_prey {
                    if genome.aggression > 0.8 || fish[i].hunger > 0.3 {
                        fish[i].behavior = BehaviorState::Hunting;
                        fish[i].hunting_target = Some(snap[prey_idx].0);
                        fish[i].hunting_timer = 0;

                        // Make prey flee
                        fish[prey_idx].behavior = BehaviorState::Fleeing;
                        fish[prey_idx].fleeing_from = Some(fid);
                    }
                }
            }

            // === Phase 2: Chase + strike ===
            if fish[i].behavior == BehaviorState::Hunting {
                fish[i].hunting_timer += 1;
                let target_id = match fish[i].hunting_target {
                    Some(id) => id,
                    None => {
                        fish[i].behavior = BehaviorState::Swimming;
                        continue;
                    }
                };

                // Find target
                let target_idx = snap.iter().position(|s| s.0 == target_id);
                let target_alive = target_idx.map(|ti| fish[ti].is_alive && fish[ti].health > 0.0 && !kills.contains(&ti)).unwrap_or(false);

                if !target_alive || fish[i].hunting_timer >= max_chase_ticks {
                    // Give up
                    fish[i].behavior = BehaviorState::Swimming;
                    fish[i].hunting_target = None;
                    fish[i].hunting_timer = 0;
                    continue;
                }

                let ti = target_idx.unwrap();
                let (_, tx, ty, _, _, _, _) = snap[ti];
                let dx = fx - tx;
                let dy = fy - ty;
                let dist_sq = dx * dx + dy * dy;

                if dist_sq < strike_radius_sq {
                    // === Strike roll ===
                    // Pack hunting: count same-species hunters targeting the same prey within 50px
                    let mut pack_count = 0_u32;
                    for k in 0..fish.len() {
                        if k == i { continue; }
                        let (_, kx, ky, kgid, kalive, kbeh, ktarget) = snap[k];
                        if !kalive || kbeh != BehaviorState::Hunting { continue; }
                        if ktarget != Some(target_id) { continue; }
                        let dkx = fx - kx;
                        let dky = fy - ky;
                        if dkx * dkx + dky * dky < 50.0 * 50.0 {
                            if let Some(kg) = genomes.get(&kgid) {
                                if genome_distance(genome, kg) < config.species_threshold {
                                    pack_count += 1;
                                }
                            }
                        }
                    }

                    // Safety in numbers for prey
                    let mut prey_allies = 0_u32;
                    for k in 0..fish.len() {
                        if k == ti || !snap[k].4 { continue; }
                        let dkx = tx - snap[k].1;
                        let dky = ty - snap[k].2;
                        if dkx * dkx + dky * dky < config.separation_radius * config.separation_radius {
                            prey_allies += 1;
                        }
                    }
                    // Allies reduce attack chance but pack hunting can overcome
                    let ally_penalty = if prey_allies >= 3 { 0.3 } else { 1.0 };

                    // Pack bonus: 1.5x per extra hunter
                    let pack_bonus = 1.0 + pack_count as f32 * 0.5;
                    let attack_chance = genome.aggression * 0.15 * pack_bonus * ally_penalty;

                    if rng.gen::<f32>() < attack_chance {
                        kills.insert(ti);
                        self.events.push(SimEvent::Predation {
                            predator_id: fid,
                            prey_id: target_id,
                        });

                        // Share meal among pack (partial hunger reduction per member)
                        let share = 0.5 / (1.0 + pack_count as f32);
                        fed_predators.push((i, share));
                        // Feed pack members too
                        for k in 0..fish.len() {
                            if k == i { continue; }
                            let (_, kx, ky, kgid, kalive, kbeh, ktarget) = snap[k];
                            if !kalive || kbeh != BehaviorState::Hunting { continue; }
                            if ktarget != Some(target_id) { continue; }
                            let dkx = fx - kx;
                            let dky = fy - ky;
                            if dkx * dkx + dky * dky < 50.0 * 50.0 {
                                if let Some(kg) = genomes.get(&kgid) {
                                    if genome_distance(genome, kg) < config.species_threshold {
                                        fed_predators.push((k, share));
                                    }
                                }
                            }
                        }

                        // Reset hunting state
                        fish[i].behavior = BehaviorState::Swimming;
                        fish[i].hunting_target = None;
                        fish[i].hunting_timer = 0;
                    }
                }
            }
        }

        // Apply kills
        for &idx in &kills {
            fish[idx].behavior = BehaviorState::Dying;
            fish[idx].dying_timer = 0;
            fish[idx].health = 0.0;
            fish[idx].killed_by_predator = true;
        }
        // Apply feeding to predators
        for &(idx, hunger_reduction) in &fed_predators {
            fish[idx].hunger = (fish[idx].hunger - hunger_reduction).max(0.0);
            fish[idx].energy = (fish[idx].energy + 0.15).min(1.0);
            fish[idx].behavior = BehaviorState::Swimming;
            fish[idx].hunting_target = None;
            fish[idx].hunting_timer = 0;
        }
    }

    fn update_fish_behavior(
        &mut self,
        fish: &mut [Fish],
        genomes: &std::collections::HashMap<u32, FishGenome>,
        config: &SimulationConfig,
        tick: u64,
        time_of_day: f32,
    ) {
        // Pre-compute predator positions (include hunting fish as threats)
        let predator_info: Vec<(f32, f32, f32, u32)> = fish
            .iter()
            .filter_map(|f| {
                let g = genomes.get(&f.genome_id)?;
                if (g.aggression > 0.6 || f.behavior == BehaviorState::Hunting) && f.is_alive {
                    Some((f.x, f.y, g.body_length, f.id))
                } else {
                    None
                }
            })
            .collect();

        // Pre-compute potential mates
        let mate_info: Vec<(usize, f32, f32, u32, u32, Sex, f32)> = fish
            .iter()
            .enumerate()
            .filter_map(|(i, f)| {
                let g = genomes.get(&f.genome_id)?;
                if f.is_alive && f.can_reproduce(g, tick, config, BASE_LIFESPAN, self.water_quality) {
                    Some((i, f.x, f.y, f.id, f.genome_id, g.sex, g.body_length))
                } else {
                    None
                }
            })
            .collect();

        let mating_radius_sq = 30.0 * 30.0;

        for i in 0..fish.len() {
            if !fish[i].is_alive {
                continue;
            }
            let genome = match genomes.get(&fish[i].genome_id) {
                Some(g) => g,
                None => continue,
            };

            // Check for nearby predators
            let danger_radius = 80.0 * (1.0 - genome.boldness * 0.5);
            let danger_radius_sq = danger_radius * danger_radius;
            let has_predator = predator_info.iter().any(|&(px, py, pred_size, pid)| {
                if pid == fish[i].id {
                    return false;
                }
                if genome.body_length >= pred_size * config.predation_size_ratio {
                    return false;
                }
                let dx = fish[i].x - px;
                let dy = fish[i].y - py;
                dx * dx + dy * dy < danger_radius_sq
            });

            // Check for nearby compatible mate
            let has_mate = if fish[i].behavior == BehaviorState::Satiated {
                mate_info.iter().find_map(|&(_, mx, my, mid, mgid, msex, _)| {
                    if mid == fish[i].id || msex == genome.sex {
                        return None;
                    }
                    let dx = fish[i].x - mx;
                    let dy = fish[i].y - my;
                    if dx * dx + dy * dy > mating_radius_sq {
                        return None;
                    }
                    if let Some(mg) = genomes.get(&mgid) {
                        if genome_distance(genome, mg) < config.species_threshold {
                            return Some(mid);
                        }
                    }
                    None
                })
            } else {
                None
            };

            fish[i].update_behavior(
                genome,
                config,
                tick,
                has_predator,
                has_mate,
                BASE_LIFESPAN,
                self.water_quality,
                time_of_day,
            );
        }
    }

    fn process_reproduction(
        &mut self,
        fish: &mut Vec<Fish>,
        genomes: &mut std::collections::HashMap<u32, FishGenome>,
        config: &SimulationConfig,
        tick: u64,
        rng: &mut impl Rng,
    ) {
        let effective_capacity = (config.base_carrying_capacity as f32 * self.water_quality) as usize;
        if fish.len() >= effective_capacity {
            return;
        }

        let mut new_eggs: Vec<(Egg, FishGenome)> = Vec::new();
        let mut reproduced: Vec<u32> = Vec::new();

        for i in 0..fish.len() {
            if fish[i].behavior != BehaviorState::Courting || fish[i].courting_timer < 90 {
                continue;
            }
            let partner_id = match fish[i].courting_partner {
                Some(id) => id,
                None => continue,
            };
            if reproduced.contains(&fish[i].id) || reproduced.contains(&partner_id) {
                continue;
            }

            let partner_idx = match fish.iter().position(|f| f.id == partner_id) {
                Some(idx) => idx,
                None => continue,
            };

            let genome_a = match genomes.get(&fish[i].genome_id) {
                Some(g) => g.clone(),
                None => continue,
            };
            let genome_b = match genomes.get(&fish[partner_idx].genome_id) {
                Some(g) => g.clone(),
                None => continue,
            };

            // Fertility roll
            let fertility_avg = (genome_a.fertility + genome_b.fertility) / 2.0;
            if rng.gen::<f32>() > fertility_avg * config.fertility_scale {
                continue;
            }

            // Inbreeding check (share a parent = inbred for simplicity)
            let inbred = genome_a.parent_a.is_some()
                && (genome_a.parent_a == genome_b.parent_a
                    || genome_a.parent_a == genome_b.parent_b
                    || genome_a.parent_b == genome_b.parent_a
                    || genome_a.parent_b == genome_b.parent_b);

            let child_genome = FishGenome::inherit(&genome_a, &genome_b, rng, inbred, config.mutation_rate_large, config.mutation_rate_small);

            // Spawn egg at parents' midpoint, snapped near sand floor or nearest decoration
            let mid_x = (fish[i].x + fish[partner_idx].x) / 2.0;
            let mut egg_y = config.tank_height - 40.0; // default: sand floor
            // Try to find nearest decoration for egg placement
            let mut best_dist = f32::MAX;
            for dec in &self.decorations {
                let dx = dec.x - mid_x;
                let dy = dec.y - fish[i].y;
                let d = (dx * dx + dy * dy).sqrt();
                if d < best_dist && d < 200.0 {
                    best_dist = d;
                    egg_y = dec.y;
                }
            }

            let egg = Egg {
                id: next_egg_id(),
                genome_id: child_genome.id,
                x: mid_x,
                y: egg_y,
                age: 0,
                parent_a_genome: genome_a.id,
                parent_b_genome: genome_b.id,
            };

            reproduced.push(fish[i].id);
            reproduced.push(partner_id);
            fish[i].last_reproduced_tick = Some(tick);
            fish[partner_idx].last_reproduced_tick = Some(tick);
            fish[i].behavior = BehaviorState::Swimming;
            fish[i].courting_partner = None;
            fish[partner_idx].behavior = BehaviorState::Swimming;
            fish[partner_idx].courting_partner = None;

            new_eggs.push((egg, child_genome));

            if fish.len() + self.eggs.len() + new_eggs.len() >= effective_capacity {
                break;
            }
        }

        for (egg, genome) in new_eggs {
            genomes.insert(genome.id, genome);
            self.eggs.push(egg);
        }
    }

    fn process_eggs(
        &mut self,
        fish: &mut Vec<Fish>,
        genomes: &mut std::collections::HashMap<u32, FishGenome>,
        config: &SimulationConfig,
        rng: &mut impl Rng,
    ) {
        // Age all eggs and hatch mature ones
        let mut hatched_indices = Vec::new();
        for (idx, egg) in self.eggs.iter_mut().enumerate() {
            egg.age += 1;
            if egg.age >= config.egg_hatch_time {
                hatched_indices.push(idx);
            }
        }

        // Hatch in reverse order for safe removal
        hatched_indices.sort_unstable();
        hatched_indices.reverse();
        for idx in hatched_indices {
            let egg = self.eggs.swap_remove(idx);
            if genomes.contains_key(&egg.genome_id) {
                let mut child = Fish::new(egg.genome_id, egg.x, egg.y, rng);
                child.is_juvenile = true;
                child.juvenile_timer = 0;
                self.events.push(SimEvent::Birth {
                    fish_id: child.id,
                    genome_id: egg.genome_id,
                    parent_a: egg.parent_a_genome,
                    parent_b: egg.parent_b_genome,
                });
                fish.push(child);
            }
        }
    }

    fn process_egg_predation(
        &mut self,
        fish: &[Fish],
        genomes: &std::collections::HashMap<u32, FishGenome>,
    ) {
        if self.eggs.is_empty() { return; }

        // Collect territory zones — eggs inside a territory are protected (predators avoid)
        let territories: Vec<(f32, f32, f32)> = fish.iter()
            .filter(|f| f.is_alive && f.territory_center.is_some())
            .filter_map(|f| {
                let (cx, cy) = f.territory_center?;
                Some((cx, cy, f.territory_radius))
            })
            .collect();

        // Aggressive large fish eat nearby eggs
        let predator_ids: Vec<(f32, f32)> = fish.iter()
            .filter(|f| f.is_alive && f.behavior != BehaviorState::Dying)
            .filter_map(|f| {
                let g = genomes.get(&f.genome_id)?;
                if g.aggression > 0.7 && g.body_length > 1.2 {
                    Some((f.x, f.y))
                } else {
                    None
                }
            })
            .collect();

        self.eggs.retain(|egg| {
            // Check if egg is inside a territory (protected)
            let in_territory = territories.iter().any(|&(cx, cy, r)| {
                let dx = egg.x - cx;
                let dy = egg.y - cy;
                dx * dx + dy * dy < r * r
            });

            for &(px, py) in &predator_ids {
                let dx = egg.x - px;
                let dy = egg.y - py;
                if dx * dx + dy * dy < 20.0 * 20.0 {
                    // Eggs in territory have 50% chance of surviving (territory defense)
                    if in_territory { return true; } // guarded — safe from this predator
                    return false; // eaten
                }
            }
            true
        });
    }

    fn process_territories(
        fish: &mut [Fish],
        genomes: &std::collections::HashMap<u32, FishGenome>,
        config: &SimulationConfig,
    ) {
        // Territorial fish: low school_affinity (<0.3) + moderate aggression (>0.4)
        // They claim a territory around a point and defend it.
        // Territory is claimed once and kept until death.

        // Collect existing territory centers for intruder checks
        let territory_snap: Vec<(u32, Option<(f32, f32)>, f32, u32)> = fish
            .iter()
            .map(|f| (f.id, f.territory_center, f.territory_radius, f.genome_id))
            .collect();

        for i in 0..fish.len() {
            if !fish[i].is_alive || fish[i].behavior == BehaviorState::Dying {
                continue;
            }
            let genome = match genomes.get(&fish[i].genome_id) {
                Some(g) => g,
                None => continue,
            };

            // Check if this fish is territorial
            if genome.school_affinity >= 0.3 || genome.aggression <= 0.4 {
                // Not territorial — clear any claimed territory
                fish[i].territory_center = None;
                fish[i].territory_radius = 0.0;
                continue;
            }

            // Claim territory if not yet claimed
            if fish[i].territory_center.is_none() && !fish[i].is_juvenile {
                // Claim at current position
                let radius = config.territory_claim_radius * genome.body_length;
                fish[i].territory_center = Some((fish[i].x, fish[i].y));
                fish[i].territory_radius = radius;
            }

            // Intruder detection: if fish has territory, check for intruders of different species
            if let Some((cx, cy)) = fish[i].territory_center {
                let radius_sq = fish[i].territory_radius * fish[i].territory_radius;
                for j in 0..territory_snap.len() {
                    if j == i { continue; }
                    let (other_id, _, _, other_gid) = territory_snap[j];
                    if other_id == fish[i].id { continue; }
                    if !fish[j].is_alive || fish[j].behavior == BehaviorState::Dying { continue; }

                    let other_genome = match genomes.get(&other_gid) {
                        Some(g) => g,
                        None => continue,
                    };

                    // Only react to different species
                    if genome_distance(genome, other_genome) < config.species_threshold {
                        continue;
                    }

                    let dx = fish[j].x - cx;
                    let dy = fish[j].y - cy;
                    if dx * dx + dy * dy < radius_sq {
                        // Intruder detected — chase them if aggressive enough, else posture
                        if genome.aggression > 0.7
                            && fish[i].behavior != BehaviorState::Hunting
                            && fish[i].behavior != BehaviorState::Fleeing
                        {
                            fish[i].behavior = BehaviorState::Hunting;
                            fish[i].hunting_target = Some(other_id);
                            fish[i].hunting_timer = 0;
                        }
                        break; // only chase one intruder at a time
                    }
                }
            }
        }
    }

    fn detect_species(
        &mut self,
        fish: &[Fish],
        genomes: &std::collections::HashMap<u32, FishGenome>,
        config: &SimulationConfig,
        tick: u64,
    ) {
        if fish.len() < 3 {
            return;
        }

        // Collect living genomes
        let living: Vec<&FishGenome> = fish
            .iter()
            .filter_map(|f| genomes.get(&f.genome_id))
            .collect();

        let n = living.len();
        if n < 3 {
            return;
        }

        // Single-linkage agglomerative clustering
        let mut cluster: Vec<usize> = (0..n).collect();

        for i in 0..n {
            for j in (i + 1)..n {
                let d = genome_distance(living[i], living[j]);
                if d < config.species_threshold {
                    // Union
                    let ci = find_root(&cluster, i);
                    let cj = find_root(&cluster, j);
                    if ci != cj {
                        cluster[ci] = cj;
                    }
                }
            }
        }

        // Collect clusters
        let mut clusters: std::collections::HashMap<usize, Vec<usize>> = std::collections::HashMap::new();
        for i in 0..n {
            let root = find_root(&cluster, i);
            clusters.entry(root).or_default().push(i);
        }

        // Filter to clusters with min members
        let valid_clusters: Vec<Vec<usize>> = clusters
            .into_values()
            .filter(|c| c.len() >= config.species_min_members as usize)
            .collect();

        // Compare to existing species
        let mut matched_species: Vec<u32> = Vec::new();

        for members in &valid_clusters {
            // Circular mean for hue (0-360)
            let (sin_sum, cos_sum) = members.iter().fold((0.0_f32, 0.0_f32), |(s, c), &i| {
                let rad = living[i].base_hue.to_radians();
                (s + rad.sin(), c + rad.cos())
            });
            let avg_hue = sin_sum.atan2(cos_sum).to_degrees().rem_euclid(360.0);
            let avg_speed = members.iter().map(|&i| living[i].speed).sum::<f32>() / members.len() as f32;
            let avg_size = members.iter().map(|&i| living[i].body_length).sum::<f32>() / members.len() as f32;

            // Try to match existing species by centroid similarity
            let mut found = false;
            for sp in &mut self.species {
                if sp.extinct_at_tick.is_some() {
                    continue;
                }
                let hue_diff = (avg_hue - sp.centroid_hue).abs().min(360.0 - (avg_hue - sp.centroid_hue).abs());
                let speed_diff = (avg_speed - sp.centroid_speed).abs();
                let size_diff = (avg_size - sp.centroid_size).abs();
                if hue_diff < 30.0 && speed_diff < 0.5 && size_diff < 0.5 {
                    sp.member_count = members.len() as u32;
                    sp.member_genome_ids = members.iter().map(|&i| living[i].id).collect();
                    sp.centroid_hue = avg_hue;
                    sp.centroid_speed = avg_speed;
                    sp.centroid_size = avg_size;
                    matched_species.push(sp.id);
                    found = true;
                    break;
                }
            }

            if !found {
                let species_id = self.next_species_id;
                self.next_species_id += 1;
                let pattern_str = members
                    .first()
                    .map(|&i| format!("{:?}", living[i].pattern))
                    .unwrap_or_default();

                self.species.push(Species {
                    id: species_id,
                    name: None,
                    description: None,
                    discovered_at_tick: tick,
                    extinct_at_tick: None,
                    centroid_hue: avg_hue,
                    centroid_speed: avg_speed,
                    centroid_size: avg_size,
                    centroid_pattern: pattern_str,
                    member_count: members.len() as u32,
                    member_genome_ids: members.iter().map(|&i| living[i].id).collect(),
                });
                self.events.push(SimEvent::NewSpecies { species_id });
                matched_species.push(species_id);
            }
        }

        // Mark extinctions
        for sp in &mut self.species {
            if sp.extinct_at_tick.is_none() && !matched_species.contains(&sp.id) {
                sp.extinct_at_tick = Some(tick);
                self.events.push(SimEvent::Extinction { species_id: sp.id });
            }
        }

        // Prune long-extinct species to prevent unbounded growth
        self.species.retain(|sp| {
            match sp.extinct_at_tick {
                Some(extinct_tick) => tick - extinct_tick < 10_000,
                None => true,
            }
        });
    }

    fn process_disease(
        &mut self,
        fish: &mut [Fish],
        genomes: &std::collections::HashMap<u32, FishGenome>,
        config: &SimulationConfig,
        rng: &mut impl Rng,
    ) {
        let spread_radius_sq = config.disease_spread_radius * config.disease_spread_radius;

        // Spontaneous outbreak: tiny per-tick chance per fish
        for f in fish.iter_mut() {
            if !f.is_alive || f.is_infected || f.recovery_timer > 0 {
                continue;
            }
            let resistance = genomes.get(&f.genome_id).map(|g| g.disease_resistance).unwrap_or(0.5);
            if rng.gen::<f32>() < config.disease_spontaneous_chance * (1.0 - resistance) {
                f.is_infected = true;
                f.infection_timer = 0;
            }
        }

        // Spreading: infected fish infect nearby fish
        let infected_positions: Vec<(f32, f32)> = fish.iter()
            .filter(|f| f.is_alive && f.is_infected)
            .map(|f| (f.x, f.y))
            .collect();

        for f in fish.iter_mut() {
            if !f.is_alive || f.is_infected || f.recovery_timer > 0 {
                continue;
            }
            let resistance = genomes.get(&f.genome_id).map(|g| g.disease_resistance).unwrap_or(0.5);
            for &(ix, iy) in &infected_positions {
                let dx = f.x - ix;
                let dy = f.y - iy;
                if dx * dx + dy * dy < spread_radius_sq {
                    if rng.gen::<f32>() < config.disease_infection_chance * (1.0 - resistance) * 0.01 {
                        f.is_infected = true;
                        f.infection_timer = 0;
                        break;
                    }
                }
            }
        }

        // Update infected fish: damage + recovery
        for f in fish.iter_mut() {
            if !f.is_alive {
                continue;
            }
            if f.is_infected {
                f.infection_timer += 1;
                f.health -= config.disease_damage;
                f.energy = (f.energy - 0.0003).max(0.0);

                if f.infection_timer >= config.disease_duration {
                    f.is_infected = false;
                    f.infection_timer = 0;
                    f.recovery_timer = config.disease_duration / 2; // temporary immunity
                }
            } else if f.recovery_timer > 0 {
                f.recovery_timer -= 1;
            }
        }
    }

    fn spawn_bubbles(&mut self, config: &SimulationConfig, tick: u64, rng: &mut impl Rng) {
        // Spawn from a few fixed points (filter, plants)
        if tick % 15 == 0 {
            let spawn_x = [config.tank_width * 0.2, config.tank_width * 0.7, config.tank_width * 0.5];
            for &x in &spawn_x {
                if rng.gen::<f32>() < config.bubble_rate * 0.3 {
                    self.bubbles.push(Bubble::new(
                        x + rng.gen_range(-10.0..10.0),
                        config.tank_height - 40.0,
                        rng,
                    ));
                }
            }
        }
    }

    /// Force-breed two fish, bypassing courting. Produces an egg immediately.
    pub fn force_breed(
        &mut self,
        fish: &mut [Fish],
        genomes: &mut std::collections::HashMap<u32, FishGenome>,
        config: &SimulationConfig,
        tick: u64,
        rng: &mut impl Rng,
        fish_a_id: u32,
        fish_b_id: u32,
    ) -> Result<u32, String> {
        let a_idx = fish.iter().position(|f| f.id == fish_a_id && f.is_alive)
            .ok_or_else(|| "Fish A not found or dead".to_string())?;
        let b_idx = fish.iter().position(|f| f.id == fish_b_id && f.is_alive)
            .ok_or_else(|| "Fish B not found or dead".to_string())?;
        if a_idx == b_idx {
            return Err("Cannot breed a fish with itself".to_string());
        }

        let genome_a = genomes.get(&fish[a_idx].genome_id).cloned()
            .ok_or_else(|| "Genome A not found".to_string())?;
        let genome_b = genomes.get(&fish[b_idx].genome_id).cloned()
            .ok_or_else(|| "Genome B not found".to_string())?;

        if genome_a.sex == genome_b.sex {
            return Err("Must be opposite sex".to_string());
        }

        let age_frac_a = fish[a_idx].age_fraction(&genome_a, BASE_LIFESPAN);
        let age_frac_b = fish[b_idx].age_fraction(&genome_b, BASE_LIFESPAN);
        if age_frac_a < genome_a.maturity_age || age_frac_b < genome_b.maturity_age {
            return Err("Both fish must be mature".to_string());
        }
        if fish[a_idx].is_juvenile || fish[b_idx].is_juvenile {
            return Err("Juvenile fish cannot breed".to_string());
        }

        // Cross-species: higher mutation rate
        let cross_species = genome_distance(&genome_a, &genome_b) >= config.species_threshold;
        let large_rate = if cross_species { config.mutation_rate_large * 2.0 } else { config.mutation_rate_large };
        let small_rate = if cross_species { config.mutation_rate_small * 1.5 } else { config.mutation_rate_small };

        let inbred = genome_a.parent_a.is_some()
            && (genome_a.parent_a == genome_b.parent_a
                || genome_a.parent_a == genome_b.parent_b
                || genome_a.parent_b == genome_b.parent_a
                || genome_a.parent_b == genome_b.parent_b);

        let child_genome = FishGenome::inherit(&genome_a, &genome_b, rng, inbred, large_rate, small_rate);

        let mid_x = (fish[a_idx].x + fish[b_idx].x) / 2.0;
        let mut egg_y = config.tank_height - 40.0;
        let mut best_dist = f32::MAX;
        for dec in &self.decorations {
            let dx = dec.x - mid_x;
            let dy = dec.y - fish[a_idx].y;
            let d = (dx * dx + dy * dy).sqrt();
            if d < best_dist && d < 200.0 {
                best_dist = d;
                egg_y = dec.y;
            }
        }

        let egg_id = next_egg_id();
        let egg = Egg {
            id: egg_id,
            genome_id: child_genome.id,
            x: mid_x,
            y: egg_y,
            age: 0,
            parent_a_genome: genome_a.id,
            parent_b_genome: genome_b.id,
        };

        fish[a_idx].last_reproduced_tick = Some(tick);
        fish[b_idx].last_reproduced_tick = Some(tick);

        genomes.insert(child_genome.id, child_genome);
        self.eggs.push(egg);

        Ok(egg_id)
    }

    pub fn food_positions(&self) -> Vec<(f32, f32)> {
        self.food.iter().map(|f| (f.x, f.y)).collect()
    }
}

fn find_root(cluster: &[usize], mut i: usize) -> usize {
    while cluster[i] != i {
        i = cluster[i];
    }
    i
}
