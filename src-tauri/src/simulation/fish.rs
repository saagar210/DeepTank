use crate::simulation::config::SimulationConfig;
use crate::simulation::genome::FishGenome;
use rand::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BehaviorState {
    Swimming,
    Foraging,
    Fleeing,
    Satiated,
    Courting,
    Resting,
    Dying,
}

impl BehaviorState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Swimming => "swimming",
            Self::Foraging => "foraging",
            Self::Fleeing => "fleeing",
            Self::Satiated => "satiated",
            Self::Courting => "courting",
            Self::Resting => "resting",
            Self::Dying => "dying",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fish {
    pub id: u32,
    pub genome_id: u32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub vx: f32,
    pub vy: f32,
    pub heading: f32,
    pub age: u32,
    pub hunger: f32,
    pub health: f32,
    pub energy: f32,
    pub behavior: BehaviorState,
    pub meals_eaten: u32,
    pub last_reproduced_tick: Option<u64>,
    pub is_alive: bool,

    // Internal state not sent to frontend every frame
    pub prev_force_x: f32,
    pub prev_force_y: f32,
    pub satiated_timer: u32,
    pub courting_partner: Option<u32>,
    pub courting_timer: u32,
    pub dying_timer: u32,
    pub starvation_ticks: u32,
    pub fleeing_from: Option<u32>,
    pub killed_by_predator: bool,

    // Disease
    pub is_infected: bool,
    pub infection_timer: u32,
    pub recovery_timer: u32,
}

static NEXT_FISH_ID: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);

pub fn next_fish_id() -> u32 {
    NEXT_FISH_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

pub fn set_fish_id_counter(val: u32) {
    NEXT_FISH_ID.store(val, std::sync::atomic::Ordering::Relaxed);
}

impl Fish {
    pub fn new(genome_id: u32, x: f32, y: f32, rng: &mut impl Rng) -> Self {
        Self {
            id: next_fish_id(),
            genome_id,
            x,
            y,
            z: rng.gen_range(0.1..0.9),
            vx: rng.gen_range(-1.0..1.0),
            vy: rng.gen_range(-1.0..1.0),
            heading: rng.gen_range(0.0..std::f32::consts::TAU),
            age: 0,
            hunger: 0.3,
            health: 1.0,
            energy: 1.0,
            behavior: BehaviorState::Swimming,
            meals_eaten: 0,
            last_reproduced_tick: None,
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
        }
    }

    pub fn age_fraction(&self, genome: &FishGenome, base_lifespan: u32) -> f32 {
        let max_age = (base_lifespan as f32 * genome.lifespan_factor) as u32;
        if max_age == 0 { return 1.0; }
        self.age as f32 / max_age as f32
    }

    #[allow(dead_code)]
    pub fn max_lifespan(&self, genome: &FishGenome, base_lifespan: u32) -> u32 {
        (base_lifespan as f32 * genome.lifespan_factor) as u32
    }

    pub fn behavior_schooling_multiplier(&self) -> f32 {
        match self.behavior {
            BehaviorState::Foraging => 0.3,
            BehaviorState::Fleeing => 0.0,
            BehaviorState::Courting => 0.0,
            BehaviorState::Resting => 0.2,
            BehaviorState::Dying => 0.0,
            _ => 1.0,
        }
    }

    pub fn behavior_speed_multiplier(&self) -> f32 {
        match self.behavior {
            BehaviorState::Fleeing => 1.3,
            BehaviorState::Satiated => 0.7,
            BehaviorState::Resting => 0.3,
            BehaviorState::Dying => 0.4,
            _ => 1.0,
        }
    }

    pub fn update_behavior(
        &mut self,
        genome: &FishGenome,
        config: &SimulationConfig,
        _tick: u64,
        has_nearby_predator: bool,
        has_nearby_mate: Option<u32>,
        base_lifespan: u32,
        water_quality: f32,
    ) {
        let age_frac = self.age_fraction(genome, base_lifespan);

        // Aging
        self.age += 1;

        // Hunger increases
        self.hunger = (self.hunger + config.hunger_rate * genome.metabolism).min(1.0);

        // Energy depletion from movement
        let speed = (self.vx * self.vx + self.vy * self.vy).sqrt();
        let energy_cost = speed * 0.0001 * genome.metabolism;
        self.energy = (self.energy - energy_cost).max(0.0);
        // Energy recovery when slow
        if speed < 0.5 {
            self.energy = (self.energy + 0.0003).min(1.0);
        }

        // Water quality health effects
        if water_quality < 0.6 {
            self.health -= 0.0001 * (0.6 - water_quality);
        }
        if water_quality < 0.4 {
            self.health -= 0.0003;
        }
        if water_quality < 0.2 {
            self.health -= 0.001;
        }

        // Elder health degradation
        if age_frac > 0.85 {
            self.health -= 0.00005 * (1.0 + (1.0 - water_quality));
        }

        // Starvation tracking
        if self.hunger >= 1.0 {
            self.starvation_ticks += 1;
        } else {
            self.starvation_ticks = 0;
        }

        self.health = self.health.clamp(0.0, 1.0);

        // === State transitions ===

        // Any state → DYING
        if self.health <= 0.0 || age_frac >= 1.0 || self.starvation_ticks >= 200 {
            if self.behavior != BehaviorState::Dying {
                self.behavior = BehaviorState::Dying;
                self.dying_timer = 0;
            }
        }

        match self.behavior {
            BehaviorState::Dying => {
                self.dying_timer += 1;
                if self.dying_timer >= 90 {
                    // ~3 seconds at 30Hz
                    self.is_alive = false;
                }
            }
            BehaviorState::Swimming => {
                if has_nearby_predator {
                    self.behavior = BehaviorState::Fleeing;
                } else if self.hunger > 0.6 {
                    self.behavior = BehaviorState::Foraging;
                } else if self.energy < 0.2 {
                    self.behavior = BehaviorState::Resting;
                }
            }
            BehaviorState::Foraging => {
                if has_nearby_predator {
                    self.behavior = BehaviorState::Fleeing;
                } else if self.hunger < 0.3 {
                    self.behavior = BehaviorState::Swimming;
                }
            }
            BehaviorState::Fleeing => {
                if !has_nearby_predator {
                    self.behavior = BehaviorState::Swimming;
                    self.fleeing_from = None;
                }
            }
            BehaviorState::Satiated => {
                self.satiated_timer += 1;
                if self.satiated_timer > 60 {
                    // Check for mate
                    if let Some(mate_id) = has_nearby_mate {
                        self.behavior = BehaviorState::Courting;
                        self.courting_partner = Some(mate_id);
                        self.courting_timer = 0;
                    } else {
                        self.behavior = BehaviorState::Swimming;
                    }
                }
            }
            BehaviorState::Courting => {
                if has_nearby_predator {
                    self.behavior = BehaviorState::Fleeing;
                    self.courting_partner = None;
                    self.courting_timer = 0;
                } else {
                    self.courting_timer += 1;
                    if self.courting_timer >= 120 {
                        // 4 seconds - reproduction handled externally
                        self.behavior = BehaviorState::Swimming;
                        self.courting_partner = None;
                        self.courting_timer = 0;
                    }
                }
            }
            BehaviorState::Resting => {
                // Drift toward bottom
                if self.y < config.tank_height * 0.7 {
                    self.vy += 0.01;
                }
                self.energy = (self.energy + 0.001).min(1.0);
                if self.energy > 0.5 {
                    self.behavior = BehaviorState::Swimming;
                }
            }
        }
    }

    /// Called when this fish eats food
    pub fn eat(&mut self) {
        self.hunger = (self.hunger - 0.3).max(0.0);
        self.meals_eaten += 1;
        self.energy = (self.energy + 0.1).min(1.0);
        self.behavior = BehaviorState::Satiated;
        self.satiated_timer = 0;
    }

    /// Check if this fish can reproduce
    pub fn can_reproduce(
        &self,
        genome: &FishGenome,
        tick: u64,
        config: &SimulationConfig,
        base_lifespan: u32,
        water_quality: f32,
    ) -> bool {
        let age_frac = self.age_fraction(genome, base_lifespan);
        self.is_alive
            && self.hunger < 0.4
            && age_frac > genome.maturity_age
            && age_frac < 0.85
            && water_quality > 0.4
            && self.last_reproduced_tick
                .map(|t| tick - t > config.reproduction_cooldown as u64)
                .unwrap_or(true)
    }
}
