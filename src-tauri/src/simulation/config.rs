use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationConfig {
    // Boids
    pub separation_weight: f32,
    pub alignment_weight: f32,
    pub cohesion_weight: f32,
    pub separation_radius: f32,
    pub alignment_radius: f32,
    pub cohesion_radius: f32,
    pub base_max_speed: f32,
    pub max_force: f32,
    pub drag: f32,
    pub boundary_margin: f32,
    pub wander_strength: f32,

    // Ecosystem
    pub base_carrying_capacity: u32,
    pub hunger_rate: f32,
    pub food_decay_ticks: u32,
    pub fertility_scale: f32,
    pub reproduction_cooldown: u32,
    pub mutation_rate_small: f32,
    pub mutation_rate_large: f32,
    pub species_threshold: f32,
    pub species_min_members: u32,
    pub predation_size_ratio: f32,
    pub inbreeding_check_depth: u32,

    // Water
    pub water_degradation_per_fish: f32,
    pub water_recovery_rate: f32,
    pub plant_recovery_bonus: f32,

    // Environment
    pub current_direction: f32,
    pub current_strength: f32,
    pub day_night_cycle: bool,
    pub day_night_speed: f32, // 0 = real-time clock, >0 = accelerated sim cycle
    pub bubble_rate: f32,
    pub particle_density: f32,
    pub tank_width: f32,
    pub tank_height: f32,

    // Auto-feeder
    pub auto_feed_enabled: bool,
    pub auto_feed_interval: u32,
    pub auto_feed_amount: u32,

    // Persistence
    pub auto_save_interval: u32,
    pub snapshot_interval: u32,

    // Ollama
    pub ollama_enabled: bool,
    pub ollama_url: String,
    pub ollama_model: String,

    // Audio
    pub master_volume: f32,
    pub ambient_enabled: bool,
    pub event_sounds_enabled: bool,

    // Visual
    pub theme: String,

    // Eggs & Juveniles
    pub egg_hatch_time: u32,
    pub juvenile_duration: u32,

    // Environmental Events
    pub environmental_events_enabled: bool,
    pub event_frequency: f32,

    // Territory
    pub territory_enabled: bool,
    pub territory_claim_radius: f32,

    // Disease
    pub disease_enabled: bool,
    pub disease_infection_chance: f32,
    pub disease_spontaneous_chance: f32,
    pub disease_duration: u32,
    pub disease_damage: f32,
    pub disease_spread_radius: f32,
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            separation_weight: 1.5,
            alignment_weight: 1.0,
            cohesion_weight: 1.0,
            separation_radius: 25.0,
            alignment_radius: 50.0,
            cohesion_radius: 75.0,
            base_max_speed: 3.0,
            max_force: 0.1,
            drag: 0.98,
            boundary_margin: 60.0,
            wander_strength: 0.3,

            base_carrying_capacity: 100,
            hunger_rate: 0.0005,
            food_decay_ticks: 300,
            fertility_scale: 0.05,
            reproduction_cooldown: 300,
            mutation_rate_small: 0.10,
            mutation_rate_large: 0.02,
            species_threshold: 2.5,
            species_min_members: 3,
            predation_size_ratio: 0.6,
            inbreeding_check_depth: 2,

            water_degradation_per_fish: 0.00001,
            water_recovery_rate: 0.00005,
            plant_recovery_bonus: 0.00002,

            current_direction: 0.0,
            current_strength: 0.0,
            day_night_cycle: true,
            day_night_speed: 1.0,
            bubble_rate: 1.0,
            particle_density: 1.0,
            tank_width: 1200.0,
            tank_height: 800.0,

            auto_feed_enabled: false,
            auto_feed_interval: 600,
            auto_feed_amount: 4,

            auto_save_interval: 900,
            snapshot_interval: 300,

            ollama_enabled: true,
            ollama_url: "http://localhost:11434".to_string(),
            ollama_model: "llama3.2".to_string(),

            master_volume: 0.3,
            ambient_enabled: true,
            event_sounds_enabled: true,

            theme: "aquarium".to_string(),

            egg_hatch_time: 180,      // 6 seconds at 30Hz
            juvenile_duration: 300,   // 10 seconds at 30Hz

            environmental_events_enabled: true,
            event_frequency: 1.0,

            territory_enabled: true,
            territory_claim_radius: 60.0,

            disease_enabled: false,
            disease_infection_chance: 0.3,
            disease_spontaneous_chance: 0.00005,
            disease_duration: 600,
            disease_damage: 0.0005,
            disease_spread_radius: 40.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_sane_values() {
        let c = SimulationConfig::default();

        // Boids weights are positive
        assert!(c.separation_weight > 0.0);
        assert!(c.alignment_weight > 0.0);
        assert!(c.cohesion_weight > 0.0);

        // Radii are ordered: separation < alignment < cohesion
        assert!(c.separation_radius < c.alignment_radius);
        assert!(c.alignment_radius < c.cohesion_radius);

        // Speed/drag
        assert!(c.base_max_speed > 0.0);
        assert!(c.drag > 0.0 && c.drag < 1.0);

        // Mutation rates valid probabilities
        assert!(c.mutation_rate_small > 0.0 && c.mutation_rate_small < 1.0);
        assert!(c.mutation_rate_large > 0.0 && c.mutation_rate_large < 1.0);
        assert!(c.mutation_rate_large < c.mutation_rate_small);

        // Tank dimensions
        assert!(c.tank_width > 0.0);
        assert!(c.tank_height > 0.0);

        // Carrying capacity
        assert!(c.base_carrying_capacity > 0);

        // Water quality rates
        assert!(c.water_degradation_per_fish > 0.0);
        assert!(c.water_recovery_rate > 0.0);

        // Volume in range
        assert!(c.master_volume >= 0.0 && c.master_volume <= 1.0);
    }

    #[test]
    fn default_config_clone_equals() {
        let c1 = SimulationConfig::default();
        let c2 = c1.clone();
        // Spot-check some fields (no PartialEq derive, but we can compare manually)
        assert_eq!(c1.base_max_speed, c2.base_max_speed);
        assert_eq!(c1.tank_width, c2.tank_width);
        assert_eq!(c1.ollama_model, c2.ollama_model);
        assert_eq!(c1.theme, c2.theme);
    }

    #[test]
    fn config_serialization_roundtrip() {
        let c = SimulationConfig::default();
        let json = serde_json::to_string(&c).expect("serialize");
        let c2: SimulationConfig = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(c.base_max_speed, c2.base_max_speed);
        assert_eq!(c.base_carrying_capacity, c2.base_carrying_capacity);
        assert_eq!(c.ollama_url, c2.ollama_url);
    }
}
