export interface FishState {
  id: number;
  x: number;
  y: number;
  z: number;
  vx: number;
  vy: number;
  heading: number;
  behavior: string;
  hunger: number;
  health: number;
  age_fraction: number;
  genome_id: number;
  energy: number;
  is_infected: boolean;
  is_juvenile: boolean;
  territory_cx?: number;
  territory_cy?: number;
  territory_r?: number;
  custom_name?: string;
  is_favorite?: boolean;
}

export interface EggState {
  id: number;
  x: number;
  y: number;
  genome_id: number;
  progress: number;
}

export interface FoodState {
  x: number;
  y: number;
  food_type: string;
}

export interface BubbleState {
  x: number;
  y: number;
  radius: number;
}

export interface SimEvent {
  Birth?: { fish_id: number; genome_id: number; parent_a: number; parent_b: number };
  Death?: { fish_id: number; genome_id: number; cause: string };
  FeedingDrop?: { x: number; y: number };
  Predation?: { predator_id: number; prey_id: number };
  NewSpecies?: { species_id: number };
  Extinction?: { species_id: number };
}

export interface DecorationState {
  id: number;
  decoration_type: string;
  x: number;
  y: number;
  scale: number;
  flip_x: boolean;
}

export interface FrameUpdate {
  tick: number;
  fish: FishState[];
  food: FoodState[];
  bubbles: BubbleState[];
  eggs: EggState[];
  decorations: DecorationState[];
  events: SimEvent[];
  water_quality: number;
  population: number;
  max_generation: number;
  species_count: number;
  time_of_day: number;
  active_event: string | null;
  genetic_diversity: number;
}

export interface PatternGene {
  Solid?: null;
  Striped?: { angle: number };
  Spotted?: { density: number };
  Gradient?: { direction: number };
  Bicolor?: { split: number };
}

export interface FishGenome {
  id: number;
  generation: number;
  parent_a: number | null;
  parent_b: number | null;
  sex: "Male" | "Female";
  base_hue: number;
  saturation: number;
  lightness: number;
  body_length: number;
  body_width: number;
  tail_size: number;
  dorsal_fin_size: number;
  pectoral_fin_size: number;
  pattern: PatternGene;
  pattern_intensity: number;
  pattern_color_offset: number;
  eye_size: number;
  speed: number;
  aggression: number;
  school_affinity: number;
  curiosity: number;
  boldness: number;
  metabolism: number;
  fertility: number;
  lifespan_factor: number;
  maturity_age: number;
  disease_resistance: number;
}

export interface Species {
  id: number;
  name: string | null;
  description: string | null;
  discovered_at_tick: number;
  extinct_at_tick: number | null;
  centroid_hue: number;
  centroid_speed: number;
  centroid_size: number;
  centroid_pattern: string;
  member_count: number;
}

export interface SpeciesHistoryEntry {
  id: number;
  name: string | null;
  description: string | null;
  discovered_at_tick: number;
  extinct_at_tick: number | null;
  centroid_hue: number;
  centroid_speed: number;
  centroid_size: number;
  centroid_pattern: string;
  member_count: number;
  representative_genome_id: number | null;
}

export interface FishDetail {
  id: number;
  genome_id: number;
  x: number;
  y: number;
  z: number;
  heading: number;
  age: number;
  hunger: number;
  health: number;
  energy: number;
  behavior: string;
  meals_eaten: number;
  is_alive: boolean;
  is_infected: boolean;
  custom_name: string | null;
  is_favorite: boolean;
  genome: FishGenome;
  species_name: string | null;
}

/** Must match BASE_LIFESPAN in ecosystem.rs */
export const BASE_LIFESPAN = 20_000;

export interface Toast {
  id: number;
  message: string;
  type: "info" | "warning" | "success" | "danger";
  timestamp: number;
}
