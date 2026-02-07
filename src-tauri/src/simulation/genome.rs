use rand::prelude::*;
use rand_distr::Normal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Sex {
    Male,
    Female,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatternGene {
    Solid,
    Striped { angle: f32 },
    Spotted { density: f32 },
    Gradient { direction: f32 },
    Bicolor { split: f32 },
}

impl PatternGene {
    pub fn random(rng: &mut impl Rng) -> Self {
        match rng.gen_range(0..5) {
            0 => PatternGene::Solid,
            1 => PatternGene::Striped {
                angle: rng.gen_range(0.0..180.0),
            },
            2 => PatternGene::Spotted {
                density: rng.gen_range(0.2..1.0),
            },
            3 => PatternGene::Gradient {
                direction: rng.gen_range(0.0..360.0),
            },
            _ => PatternGene::Bicolor {
                split: rng.gen_range(0.3..0.7),
            },
        }
    }

    pub fn type_index(&self) -> u8 {
        match self {
            PatternGene::Solid => 0,
            PatternGene::Striped { .. } => 1,
            PatternGene::Spotted { .. } => 2,
            PatternGene::Gradient { .. } => 3,
            PatternGene::Bicolor { .. } => 4,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FishGenome {
    // Identity
    pub id: u32,
    pub generation: u32,
    pub parent_a: Option<u32>,
    pub parent_b: Option<u32>,
    pub sex: Sex,

    // Appearance
    pub base_hue: f32,
    pub saturation: f32,
    pub lightness: f32,
    pub body_length: f32,
    pub body_width: f32,
    pub tail_size: f32,
    pub dorsal_fin_size: f32,
    pub pectoral_fin_size: f32,
    pub pattern: PatternGene,
    pub pattern_intensity: f32,
    pub pattern_color_offset: f32,
    pub eye_size: f32,

    // Behavior
    pub speed: f32,
    pub aggression: f32,
    pub school_affinity: f32,
    pub curiosity: f32,
    pub boldness: f32,

    // Lifecycle
    pub metabolism: f32,
    pub fertility: f32,
    pub lifespan_factor: f32,
    pub maturity_age: f32,
    pub disease_resistance: f32,
}

static NEXT_GENOME_ID: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);

pub fn next_genome_id() -> u32 {
    NEXT_GENOME_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

pub fn set_genome_id_counter(val: u32) {
    NEXT_GENOME_ID.store(val, std::sync::atomic::Ordering::Relaxed);
}

impl FishGenome {
    pub fn random(rng: &mut impl Rng) -> Self {
        Self {
            id: next_genome_id(),
            generation: 0,
            parent_a: None,
            parent_b: None,
            sex: if rng.gen_bool(0.5) { Sex::Male } else { Sex::Female },

            base_hue: rng.gen_range(0.0..360.0),
            saturation: rng.gen_range(0.3..1.0),
            lightness: rng.gen_range(0.3..0.7),
            body_length: rng.gen_range(0.6..2.0),
            body_width: rng.gen_range(0.5..1.5),
            tail_size: rng.gen_range(0.5..2.0),
            dorsal_fin_size: rng.gen_range(0.3..1.5),
            pectoral_fin_size: rng.gen_range(0.3..1.5),
            pattern: PatternGene::random(rng),
            pattern_intensity: rng.gen_range(0.0..1.0),
            pattern_color_offset: rng.gen_range(0.0..180.0),
            eye_size: rng.gen_range(0.5..1.5),

            speed: rng.gen_range(0.5..2.0),
            aggression: rng.gen_range(0.2..0.5), // moderate for initial pop
            school_affinity: rng.gen_range(0.0..1.0),
            curiosity: rng.gen_range(0.0..1.0),
            boldness: rng.gen_range(0.0..1.0),

            metabolism: rng.gen_range(0.5..2.0),
            fertility: rng.gen_range(0.3..1.0),
            lifespan_factor: rng.gen_range(0.5..2.0),
            maturity_age: rng.gen_range(0.3..0.7),
            disease_resistance: rng.gen_range(0.2..0.8),
        }
    }

    /// Generate initial population with deliberate diversity
    pub fn random_diverse(rng: &mut impl Rng, index: usize, total: usize) -> Self {
        let mut genome = Self::random(rng);
        // Distribute hues evenly across the color wheel
        genome.base_hue = (360.0 / total as f32) * index as f32 + rng.gen_range(-15.0..15.0);
        genome.base_hue = genome.base_hue.rem_euclid(360.0);
        // Ensure at least 3 pattern types by cycling
        genome.pattern = match index % 5 {
            0 => PatternGene::Solid,
            1 => PatternGene::Striped { angle: rng.gen_range(0.0..180.0) },
            2 => PatternGene::Spotted { density: rng.gen_range(0.2..1.0) },
            3 => PatternGene::Gradient { direction: rng.gen_range(0.0..360.0) },
            _ => PatternGene::Bicolor { split: rng.gen_range(0.3..0.7) },
        };
        // Vary body sizes
        genome.body_length = 0.6 + (1.4 / total as f32) * index as f32 + rng.gen_range(-0.1..0.1);
        genome.body_length = genome.body_length.clamp(0.6, 2.0);
        // Alternate sex
        genome.sex = if index % 2 == 0 { Sex::Male } else { Sex::Female };
        genome
    }

    pub fn inherit(parent_a: &FishGenome, parent_b: &FishGenome, rng: &mut impl Rng, inbred: bool, rate_large: f32, rate_small: f32) -> Self {
        let mutation_scale = if inbred { 1.5 } else { 1.0 };
        let gen = parent_a.generation.max(parent_b.generation) + 1;

        let mut child = Self {
            id: next_genome_id(),
            generation: gen,
            parent_a: Some(parent_a.id),
            parent_b: Some(parent_b.id),
            sex: if rng.gen_bool(0.5) { Sex::Male } else { Sex::Female },

            base_hue: inherit_hue(parent_a.base_hue, parent_b.base_hue, rng, mutation_scale, rate_large, rate_small),
            saturation: inherit_trait(parent_a.saturation, parent_b.saturation, 0.3, 1.0, rng, mutation_scale, rate_large, rate_small),
            lightness: inherit_trait(parent_a.lightness, parent_b.lightness, 0.3, 0.7, rng, mutation_scale, rate_large, rate_small),
            body_length: inherit_trait(parent_a.body_length, parent_b.body_length, 0.6, 2.0, rng, mutation_scale, rate_large, rate_small),
            body_width: inherit_trait(parent_a.body_width, parent_b.body_width, 0.5, 1.5, rng, mutation_scale, rate_large, rate_small),
            tail_size: inherit_trait(parent_a.tail_size, parent_b.tail_size, 0.5, 2.0, rng, mutation_scale, rate_large, rate_small),
            dorsal_fin_size: inherit_trait(parent_a.dorsal_fin_size, parent_b.dorsal_fin_size, 0.3, 1.5, rng, mutation_scale, rate_large, rate_small),
            pectoral_fin_size: inherit_trait(parent_a.pectoral_fin_size, parent_b.pectoral_fin_size, 0.3, 1.5, rng, mutation_scale, rate_large, rate_small),
            pattern: inherit_pattern(&parent_a.pattern, &parent_b.pattern, rng),
            pattern_intensity: inherit_trait(parent_a.pattern_intensity, parent_b.pattern_intensity, 0.0, 1.0, rng, mutation_scale, rate_large, rate_small),
            pattern_color_offset: inherit_trait(parent_a.pattern_color_offset, parent_b.pattern_color_offset, 0.0, 180.0, rng, mutation_scale, rate_large, rate_small),
            eye_size: inherit_trait(parent_a.eye_size, parent_b.eye_size, 0.5, 1.5, rng, mutation_scale, rate_large, rate_small),

            speed: inherit_trait(parent_a.speed, parent_b.speed, 0.5, 2.0, rng, mutation_scale, rate_large, rate_small),
            aggression: inherit_trait(parent_a.aggression, parent_b.aggression, 0.0, 1.0, rng, mutation_scale, rate_large, rate_small),
            school_affinity: inherit_trait(parent_a.school_affinity, parent_b.school_affinity, 0.0, 1.0, rng, mutation_scale, rate_large, rate_small),
            curiosity: inherit_trait(parent_a.curiosity, parent_b.curiosity, 0.0, 1.0, rng, mutation_scale, rate_large, rate_small),
            boldness: inherit_trait(parent_a.boldness, parent_b.boldness, 0.0, 1.0, rng, mutation_scale, rate_large, rate_small),

            metabolism: inherit_trait(parent_a.metabolism, parent_b.metabolism, 0.5, 2.0, rng, mutation_scale, rate_large, rate_small),
            fertility: inherit_trait(parent_a.fertility, parent_b.fertility, 0.3, 1.0, rng, mutation_scale, rate_large, rate_small),
            lifespan_factor: inherit_trait(parent_a.lifespan_factor, parent_b.lifespan_factor, 0.5, 2.0, rng, mutation_scale, rate_large, rate_small),
            maturity_age: inherit_trait(parent_a.maturity_age, parent_b.maturity_age, 0.3, 0.7, rng, mutation_scale, rate_large, rate_small),
            disease_resistance: inherit_trait(parent_a.disease_resistance, parent_b.disease_resistance, 0.0, 1.0, rng, mutation_scale, rate_large, rate_small),
        };

        // Inbreeding penalties
        if inbred {
            child.lifespan_factor *= 0.85;
            child.fertility *= 0.90;
        }

        child
    }
}

fn inherit_trait(a: f32, b: f32, min: f32, max: f32, rng: &mut impl Rng, mutation_scale: f32, rate_large: f32, rate_small: f32) -> f32 {
    // Inheritance: dominant (60%) or blended (40%)
    let base = if rng.gen_bool(0.6) {
        if rng.gen_bool(0.5) { a } else { b }
    } else {
        let w: f32 = rng.gen_range(0.3..0.7);
        a * w + b * (1.0 - w)
    };

    // Mutation
    let range = max - min;
    if range <= 0.0 {
        return base.clamp(min, max);
    }
    let roll: f32 = rng.gen();
    let mutated = if roll < rate_large * mutation_scale {
        // Large mutation
        let sigma = (0.2 * range) as f64;
        let normal = Normal::new(0.0_f64, sigma).unwrap_or(Normal::new(0.0, 1.0).unwrap());
        base + normal.sample(rng) as f32
    } else if roll < (rate_large + rate_small) * mutation_scale {
        // Small mutation
        let sigma = (0.05 * range) as f64;
        let normal = Normal::new(0.0_f64, sigma).unwrap_or(Normal::new(0.0, 1.0).unwrap());
        base + normal.sample(rng) as f32
    } else {
        base
    };

    mutated.clamp(min, max)
}

/// Circular hue inheritance (wraps around 0/360 boundary correctly)
fn inherit_hue(a: f32, b: f32, rng: &mut impl Rng, mutation_scale: f32, rate_large: f32, rate_small: f32) -> f32 {
    // Use shortest arc on color wheel
    let mut diff = b - a;
    if diff > 180.0 { diff -= 360.0; }
    if diff < -180.0 { diff += 360.0; }

    let base = if rng.gen_bool(0.6) {
        if rng.gen_bool(0.5) { a } else { b }
    } else {
        let w: f32 = rng.gen_range(0.3..0.7);
        a + diff * w
    };

    // Mutation
    let roll: f32 = rng.gen();
    let mutated = if roll < rate_large * mutation_scale {
        base + rng.gen_range(-36.0..36.0) // large hue mutation
    } else if roll < (rate_large + rate_small) * mutation_scale {
        base + rng.gen_range(-9.0..9.0) // small hue mutation
    } else {
        base
    };

    mutated.rem_euclid(360.0)
}

fn inherit_pattern(a: &PatternGene, b: &PatternGene, rng: &mut impl Rng) -> PatternGene {
    let roll: f32 = rng.gen();
    if roll < 0.10 {
        // Completely new pattern type
        PatternGene::random(rng)
    } else if roll < 0.30 {
        // Inherit one parent's pattern but mutate sub-params
        let base = if rng.gen_bool(0.5) { a } else { b };
        match base {
            PatternGene::Solid => PatternGene::Solid,
            PatternGene::Striped { angle } => PatternGene::Striped {
                angle: (angle + rng.gen_range(-20.0..20.0)).clamp(0.0, 180.0),
            },
            PatternGene::Spotted { density } => PatternGene::Spotted {
                density: (density + rng.gen_range(-0.15..0.15)).clamp(0.2, 1.0),
            },
            PatternGene::Gradient { direction } => PatternGene::Gradient {
                direction: (direction + rng.gen_range(-30.0..30.0)).rem_euclid(360.0),
            },
            PatternGene::Bicolor { split } => PatternGene::Bicolor {
                split: (split + rng.gen_range(-0.1..0.1)).clamp(0.3, 0.7),
            },
        }
    } else {
        // Inherit one parent's pattern directly
        if rng.gen_bool(0.5) { a.clone() } else { b.clone() }
    }
}

/// Genome distance for species affinity and reproduction compatibility
pub fn genome_distance(a: &FishGenome, b: &FishGenome) -> f32 {
    let mut d = 0.0_f32;

    // Appearance traits (weighted higher)
    d += hue_distance(a.base_hue, b.base_hue) / 180.0 * 3.0;
    d += (a.saturation - b.saturation).abs() * 1.5;
    d += (a.body_length - b.body_length).abs() / 1.4 * 2.0;
    d += (a.body_width - b.body_width).abs() * 1.0;
    d += pattern_distance(&a.pattern, &b.pattern) * 2.5;
    d += (a.pattern_intensity - b.pattern_intensity).abs() * 1.0;

    // Behavior traits (weighted lower)
    d += (a.speed - b.speed).abs() / 1.5 * 0.5;
    d += (a.aggression - b.aggression).abs() * 0.5;
    d += (a.school_affinity - b.school_affinity).abs() * 0.5;
    d += (a.disease_resistance - b.disease_resistance).abs() * 0.3;

    d
}

fn hue_distance(a: f32, b: f32) -> f32 {
    let diff = (a - b).abs();
    diff.min(360.0 - diff)
}

fn pattern_distance(a: &PatternGene, b: &PatternGene) -> f32 {
    if a.type_index() != b.type_index() {
        return 1.0;
    }
    match (a, b) {
        (PatternGene::Solid, PatternGene::Solid) => 0.0,
        (PatternGene::Striped { angle: a }, PatternGene::Striped { angle: b }) => {
            (a - b).abs() / 180.0
        }
        (PatternGene::Spotted { density: a }, PatternGene::Spotted { density: b }) => {
            (a - b).abs() / 0.8
        }
        (PatternGene::Gradient { direction: a }, PatternGene::Gradient { direction: b }) => {
            hue_distance(*a, *b) / 180.0
        }
        (PatternGene::Bicolor { split: a }, PatternGene::Bicolor { split: b }) => {
            (a - b).abs() / 0.4
        }
        _ => 1.0,
    }
}
