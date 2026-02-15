# DeepTank Genetic System

## Overview

DeepTank simulates realistic evolutionary genetics through a diploid 23-trait genome system with Mendelian inheritance, blended traits, mutations, and inbreeding penalties. This document explains the system in detail.

## The 23 Traits

All traits are stored as floating-point values in the range [0.0, 1.0] (normalized). Each fish is **diploid** — it has two copies (alleles) of each trait, one from each parent.

### Morphology (6 traits)

| Trait | Range | Effect |
|-------|-------|--------|
| **body_length** | 0.0-1.0 | Fish size. Affects predation success (larger fish hunt smaller fish). Also affects metabolism. |
| **body_width** | 0.0-1.0 | Girth. Affects turning speed (wider fish turn slower). |
| **tail_size** | 0.0-1.0 | Tail fin size. Purely visual; used in sprite generation. |
| **dorsal_fin_size** | 0.0-1.0 | Dorsal fin size. Purely visual. |
| **pectoral_fin_size** | 0.0-1.0 | Side fin size. Purely visual. |
| **eye_size** | 0.0-1.0 | Eye size. Purely visual; may affect perception radius in future. |

### Color (5 traits)

| Trait | Range | Effect |
|-------|-------|--------|
| **base_hue** | 0-360 degrees (circular) | Hue on HSL color wheel. Circular interpolation (0° ≈ 360°). |
| **saturation** | 0.0-1.0 | HSL saturation (color intensity). 0 = grayscale, 1 = pure color. |
| **lightness** | 0.0-1.0 | HSL lightness. 0 = black, 0.5 = full color, 1 = white. |
| **pattern** | {0-4} (enum) | Pattern type: Solid, Striped, Spotted, Gradient, Bicolor. |
| **pattern_intensity** | 0.0-1.0 | How prominent is the pattern. 0 = barely visible, 1 = strong contrast. |

### Behavior (6 traits)

| Trait | Range | Effect |
|-------|-------|--------|
| **speed** | 0.0-1.0 | Base movement speed. Multiplier on physics velocity. Higher = faster = more metabolism. |
| **aggression** | 0.0-1.0 | Tendency to attack/hunt. 0 = passive, 1 = predatory. Affects hunting behavior. |
| **school_affinity** | 0.0-1.0 | Desire to stay in school (group). 0 = solitary, 1 = strongly schooling. |
| **curiosity** | 0.0-1.0 | Interest in exploring new areas. Affects wandering behavior. |
| **boldness** | 0.0-1.0 | Risk-taking tendency. 0 = cautious, 1 = reckless. |
| **metabolism** | 0.0-1.0 | Energy usage rate. Higher metabolism = hunger increases faster, but growth faster. |

### Lifecycle (6 traits)

| Trait | Range | Effect |
|-------|-------|--------|
| **fertility** | 0.0-1.0 | Reproduction probability when paired. Higher = more eager to breed. |
| **lifespan_factor** | 0.5-2.0 | Multiplier on max lifespan. 0.5 = short-lived, 2.0 = long-lived. |
| **maturity_age** | 100-1000 ticks | Age before fish can reproduce. Higher = later maturity = longer juvenile period. |
| **disease_resistance** | 0.0-1.0 | Immune strength. Higher = less likely to get infected, disease deals less damage. |
| **reserved_trait_1** | 0.0-1.0 | Reserved for future use. Currently unused but inherited. |
| **reserved_trait_2** | 0.0-1.0 | Reserved for future use. Currently unused but inherited. |

## Inheritance Model

### Diploid System

Each fish carries two alleles (copies) for each trait:

```
Parent A: body_length = [0.8, 0.9]  (heterozygous)
Parent B: body_length = [0.7, 0.7]  (homozygous)

Offspring receives:
- One allele from Parent A (50% chance 0.8, 50% chance 0.9)
- One allele from Parent B (100% chance 0.7)

Result: [0.8, 0.7] or [0.9, 0.7] (random Mendelian segregation)
```

### Dominance Rules

Traits use three different inheritance strategies:

#### 1. Dominant Traits (Use Maximum Allele)

These traits express the dominant phenotype if either allele is present:

- body_length, body_width, tail_size (morphology — larger is dominant)
- speed, aggression, fertility (behavior — more active is dominant)

**Example:**
```
Alleles: [0.3, 0.9] → Phenotype: 0.9 (the larger one wins)
```

#### 2. Blended Traits (Average Both Alleles)

These traits blend together (codominance):

- base_hue (circular average, accounting for 0°/360° wraparound)
- saturation, lightness, pattern_intensity
- school_affinity, curiosity, boldness
- metabolism, disease_resistance

**Example:**
```
Alleles: [0.4, 0.8] → Phenotype: 0.6 (average)
```

#### 3. Mendelian Patterns

Pattern type (Solid, Striped, etc.) follows a probabilistic Mendelian model:

```
Parent A pattern: Striped
Parent B pattern: Solid

Child:
  - 70% chance of Striped (dominant)
  - 30% chance of Solid (recessive)
```

This creates diversity in morphology beyond just blending.

## Mutations

When offspring are born, each allele mutates independently with probabilities:

### Two-Tier Mutation System

```
For each allele of each trait:

  88% chance: No mutation (exact copy)
   2% chance: Large mutation (±0.3 change)
  10% chance: Small mutation (±0.1 change)
```

**Large Mutation Example:**
```
Parent allele: 0.8
Large mutation: -0.3
Result: 0.5 (clamped to [0.0, 1.0])
```

**Small Mutation Example:**
```
Parent allele: 0.8
Small mutation: +0.1
Result: 0.9
```

**No Mutation:**
```
Parent allele: 0.8
Result: 0.8 (exact copy)
```

### Mutation Rates in Code

```rust
// genome.rs
const LARGE_MUTATION_RATE: f32 = 0.02;   // 2%
const LARGE_MUTATION_SIZE: f32 = 0.3;    // ±0.3

const SMALL_MUTATION_RATE: f32 = 0.10;   // 10%
const SMALL_MUTATION_SIZE: f32 = 0.1;    // ±0.1
```

These rates were tuned to provide diversity without creating completely different fish too quickly. Increase LARGE_MUTATION_RATE for faster evolution, decrease for more stability.

## Genetic Diversity & Inbreeding

The system tracks genetic diversity in the population:

```
diversity_score = avg(euclidean_distance between all fish pairs)
```

If diversity drops below threshold (0.3):

```
fertility *= 0.5        // Half-speed breeding
lifespan *= 0.8         // 20% shorter lives
```

This is an **inbreeding penalty** that incentivizes breeding diverse traits and prevents populations from collapsing genetically.

**Example:**
```
Tank 1: High diversity (0.5)
  - Normal fertility, normal lifespan
  - Population grows steadily

Tank 2: Low diversity (0.2) [all fish look similar]
  - 50% reduced fertility
  - 80% of normal lifespan
  - Population struggles, pressure to diversify
```

## Speciation

Fish are automatically grouped into **species** based on genome similarity using single-linkage clustering.

### Species Clustering Algorithm

1. **Compute genome distance** for all fish pairs (Euclidean distance in trait space)
2. **Single-linkage clustering:** Two fish in same species if distance < 2.5
3. **Min members:** Species must have ≥ 3 individuals to be recognized
4. **Auto-naming:** Generate species name from trait phenotypes

**Example:**
```
Fish A: [red, striped, fast]
Fish B: [red-ish, striped, fast]
Fish C: [blue, solid, slow]

Distance(A, B) = 0.2 → same species
Distance(A, C) = 1.8 → different species
Distance(B, C) = 1.9 → different species

Species 1: A, B → "Red-Striped Speedster"
Species 2: C → "Blue-Solid Slowpoke"
```

### Species Naming

If **Ollama is configured:**
- Send phenotype data to LLM
- Get creative names: "Crimson Dart", "Twilight Cruiser", etc.

If **Ollama unavailable (fallback):**
- Deterministic naming from traits: `"{hue}-{pattern} {behavior}"`
- Example: "Red-Striped Hunter"

## Evolution in Action

### Selection Pressures

The simulation applies several selective pressures:

| Pressure | Effect | Favors |
|----------|--------|--------|
| **Predation** | Larger fish hunt smaller fish | Size, aggression, speed |
| **Starvation** | Low-energy fish die | Metabolism efficiency, foraging |
| **Water Quality** | Poor water harms growth | Diverse population (reduces waste) |
| **Disease** | Reduces health | Disease resistance, isolation |
| **Reproduction** | High fertility = more offspring | Fertility, maturity, lifespan |
| **Environmental Events** | Cold Snap, Heatwave, etc. | Adapted metabolism, resilience |

### Example Evolution Scenario

**Setup:**
- Start with random population (traits ~0.5 across the board, all same species)
- Add 5 predatory fish (body_length=0.9, aggression=0.8)

**Generation 0-10:**
- Predators hunt small prey
- Prey fish with high speed/small size survive longer
- Slow prey are removed from population

**Generation 10-30:**
- Prey population splits into two strategies:
  - **Hiders:** Small, fast, high school_affinity (safety in numbers)
  - **Lurkers:** Hide near decorations, medium speed, low aggression
- Predators face declining success → starvation → death (no replacement)

**Generation 30-50:**
- Two new predatory species evolve FROM prey population
  - One branch: Larger, hunt small hiders
  - One branch: Smaller, hunt lurkers
- Original predators extinct
- Population stabilizes with 3 species

**Result:** Original random population → 3 distinct adaptive radiations driven purely by selection pressure.

## Genetic Algorithms Theory

DeepTank implements principles from evolutionary biology and genetic algorithms:

### Core Concepts

| Term | Definition |
|------|-----------|
| **Allele** | One copy of a trait (fish have 2 per trait) |
| **Genotype** | Entire set of alleles (internal blueprint) |
| **Phenotype** | Expressed traits (what you see: color, size, speed) |
| **Fitness** | Probability of survival and reproduction |
| **Heritability** | % of trait variation due to genes vs environment |
| **Genetic Drift** | Random change in trait frequency over time |
| **Founder Effect** | Reduced diversity when starting from small population |
| **Adaptive Radiation** | One species diverges into many (under selection pressure) |
| **Clade** | Group of related species sharing common ancestor |

### Simulation vs Real Biology

| Aspect | DeepTank | Real Biology |
|--------|----------|-------------|
| **Mutation Rate** | 12% per allele | ~10^-8 per nucleotide |
| **Time Scale** | Generations visible in hours | Millions of years |
| **Genome Size** | 23 traits | 20,000+ genes |
| **Phenotypic Complexity** | Simplified (23 traits = fitness proxy) | Extremely complex |
| **Heritability** | 100% (all traits inherited) | 0-100% per trait |

DeepTank is **scientifically inspired but simplified** for speed and visibility.

## Tuning Genetic Evolution

### Configuration Parameters (in config.rs)

```rust
// Mutation rates
pub mutation_rate_large: f32,        // Default: 0.02 (2%)
pub mutation_rate_small: f32,        // Default: 0.10 (10%)
pub mutation_size_large: f32,        // Default: 0.30 (±0.3)
pub mutation_size_small: f32,        // Default: 0.10 (±0.1)

// Diversity penalties
pub inbreeding_diversity_threshold: f32,  // Default: 0.30
pub inbreeding_fertility_penalty: f32,    // Default: 0.5 (50% reduction)
pub inbreeding_lifespan_penalty: f32,     // Default: 0.8 (20% reduction)

// Speciation
pub species_clustering_distance: f32,  // Default: 2.5
pub species_min_members: u32,          // Default: 3
```

### How to Tune Evolution

**Faster Evolution:**
```
mutation_rate_large = 0.05       // Increase from 2% to 5%
mutation_size_large = 0.5        // Increase from ±0.3 to ±0.5
```

**Slower Evolution (more stable):**
```
mutation_rate_large = 0.01       // Decrease to 1%
mutation_size_large = 0.1        // Decrease to ±0.1
```

**Higher Diversity:**
```
inbreeding_diversity_threshold = 0.5  // Increase from 0.3
inbreeding_fertility_penalty = 0.7    // Reduce penalty
```

**More Speciation (split into more species):**
```
species_clustering_distance = 1.5  // Decrease (stricter threshold)
```

## Genetic Data Export

The simulator can export genetic data:

```bash
# CSV export of fish genomes
deeptank export --format csv --output genomes.csv

# JSON export of population snapshots
deeptank export --format json --output population.json
```

**CSV Format:**
```
fish_id,generation,body_length,body_width,...,base_hue,saturation,lightness,pattern
1,0,0.52,0.48,...,0.21,0.75,0.65,Striped
2,0,0.55,0.49,...,0.23,0.73,0.64,Solid
```

**Uses:**
- Analyze real evolution patterns
- Export to research tools
- Recreate tanks from saved populations

## Advanced Topics

### Circular Hue Math

Hue is circular (0° ≈ 360°), so averaging requires special math:

```
Allele A: 10° (red)
Allele B: 350° (red-ish)

Simple average: (10 + 350) / 2 = 180° (green) ✗ WRONG
Circular average: 0° (red) ✓ CORRECT
```

**Implementation:**
```rust
// Convert to unit vectors, average, convert back
let a_vec = (hue_a * PI / 180.0).sin_cos();
let b_vec = (hue_b * PI / 180.0).sin_cos();
let avg = ((a_vec.0 + b_vec.0) / 2, (a_vec.1 + b_vec.1) / 2);
let result = avg.1.atan2(avg.0) * 180.0 / PI;
```

### Trait Normalization

Traits are stored as [0, 1], but some have different actual ranges:

| Trait | Internal | Actual | Conversion |
|-------|----------|--------|------------|
| body_length | 0.0-1.0 | 5-50 pixels | pixels = 5 + (trait * 45) |
| lifespan_factor | 0.0-1.0 | 0.5-2.0x | multiplier = 0.5 + (trait * 1.5) |
| base_hue | 0.0-1.0 | 0-360° | degrees = trait * 360 |

This normalization is handled in sprite generation and behavior calculations.

---

**The genetic system is the heart of DeepTank. It creates natural diversity and evolution without any designer intervention.**
