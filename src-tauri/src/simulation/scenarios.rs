use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scenario {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub goals: Vec<ScenarioGoal>,
    pub initial_fish_count: u32,
    /// Config overrides as (key, value) pairs applied on top of defaults
    pub config_overrides: Vec<(&'static str, f32)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScenarioGoal {
    ReachPopulation(u32),
    ReachGeneration(u32),
    ReachSpeciesCount(u32),
    SurviveTicks(u64),
    TraitAbove { trait_name: String, value: f32 },
    TraitBelow { trait_name: String, value: f32 },
    DiversityAbove(f32),
}

impl ScenarioGoal {
    pub fn description(&self) -> String {
        match self {
            Self::ReachPopulation(n) => format!("Reach population of {}", n),
            Self::ReachGeneration(n) => format!("Reach generation {}", n),
            Self::ReachSpeciesCount(n) => format!("Maintain {} simultaneous species", n),
            Self::SurviveTicks(n) => format!("Survive for {} ticks", n),
            Self::TraitAbove { trait_name, value } => format!("Breed a fish with {} > {:.1}", trait_name, value),
            Self::TraitBelow { trait_name, value } => format!("Reduce max {} below {:.1}", trait_name, value),
            Self::DiversityAbove(v) => format!("Keep genetic diversity above {:.0}%", v * 100.0),
        }
    }
}

pub fn all_scenarios() -> Vec<Scenario> {
    vec![
        Scenario {
            id: "survival",
            name: "Survival",
            description: "Start with 5 fish in harsh conditions. Reach a population of 30.",
            goals: vec![ScenarioGoal::ReachPopulation(30)],
            initial_fish_count: 5,
            config_overrides: vec![
                ("hunger_rate", 0.001),
                ("auto_feed_enabled", 0.0),
            ],
        },
        Scenario {
            id: "apex_predator",
            name: "Apex Predator",
            description: "Breed a fish with aggression > 0.95 and speed > 1.8.",
            goals: vec![
                ScenarioGoal::TraitAbove { trait_name: "aggression".to_string(), value: 0.95 },
                ScenarioGoal::TraitAbove { trait_name: "speed".to_string(), value: 1.8 },
            ],
            initial_fish_count: 15,
            config_overrides: vec![],
        },
        Scenario {
            id: "biodiversity",
            name: "Biodiversity Challenge",
            description: "Maintain 5+ simultaneous species for 5000 ticks.",
            goals: vec![
                ScenarioGoal::ReachSpeciesCount(5),
                ScenarioGoal::SurviveTicks(5000),
            ],
            initial_fish_count: 20,
            config_overrides: vec![
                ("mutation_rate_large", 0.05),
            ],
        },
        Scenario {
            id: "peaceful_kingdom",
            name: "Peaceful Kingdom",
            description: "Through selective pressure, reduce max aggression below 0.2.",
            goals: vec![
                ScenarioGoal::TraitBelow { trait_name: "aggression".to_string(), value: 0.2 },
            ],
            initial_fish_count: 20,
            config_overrides: vec![],
        },
        Scenario {
            id: "ice_age",
            name: "Ice Age",
            description: "A permanent cold snap. Reach generation 50.",
            goals: vec![ScenarioGoal::ReachGeneration(50)],
            initial_fish_count: 15,
            config_overrides: vec![],
        },
    ]
}

/// Check if all goals of the active scenario are met.
/// Returns a Vec of (goal_index, is_complete) pairs.
pub fn check_goals(
    scenario: &Scenario,
    population: u32,
    max_generation: u32,
    species_count: u32,
    tick: u64,
    diversity: f32,
    genomes: &std::collections::HashMap<u32, super::genome::FishGenome>,
    fish: &[super::fish::Fish],
) -> Vec<(usize, bool)> {
    scenario.goals.iter().enumerate().map(|(i, goal)| {
        let met = match goal {
            ScenarioGoal::ReachPopulation(n) => population >= *n,
            ScenarioGoal::ReachGeneration(n) => max_generation >= *n,
            ScenarioGoal::ReachSpeciesCount(n) => species_count >= *n,
            ScenarioGoal::SurviveTicks(n) => tick >= *n,
            ScenarioGoal::DiversityAbove(v) => diversity >= *v,
            ScenarioGoal::TraitAbove { trait_name, value } => {
                fish.iter().any(|f| {
                    if let Some(g) = genomes.get(&f.genome_id) {
                        get_trait(g, trait_name) > *value
                    } else {
                        false
                    }
                })
            }
            ScenarioGoal::TraitBelow { trait_name, value } => {
                // All living fish must have the trait below the value
                fish.iter().all(|f| {
                    if let Some(g) = genomes.get(&f.genome_id) {
                        get_trait(g, trait_name) < *value
                    } else {
                        true
                    }
                }) && !fish.is_empty()
            }
        };
        (i, met)
    }).collect()
}

fn get_trait(g: &super::genome::FishGenome, name: &str) -> f32 {
    match name {
        "speed" => g.speed,
        "aggression" => g.aggression,
        "boldness" => g.boldness,
        "school_affinity" => g.school_affinity,
        "metabolism" => g.metabolism,
        "size" | "body_length" => g.body_length,
        "disease_resistance" => g.disease_resistance,
        _ => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulation::fish::Fish;
    use crate::simulation::genome::FishGenome;
    use rand::SeedableRng;
    use rand::rngs::StdRng;
    use std::collections::HashMap;

    fn seeded_rng() -> StdRng {
        StdRng::seed_from_u64(42)
    }

    #[test]
    fn all_scenarios_valid() {
        let scenarios = all_scenarios();
        assert_eq!(scenarios.len(), 5);
        for s in &scenarios {
            assert!(!s.id.is_empty());
            assert!(!s.name.is_empty());
            assert!(!s.goals.is_empty());
            assert!(s.initial_fish_count > 0);
        }
    }

    #[test]
    fn scenario_ids_unique() {
        let scenarios = all_scenarios();
        let mut ids: Vec<&str> = scenarios.iter().map(|s| s.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), scenarios.len());
    }

    #[test]
    fn goal_descriptions_nonempty() {
        let scenarios = all_scenarios();
        for s in &scenarios {
            for g in &s.goals {
                let desc = g.description();
                assert!(!desc.is_empty(), "Goal description for {} should not be empty", s.id);
            }
        }
    }

    #[test]
    fn check_goals_population() {
        let scenario = &all_scenarios()[0]; // survival: ReachPopulation(30)
        let genomes = HashMap::new();
        let fish: Vec<Fish> = Vec::new();

        let results = check_goals(scenario, 29, 0, 0, 0, 0.0, &genomes, &fish);
        assert!(!results[0].1, "29 < 30 should not meet goal");

        let results = check_goals(scenario, 30, 0, 0, 0, 0.0, &genomes, &fish);
        assert!(results[0].1, "30 >= 30 should meet goal");
    }

    #[test]
    fn check_goals_trait_above() {
        let mut rng = seeded_rng();
        let mut genome = FishGenome::random(&mut rng);
        genome.aggression = 0.96;
        genome.speed = 1.9;
        let fish = vec![Fish::new(genome.id, 100.0, 100.0, &mut rng)];
        let mut genomes = HashMap::new();
        genomes.insert(genome.id, genome);

        let scenario = &all_scenarios()[1]; // apex_predator
        let results = check_goals(scenario, 1, 0, 0, 0, 0.0, &genomes, &fish);
        assert!(results[0].1, "Aggression 0.96 > 0.95");
        assert!(results[1].1, "Speed 1.9 > 1.8");
    }

    #[test]
    fn check_goals_trait_below() {
        let mut rng = seeded_rng();
        let mut genome = FishGenome::random(&mut rng);
        genome.aggression = 0.1;
        let fish = vec![Fish::new(genome.id, 100.0, 100.0, &mut rng)];
        let mut genomes = HashMap::new();
        genomes.insert(genome.id, genome);

        let scenario = &all_scenarios()[3]; // peaceful_kingdom: aggression < 0.2
        let results = check_goals(scenario, 1, 0, 0, 0, 0.0, &genomes, &fish);
        assert!(results[0].1, "Aggression 0.1 < 0.2");
    }

    #[test]
    fn check_goals_empty_fish_fails_trait_below() {
        let genomes = HashMap::new();
        let fish: Vec<Fish> = Vec::new();

        let scenario = &all_scenarios()[3]; // peaceful_kingdom
        let results = check_goals(scenario, 0, 0, 0, 0, 0.0, &genomes, &fish);
        assert!(!results[0].1, "Empty fish list should not meet TraitBelow");
    }
}
