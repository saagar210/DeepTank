use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Achievement {
    pub id: String,
    pub name: String,
    pub description: String,
    pub unlocked_at_tick: Option<u64>,
}

pub fn default_achievements() -> Vec<Achievement> {
    vec![
        Achievement { id: "first_birth".into(), name: "It's Alive!".into(), description: "Witness the first birth in your aquarium".into(), unlocked_at_tick: None },
        Achievement { id: "first_speciation".into(), name: "Darwin's Delight".into(), description: "A new species has been discovered".into(), unlocked_at_tick: None },
        Achievement { id: "gen_100".into(), name: "Centurion".into(), description: "Reach generation 100".into(), unlocked_at_tick: None },
        Achievement { id: "five_species".into(), name: "Biodiversity".into(), description: "Have 5 species alive simultaneously".into(), unlocked_at_tick: None },
        Achievement { id: "apex_predator".into(), name: "Apex Predator".into(), description: "A fish with aggression > 0.95 exists".into(), unlocked_at_tick: None },
        Achievement { id: "crystal_clear".into(), name: "Crystal Clear".into(), description: "Water quality above 95% for 1000 ticks".into(), unlocked_at_tick: None },
        Achievement { id: "speed_demon".into(), name: "Speed Demon".into(), description: "A fish with speed > 1.9 exists".into(), unlocked_at_tick: None },
        Achievement { id: "population_50".into(), name: "Thriving Colony".into(), description: "Reach a population of 50".into(), unlocked_at_tick: None },
        Achievement { id: "first_extinction".into(), name: "Gone Forever".into(), description: "A species has gone extinct".into(), unlocked_at_tick: None },
        Achievement { id: "first_predation".into(), name: "Circle of Life".into(), description: "Witness a predation event".into(), unlocked_at_tick: None },
        Achievement { id: "gen_10".into(), name: "Getting Started".into(), description: "Reach generation 10".into(), unlocked_at_tick: None },
        Achievement { id: "meals_100".into(), name: "Feast Mode".into(), description: "A single fish eats 100 meals".into(), unlocked_at_tick: None },
        Achievement { id: "tiny_fish".into(), name: "Fun Size".into(), description: "A fish with body_length < 0.65 exists".into(), unlocked_at_tick: None },
        Achievement { id: "giant_fish".into(), name: "Absolute Unit".into(), description: "A fish with body_length > 1.95 exists".into(), unlocked_at_tick: None },
        Achievement { id: "full_tank".into(), name: "Standing Room Only".into(), description: "Reach maximum carrying capacity".into(), unlocked_at_tick: None },
    ]
}

pub fn check_achievements(
    achievements: &mut Vec<Achievement>,
    tick: u64,
    population: u32,
    max_generation: u32,
    species_count: u32,
    _water_quality: f32,
    high_wq_streak: u32,
    had_birth: bool,
    had_speciation: bool,
    had_extinction: bool,
    had_predation: bool,
    max_aggression: f32,
    max_speed: f32,
    max_meals: u32,
    min_body: f32,
    max_body: f32,
    carrying_capacity: u32,
) -> Vec<String> {
    let mut newly_unlocked = Vec::new();

    for a in achievements.iter_mut() {
        if a.unlocked_at_tick.is_some() { continue; }
        let unlocked = match a.id.as_str() {
            "first_birth" => had_birth,
            "first_speciation" => had_speciation,
            "gen_100" => max_generation >= 100,
            "gen_10" => max_generation >= 10,
            "five_species" => species_count >= 5,
            "apex_predator" => max_aggression > 0.95,
            "crystal_clear" => high_wq_streak >= 1000,
            "speed_demon" => max_speed > 1.9,
            "population_50" => population >= 50,
            "first_extinction" => had_extinction,
            "first_predation" => had_predation,
            "meals_100" => max_meals >= 100,
            "tiny_fish" => min_body < 0.65 && min_body > 0.0,
            "giant_fish" => max_body > 1.95,
            "full_tank" => population >= carrying_capacity,
            _ => false,
        };
        if unlocked {
            a.unlocked_at_tick = Some(tick);
            newly_unlocked.push(a.name.clone());
        }
    }
    newly_unlocked
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_achievements_count() {
        let achievements = default_achievements();
        assert_eq!(achievements.len(), 15);
    }

    #[test]
    fn all_achievements_start_locked() {
        let achievements = default_achievements();
        for a in &achievements {
            assert!(a.unlocked_at_tick.is_none(), "{} should start locked", a.id);
        }
    }

    #[test]
    fn unique_achievement_ids() {
        let achievements = default_achievements();
        let mut ids: Vec<&str> = achievements.iter().map(|a| a.id.as_str()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), achievements.len(), "Achievement IDs must be unique");
    }

    #[test]
    fn first_birth_unlocks() {
        let mut achievements = default_achievements();
        let unlocked = check_achievements(
            &mut achievements, 100, 0, 0, 0, 1.0, 0,
            true, false, false, false,
            0.0, 0.0, 0, 1.0, 1.0, 100,
        );
        assert!(unlocked.contains(&"It's Alive!".to_string()));
    }

    #[test]
    fn population_50_unlocks() {
        let mut achievements = default_achievements();
        let unlocked = check_achievements(
            &mut achievements, 100, 50, 0, 0, 1.0, 0,
            false, false, false, false,
            0.0, 0.0, 0, 1.0, 1.0, 100,
        );
        assert!(unlocked.contains(&"Thriving Colony".to_string()));
    }

    #[test]
    fn achievement_only_unlocks_once() {
        let mut achievements = default_achievements();
        let unlocked1 = check_achievements(
            &mut achievements, 100, 50, 0, 0, 1.0, 0,
            true, false, false, false,
            0.0, 0.0, 0, 1.0, 1.0, 100,
        );
        assert!(!unlocked1.is_empty());

        // Call again with same conditions — should NOT unlock again
        let unlocked2 = check_achievements(
            &mut achievements, 200, 50, 0, 0, 1.0, 0,
            true, false, false, false,
            0.0, 0.0, 0, 1.0, 1.0, 100,
        );
        assert!(unlocked2.is_empty(), "Already-unlocked achievements shouldn't re-trigger");
    }

    #[test]
    fn apex_predator_unlocks() {
        let mut achievements = default_achievements();
        let unlocked = check_achievements(
            &mut achievements, 100, 10, 0, 0, 1.0, 0,
            false, false, false, false,
            0.96, 0.0, 0, 1.0, 1.0, 100,
        );
        assert!(unlocked.contains(&"Apex Predator".to_string()));
    }

    #[test]
    fn crystal_clear_unlocks() {
        let mut achievements = default_achievements();
        let unlocked = check_achievements(
            &mut achievements, 100, 10, 0, 0, 0.96, 1001,
            false, false, false, false,
            0.0, 0.0, 0, 1.0, 1.0, 100,
        );
        assert!(unlocked.contains(&"Crystal Clear".to_string()));
    }

    #[test]
    fn full_tank_unlocks() {
        let mut achievements = default_achievements();
        let unlocked = check_achievements(
            &mut achievements, 100, 100, 0, 0, 1.0, 0,
            false, false, false, false,
            0.0, 0.0, 0, 1.0, 1.0, 100,
        );
        assert!(unlocked.contains(&"Standing Room Only".to_string()));
    }
}
