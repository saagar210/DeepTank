import { describe, it, expect } from "vitest";
import type {
  FishState,
  EggState,
  FoodState,
  BubbleState,
  SimEvent,
  DecorationState,
  FrameUpdate,
  PatternGene,
  FishGenome,
  Species,
  FishDetail,
  Toast,
} from "./types";
import { BASE_LIFESPAN } from "./types";

describe("BASE_LIFESPAN constant", () => {
  it("matches Rust ecosystem::BASE_LIFESPAN (20000)", () => {
    expect(BASE_LIFESPAN).toBe(20_000);
  });
});

describe("type guards and structural validation", () => {
  it("FishState has required fields", () => {
    const fish: FishState = {
      id: 1,
      x: 100,
      y: 200,
      z: 0.5,
      vx: 1.0,
      vy: -0.5,
      heading: 0.3,
      behavior: "swimming",
      hunger: 0.2,
      health: 1.0,
      age_fraction: 0.1,
      genome_id: 42,
      energy: 0.9,
      is_infected: false,
      is_juvenile: false,
    };
    expect(fish.id).toBe(1);
    expect(fish.territory_cx).toBeUndefined(); // optional
  });

  it("FishState optional fields work", () => {
    const fish: FishState = {
      id: 2,
      x: 0,
      y: 0,
      z: 0,
      vx: 0,
      vy: 0,
      heading: 0,
      behavior: "resting",
      hunger: 0,
      health: 1,
      age_fraction: 0,
      genome_id: 1,
      energy: 1,
      is_infected: false,
      is_juvenile: true,
      territory_cx: 400,
      territory_cy: 300,
      territory_r: 60,
      custom_name: "Nemo",
      is_favorite: true,
    };
    expect(fish.custom_name).toBe("Nemo");
    expect(fish.territory_r).toBe(60);
  });

  it("EggState has required fields", () => {
    const egg: EggState = { id: 1, x: 100, y: 700, genome_id: 5, progress: 0.5 };
    expect(egg.progress).toBe(0.5);
  });

  it("FoodState has food_type field", () => {
    const food: FoodState = { x: 100, y: 50, food_type: "pellet" };
    expect(food.food_type).toBe("pellet");
  });

  it("BubbleState has radius", () => {
    const bubble: BubbleState = { x: 200, y: 100, radius: 3.0 };
    expect(bubble.radius).toBe(3.0);
  });

  it("SimEvent variants are mutually exclusive", () => {
    const birth: SimEvent = {
      Birth: { fish_id: 1, genome_id: 2, parent_a: 3, parent_b: 4 },
    };
    expect(birth.Birth).toBeDefined();
    expect(birth.Death).toBeUndefined();

    const death: SimEvent = {
      Death: { fish_id: 1, genome_id: 2, cause: "old_age" },
    };
    expect(death.Death?.cause).toBe("old_age");
  });

  it("DecorationState has all fields", () => {
    const deco: DecorationState = {
      id: 1,
      decoration_type: "tall_plant",
      x: 200,
      y: 500,
      scale: 1.5,
      flip_x: true,
    };
    expect(deco.flip_x).toBe(true);
  });

  it("FrameUpdate has all top-level fields", () => {
    const frame: FrameUpdate = {
      tick: 1000,
      fish: [],
      food: [],
      bubbles: [],
      eggs: [],
      decorations: [],
      events: [],
      water_quality: 0.95,
      population: 25,
      max_generation: 12,
      species_count: 3,
      time_of_day: 14.5,
      active_event: null,
      genetic_diversity: 0.82,
    };
    expect(frame.tick).toBe(1000);
    expect(frame.active_event).toBeNull();
    expect(frame.genetic_diversity).toBe(0.82);
  });

  it("PatternGene variants", () => {
    const solid: PatternGene = { Solid: null };
    const striped: PatternGene = { Striped: { angle: 45 } };
    const spotted: PatternGene = { Spotted: { density: 0.7 } };
    const gradient: PatternGene = { Gradient: { direction: 180 } };
    const bicolor: PatternGene = { Bicolor: { split: 0.5 } };

    expect(solid.Solid).toBeNull();
    expect(striped.Striped?.angle).toBe(45);
    expect(spotted.Spotted?.density).toBe(0.7);
    expect(gradient.Gradient?.direction).toBe(180);
    expect(bicolor.Bicolor?.split).toBe(0.5);
  });

  it("FishGenome has all 23+ trait fields", () => {
    const genome: FishGenome = {
      id: 1,
      generation: 5,
      parent_a: 2,
      parent_b: 3,
      sex: "Male",
      base_hue: 200,
      saturation: 0.8,
      lightness: 0.5,
      body_length: 1.2,
      body_width: 0.9,
      tail_size: 1.1,
      dorsal_fin_size: 0.7,
      pectoral_fin_size: 0.8,
      pattern: { Striped: { angle: 60 } },
      pattern_intensity: 0.6,
      pattern_color_offset: 90,
      eye_size: 1.0,
      speed: 1.5,
      aggression: 0.3,
      school_affinity: 0.7,
      curiosity: 0.5,
      boldness: 0.4,
      metabolism: 1.0,
      fertility: 0.8,
      lifespan_factor: 1.2,
      maturity_age: 0.4,
      disease_resistance: 0.6,
    };

    expect(genome.sex).toBe("Male");
    expect(genome.base_hue).toBe(200);
    expect(genome.disease_resistance).toBe(0.6);
    // Count all fields
    expect(Object.keys(genome).length).toBeGreaterThanOrEqual(27);
  });

  it("Species has centroid fields", () => {
    const species: Species = {
      id: 1,
      name: "Blue Darters",
      description: "Swift blue schooling fish",
      discovered_at_tick: 500,
      extinct_at_tick: null,
      centroid_hue: 210,
      centroid_speed: 1.6,
      centroid_size: 1.1,
      centroid_pattern: "Striped { angle: 45.0 }",
      member_count: 8,
    };
    expect(species.extinct_at_tick).toBeNull();
    expect(species.member_count).toBe(8);
  });

  it("FishDetail extends with genome and species", () => {
    const detail: FishDetail = {
      id: 1,
      genome_id: 42,
      x: 100,
      y: 200,
      z: 0.5,
      heading: 1.2,
      age: 5000,
      hunger: 0.3,
      health: 0.9,
      energy: 0.8,
      behavior: "swimming",
      meals_eaten: 25,
      is_alive: true,
      is_infected: false,
      custom_name: "Bubbles",
      is_favorite: true,
      genome: {
        id: 42,
        generation: 3,
        parent_a: 10,
        parent_b: 11,
        sex: "Female",
        base_hue: 120,
        saturation: 0.7,
        lightness: 0.5,
        body_length: 1.0,
        body_width: 0.8,
        tail_size: 1.0,
        dorsal_fin_size: 0.8,
        pectoral_fin_size: 0.7,
        pattern: { Spotted: { density: 0.5 } },
        pattern_intensity: 0.4,
        pattern_color_offset: 60,
        eye_size: 1.0,
        speed: 1.2,
        aggression: 0.2,
        school_affinity: 0.9,
        curiosity: 0.6,
        boldness: 0.3,
        metabolism: 0.8,
        fertility: 0.7,
        lifespan_factor: 1.1,
        maturity_age: 0.35,
        disease_resistance: 0.5,
      },
      species_name: "Green Schoolers",
    };
    expect(detail.is_alive).toBe(true);
    expect(detail.genome.sex).toBe("Female");
    expect(detail.species_name).toBe("Green Schoolers");
  });

  it("Toast has valid type literals", () => {
    const toasts: Toast[] = [
      { id: 1, message: "Fish born!", type: "success", timestamp: Date.now() },
      { id: 2, message: "Species extinct", type: "danger", timestamp: Date.now() },
      { id: 3, message: "Low water quality", type: "warning", timestamp: Date.now() },
      { id: 4, message: "Simulation paused", type: "info", timestamp: Date.now() },
    ];
    expect(toasts).toHaveLength(4);
    const types = toasts.map((t) => t.type);
    expect(types).toEqual(["success", "danger", "warning", "info"]);
  });
});
