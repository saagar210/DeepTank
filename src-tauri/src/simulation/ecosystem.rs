use crate::simulation::config::SimulationConfig;
use crate::simulation::fish::{BehaviorState, Fish};
use crate::simulation::genome::{genome_distance, FishGenome, Sex};
use rand::prelude::*;
use serde::{Deserialize, Serialize};

pub const BASE_LIFESPAN: u32 = 20_000;

// ─── Food ───

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FoodParticle {
    pub x: f32,
    pub y: f32,
    pub age: u32,
    pub on_floor: bool,
}

impl FoodParticle {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y, age: 0, on_floor: false }
    }

    pub fn update(&mut self, config: &SimulationConfig, tick: u64) {
        self.age += 1;
        if !self.on_floor {
            // Drift downward with sine wobble
            self.y += 0.5;
            self.x += (tick as f32 * 0.05 + self.x * 0.1).sin() * 0.3;
            if self.y >= config.tank_height - 30.0 {
                self.on_floor = true;
                self.y = config.tank_height - 30.0;
            }
        }
    }

    pub fn is_expired(&self, config: &SimulationConfig) -> bool {
        self.age >= config.food_decay_ticks
    }
}

// ─── Simulation Events ───

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SimEvent {
    Birth { fish_id: u32, genome_id: u32, parent_a: u32, parent_b: u32 },
    Death { fish_id: u32, genome_id: u32, cause: DeathCause },
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
    pub water_quality: f32,
    pub species: Vec<Species>,
    pub events: Vec<SimEvent>,
    pub plant_count: u32,
    next_species_id: u32,
    last_speciation_tick: u64,
    auto_feed_timer: u32,
}

impl EcosystemManager {
    pub fn new() -> Self {
        Self {
            food: Vec::new(),
            bubbles: Vec::new(),
            water_quality: 1.0,
            species: Vec::new(),
            events: Vec::new(),
            plant_count: 2,
            next_species_id: 1,
            last_speciation_tick: 0,
            auto_feed_timer: 0,
        }
    }

    pub fn restore_species_counter(&mut self, val: u32) {
        self.next_species_id = val;
    }

    pub fn restore_speciation_tick(&mut self, tick: u64) {
        self.last_speciation_tick = tick;
    }

    pub fn drop_food(&mut self, x: f32, y: f32) {
        self.food.push(FoodParticle::new(x, y.max(5.0).min(50.0)));
        self.events.push(SimEvent::FeedingDrop { x, y });
    }

    pub fn update(
        &mut self,
        fish: &mut Vec<Fish>,
        genomes: &mut std::collections::HashMap<u32, FishGenome>,
        config: &SimulationConfig,
        tick: u64,
        rng: &mut impl Rng,
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

        // Update water quality
        self.update_water_quality(fish.len(), config);

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
        self.update_fish_behavior(fish, genomes, config, tick);

        // Reproduction
        self.process_reproduction(fish, genomes, config, tick, rng);

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
        for f in fish.iter_mut() {
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
                    f.eat();
                    break;
                }
            }
        }

        // Remove eaten food (reverse order to preserve indices)
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
        let attack_radius = 15.0;
        let attack_radius_sq = attack_radius * attack_radius;

        let mut kills: Vec<usize> = Vec::new();
        let mut predator_fed: Vec<usize> = Vec::new();

        let fish_snapshot: Vec<(f32, f32, u32, bool, BehaviorState)> = fish
            .iter()
            .map(|f| (f.x, f.y, f.genome_id, f.is_alive, f.behavior))
            .collect();

        for i in 0..fish.len() {
            let (ax, ay, a_gid, a_alive, a_beh) = fish_snapshot[i];
            if !a_alive || kills.contains(&i) || predator_fed.contains(&i) {
                continue;
            }
            let a_genome = match genomes.get(&a_gid) {
                Some(g) => g,
                None => continue,
            };
            if a_genome.aggression <= 0.7 {
                continue;
            }
            if a_beh == BehaviorState::Fleeing || a_beh == BehaviorState::Resting || a_beh == BehaviorState::Dying {
                continue;
            }

            for j in 0..fish.len() {
                if i == j || kills.contains(&j) {
                    continue;
                }
                let (bx, by, b_gid, b_alive, _) = fish_snapshot[j];
                if !b_alive {
                    continue;
                }
                let b_genome = match genomes.get(&b_gid) {
                    Some(g) => g,
                    None => continue,
                };

                if b_genome.body_length >= a_genome.body_length * config.predation_size_ratio {
                    continue;
                }

                let dx = ax - bx;
                let dy = ay - by;
                if dx * dx + dy * dy > attack_radius_sq {
                    continue;
                }

                // Safety in numbers check
                let mut nearby_allies = 0_u32;
                for k in 0..fish.len() {
                    if k == j || !fish_snapshot[k].3 {
                        continue;
                    }
                    let (kx, ky, _, _, _) = fish_snapshot[k];
                    let dkx = bx - kx;
                    let dky = by - ky;
                    if dkx * dkx + dky * dky < config.separation_radius * config.separation_radius {
                        nearby_allies += 1;
                    }
                }
                if nearby_allies >= 3 {
                    continue;
                }

                // Predation roll
                if rng.gen::<f32>() < a_genome.aggression * 0.1 {
                    kills.push(j);
                    predator_fed.push(i);
                    self.events.push(SimEvent::Predation {
                        predator_id: fish[i].id,
                        prey_id: fish[j].id,
                    });
                    break;
                }
            }
        }

        for &idx in &kills {
            fish[idx].behavior = BehaviorState::Dying;
            fish[idx].dying_timer = 0;
            fish[idx].health = 0.0;
            fish[idx].killed_by_predator = true;
        }
        for &idx in &predator_fed {
            fish[idx].hunger = (fish[idx].hunger - 0.5).max(0.0);
            fish[idx].energy = (fish[idx].energy + 0.2).min(1.0);
        }
    }

    fn update_fish_behavior(
        &mut self,
        fish: &mut [Fish],
        genomes: &std::collections::HashMap<u32, FishGenome>,
        config: &SimulationConfig,
        tick: u64,
    ) {
        // Pre-compute predator positions
        let predator_info: Vec<(f32, f32, f32, u32)> = fish
            .iter()
            .filter_map(|f| {
                let g = genomes.get(&f.genome_id)?;
                if g.aggression > 0.7 && f.is_alive {
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

        let mut new_fish: Vec<(Fish, FishGenome)> = Vec::new();
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
            let spawn_x = (fish[i].x + fish[partner_idx].x) / 2.0;
            let spawn_y = (fish[i].y + fish[partner_idx].y) / 2.0;
            let child = Fish::new(child_genome.id, spawn_x, spawn_y, rng);

            self.events.push(SimEvent::Birth {
                fish_id: child.id,
                genome_id: child_genome.id,
                parent_a: genome_a.id,
                parent_b: genome_b.id,
            });

            reproduced.push(fish[i].id);
            reproduced.push(partner_id);
            fish[i].last_reproduced_tick = Some(tick);
            fish[partner_idx].last_reproduced_tick = Some(tick);
            fish[i].behavior = BehaviorState::Swimming;
            fish[i].courting_partner = None;
            fish[partner_idx].behavior = BehaviorState::Swimming;
            fish[partner_idx].courting_partner = None;

            new_fish.push((child, child_genome));

            if fish.len() + new_fish.len() >= effective_capacity {
                break;
            }
        }

        for (child, genome) in new_fish {
            let gid = genome.id;
            genomes.insert(gid, genome);
            fish.push(child);
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
