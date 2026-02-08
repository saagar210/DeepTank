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
    Hunting,
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
            Self::Hunting => "hunting",
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

    // Juvenile stage
    pub is_juvenile: bool,
    pub juvenile_timer: u32,

    // Stress (from glass taps)
    pub stress: f32,
    pub tap_flee_timer: u32,

    // Hunting (predation overhaul)
    pub hunting_target: Option<u32>,  // target fish id
    pub hunting_timer: u32,

    // Territory
    pub territory_center: Option<(f32, f32)>,
    pub territory_radius: f32,

    // Naming & favorites
    pub custom_name: Option<String>,
    pub is_favorite: bool,

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
            is_juvenile: false,
            juvenile_timer: 0,
            stress: 0.0,
            tap_flee_timer: 0,
            hunting_target: None,
            hunting_timer: 0,
            territory_center: None,
            territory_radius: 0.0,
            custom_name: None,
            is_favorite: false,
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

    pub fn behavior_schooling_multiplier(&self) -> f32 {
        match self.behavior {
            BehaviorState::Foraging => 0.3,
            BehaviorState::Fleeing => 0.0,
            BehaviorState::Courting => 0.0,
            BehaviorState::Resting => 0.2,
            BehaviorState::Hunting => 0.0,
            BehaviorState::Dying => 0.0,
            _ => 1.0,
        }
    }

    pub fn behavior_speed_multiplier(&self) -> f32 {
        match self.behavior {
            BehaviorState::Fleeing => 1.4,  // improved prey evasion (was 1.3)
            BehaviorState::Hunting => 1.2,
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
        time_of_day: f32,
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

        // Juvenile growth
        if self.is_juvenile {
            self.juvenile_timer += 1;
            if self.juvenile_timer >= config.juvenile_duration {
                self.is_juvenile = false;
            }
        }

        // Stress decay and damage
        if self.stress > 0.0 {
            self.stress = (self.stress - 0.001).max(0.0);
        }
        if self.stress > 0.5 {
            self.health -= 0.0002;
        }
        // Tap flee timer countdown
        if self.tap_flee_timer > 0 {
            self.tap_flee_timer -= 1;
            if self.tap_flee_timer == 0 && self.behavior == BehaviorState::Fleeing && self.fleeing_from.is_none() {
                self.behavior = BehaviorState::Swimming;
            }
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
            BehaviorState::Hunting => {
                // Hunting state is managed by process_predation, not the behavior FSM.
                // Only override: dying check above can interrupt hunting.
            }
            BehaviorState::Swimming => {
                let is_night = time_of_day >= 21.0 || time_of_day < 5.0;
                let is_nocturnal = genome.boldness > 0.7;
                if has_nearby_predator {
                    self.behavior = BehaviorState::Fleeing;
                } else if self.hunger > 0.6 {
                    self.behavior = BehaviorState::Foraging;
                } else if self.energy < 0.2 {
                    self.behavior = BehaviorState::Resting;
                } else if is_night && !is_nocturnal && config.day_night_cycle {
                    // Non-nocturnal fish rest at night (40% chance per tick when swimming)
                    if self.age % 5 == 0 { // ~40% effective per second at 30Hz
                        self.behavior = BehaviorState::Resting;
                    }
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
            && !self.is_juvenile
            && self.hunger < 0.4
            && age_frac > genome.maturity_age
            && age_frac < 0.85
            && water_quality > 0.4
            && self.last_reproduced_tick
                .map(|t| tick - t > config.reproduction_cooldown as u64)
                .unwrap_or(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulation::genome::FishGenome;
    use crate::simulation::config::SimulationConfig;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn seeded_rng() -> StdRng {
        StdRng::seed_from_u64(42)
    }

    fn test_genome() -> FishGenome {
        let mut rng = seeded_rng();
        FishGenome::random(&mut rng)
    }

    #[test]
    fn fish_new_defaults() {
        let mut rng = seeded_rng();
        let genome = test_genome();
        let f = Fish::new(genome.id, 100.0, 200.0, &mut rng);

        assert!(f.is_alive);
        assert_eq!(f.age, 0);
        assert!((f.hunger - 0.3).abs() < 0.01);
        assert!((f.health - 1.0).abs() < 0.01);
        assert!((f.energy - 1.0).abs() < 0.01);
        assert_eq!(f.behavior, BehaviorState::Swimming);
        assert_eq!(f.meals_eaten, 0);
        assert!(!f.is_juvenile);
        assert!(!f.is_infected);
        assert!(!f.is_favorite);
        assert!(f.custom_name.is_none());
        assert!(f.x == 100.0 && f.y == 200.0);
    }

    #[test]
    fn fish_ids_are_unique() {
        let mut rng = seeded_rng();
        let genome = test_genome();
        let f1 = Fish::new(genome.id, 0.0, 0.0, &mut rng);
        let f2 = Fish::new(genome.id, 0.0, 0.0, &mut rng);
        assert_ne!(f1.id, f2.id);
    }

    #[test]
    fn eat_reduces_hunger_and_changes_state() {
        let mut rng = seeded_rng();
        let genome = test_genome();
        let mut f = Fish::new(genome.id, 0.0, 0.0, &mut rng);
        f.hunger = 0.8;
        f.behavior = BehaviorState::Foraging;

        f.eat();

        assert!((f.hunger - 0.5).abs() < 0.01);
        assert_eq!(f.meals_eaten, 1);
        assert_eq!(f.behavior, BehaviorState::Satiated);
        assert!((f.energy - 1.0).abs() < 0.1); // energy capped at 1.0
    }

    #[test]
    fn eat_hunger_floors_at_zero() {
        let mut rng = seeded_rng();
        let genome = test_genome();
        let mut f = Fish::new(genome.id, 0.0, 0.0, &mut rng);
        f.hunger = 0.1;
        f.eat();
        assert!(f.hunger >= 0.0);
    }

    #[test]
    fn age_fraction_calculation() {
        let mut rng = seeded_rng();
        let genome = test_genome();
        let mut f = Fish::new(genome.id, 0.0, 0.0, &mut rng);

        f.age = 0;
        assert!((f.age_fraction(&genome, 20_000) - 0.0).abs() < 0.01);

        f.age = 10_000;
        let expected = 10_000.0 / (20_000.0 * genome.lifespan_factor);
        assert!((f.age_fraction(&genome, 20_000) - expected).abs() < 0.01);
    }

    #[test]
    fn behavior_speed_multipliers() {
        let mut rng = seeded_rng();
        let genome = test_genome();
        let mut f = Fish::new(genome.id, 0.0, 0.0, &mut rng);

        f.behavior = BehaviorState::Swimming;
        assert!((f.behavior_speed_multiplier() - 1.0).abs() < 0.01);

        f.behavior = BehaviorState::Fleeing;
        assert!((f.behavior_speed_multiplier() - 1.4).abs() < 0.01);

        f.behavior = BehaviorState::Hunting;
        assert!((f.behavior_speed_multiplier() - 1.2).abs() < 0.01);

        f.behavior = BehaviorState::Resting;
        assert!((f.behavior_speed_multiplier() - 0.3).abs() < 0.01);
    }

    #[test]
    fn behavior_schooling_multipliers() {
        let mut rng = seeded_rng();
        let genome = test_genome();
        let mut f = Fish::new(genome.id, 0.0, 0.0, &mut rng);

        f.behavior = BehaviorState::Swimming;
        assert!((f.behavior_schooling_multiplier() - 1.0).abs() < 0.01);

        f.behavior = BehaviorState::Fleeing;
        assert!((f.behavior_schooling_multiplier() - 0.0).abs() < 0.01);

        f.behavior = BehaviorState::Hunting;
        assert!((f.behavior_schooling_multiplier() - 0.0).abs() < 0.01);
    }

    #[test]
    fn can_reproduce_basic_conditions() {
        let mut rng = seeded_rng();
        let mut genome = test_genome();
        genome.maturity_age = 0.3;
        genome.lifespan_factor = 1.0;
        let config = SimulationConfig::default();

        let mut f = Fish::new(genome.id, 0.0, 0.0, &mut rng);
        f.age = 8000; // age_frac = 8000/20000 = 0.4, above maturity_age 0.3
        f.hunger = 0.2;
        f.is_alive = true;
        f.is_juvenile = false;

        assert!(f.can_reproduce(&genome, 1000, &config, 20_000, 0.8));
    }

    #[test]
    fn cannot_reproduce_if_hungry() {
        let mut rng = seeded_rng();
        let mut genome = test_genome();
        genome.maturity_age = 0.3;
        genome.lifespan_factor = 1.0;
        let config = SimulationConfig::default();

        let mut f = Fish::new(genome.id, 0.0, 0.0, &mut rng);
        f.age = 8000;
        f.hunger = 0.5; // above 0.4 threshold
        assert!(!f.can_reproduce(&genome, 1000, &config, 20_000, 0.8));
    }

    #[test]
    fn cannot_reproduce_if_juvenile() {
        let mut rng = seeded_rng();
        let mut genome = test_genome();
        genome.maturity_age = 0.3;
        genome.lifespan_factor = 1.0;
        let config = SimulationConfig::default();

        let mut f = Fish::new(genome.id, 0.0, 0.0, &mut rng);
        f.age = 8000;
        f.hunger = 0.2;
        f.is_juvenile = true;
        assert!(!f.can_reproduce(&genome, 1000, &config, 20_000, 0.8));
    }

    #[test]
    fn cannot_reproduce_poor_water() {
        let mut rng = seeded_rng();
        let mut genome = test_genome();
        genome.maturity_age = 0.3;
        genome.lifespan_factor = 1.0;
        let config = SimulationConfig::default();

        let mut f = Fish::new(genome.id, 0.0, 0.0, &mut rng);
        f.age = 8000;
        f.hunger = 0.2;
        assert!(!f.can_reproduce(&genome, 1000, &config, 20_000, 0.3)); // water < 0.4
    }

    #[test]
    fn behavior_state_as_str() {
        assert_eq!(BehaviorState::Swimming.as_str(), "swimming");
        assert_eq!(BehaviorState::Foraging.as_str(), "foraging");
        assert_eq!(BehaviorState::Fleeing.as_str(), "fleeing");
        assert_eq!(BehaviorState::Satiated.as_str(), "satiated");
        assert_eq!(BehaviorState::Courting.as_str(), "courting");
        assert_eq!(BehaviorState::Resting.as_str(), "resting");
        assert_eq!(BehaviorState::Hunting.as_str(), "hunting");
        assert_eq!(BehaviorState::Dying.as_str(), "dying");
    }

    #[test]
    fn dying_fish_eventually_dies() {
        let mut rng = seeded_rng();
        let genome = test_genome();
        let config = SimulationConfig::default();
        let mut f = Fish::new(genome.id, 400.0, 400.0, &mut rng);
        f.health = 0.0; // trigger dying

        for tick in 0..200 {
            f.update_behavior(&genome, &config, tick, false, None, 20_000, 1.0, 12.0);
            if !f.is_alive { break; }
        }
        assert!(!f.is_alive, "Fish should die within 200 ticks of health=0");
    }
}
