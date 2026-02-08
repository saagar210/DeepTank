use crate::simulation::config::SimulationConfig;
use crate::simulation::fish::{BehaviorState, Fish};
use crate::simulation::genome::{genome_distance, FishGenome};
use noise::{NoiseFn, Perlin};

pub struct SpatialGrid {
    cell_size: f32,
    cols: usize,
    rows: usize,
    cells: Vec<Vec<usize>>, // cell index -> list of fish indices
}

impl SpatialGrid {
    pub fn new(width: f32, height: f32, cell_size: f32) -> Self {
        let cols = (width / cell_size).ceil() as usize + 1;
        let rows = (height / cell_size).ceil() as usize + 1;
        Self {
            cell_size,
            cols,
            rows,
            cells: vec![Vec::new(); cols * rows],
        }
    }

    pub fn rebuild(&mut self, fish: &[Fish]) {
        for cell in &mut self.cells {
            cell.clear();
        }
        for (i, f) in fish.iter().enumerate() {
            let col = (f.x.max(0.0) / self.cell_size) as usize;
            let row = (f.y.max(0.0) / self.cell_size) as usize;
            let col = col.min(self.cols - 1);
            let row = row.min(self.rows - 1);
            let idx = row * self.cols + col;
            self.cells[idx].push(i);
        }
    }

    pub fn neighbors(&self, x: f32, y: f32, radius: f32) -> Vec<usize> {
        let mut result = Vec::new();
        let min_col = ((x - radius) / self.cell_size).floor().max(0.0) as usize;
        let max_col = ((x + radius) / self.cell_size).ceil() as usize;
        let min_row = ((y - radius) / self.cell_size).floor().max(0.0) as usize;
        let max_row = ((y + radius) / self.cell_size).ceil() as usize;

        for row in min_row..=max_row.min(self.rows - 1) {
            for col in min_col..=max_col.min(self.cols - 1) {
                let idx = row * self.cols + col;
                result.extend_from_slice(&self.cells[idx]);
            }
        }
        result
    }
}

pub struct BoidsEngine {
    pub perlin: Perlin,
    pub grid: SpatialGrid,
}

impl BoidsEngine {
    pub fn new(config: &SimulationConfig) -> Self {
        Self {
            perlin: Perlin::new(42),
            grid: SpatialGrid::new(config.tank_width, config.tank_height, config.cohesion_radius),
        }
    }

    pub fn update(
        &mut self,
        fish: &mut [Fish],
        genomes: &std::collections::HashMap<u32, FishGenome>,
        config: &SimulationConfig,
        tick: u64,
        food_positions: &[(f32, f32)],
        obstacles: &[(f32, f32, f32)],
    ) {
        self.grid.rebuild(fish);

        // Compute forces for all fish, then apply (avoids borrow issues)
        let forces: Vec<(f32, f32)> = (0..fish.len())
            .map(|i| {
                self.compute_forces(i, fish, genomes, config, tick, food_positions, obstacles)
            })
            .collect();

        for (i, (fx, fy)) in forces.into_iter().enumerate() {
            let f = &mut fish[i];
            let genome = match genomes.get(&f.genome_id) {
                Some(g) => g,
                None => continue,
            };

            // Acceleration smoothing
            let smoothed_fx = f.prev_force_x * 0.3 + fx * 0.7;
            let smoothed_fy = f.prev_force_y * 0.3 + fy * 0.7;
            f.prev_force_x = smoothed_fx;
            f.prev_force_y = smoothed_fy;

            // Apply force (clamped to max_force)
            let force_mag = (smoothed_fx * smoothed_fx + smoothed_fy * smoothed_fy).sqrt();
            let (applied_fx, applied_fy) = if force_mag > config.max_force {
                let scale = config.max_force / force_mag;
                (smoothed_fx * scale, smoothed_fy * scale)
            } else {
                (smoothed_fx, smoothed_fy)
            };

            f.vx += applied_fx;
            f.vy += applied_fy;

            // Clamp to max speed
            let max_speed = config.base_max_speed * genome.speed;
            let speed = (f.vx * f.vx + f.vy * f.vy).sqrt();
            if speed > max_speed {
                let scale = max_speed / speed;
                f.vx *= scale;
                f.vy *= scale;
            }

            // Apply drag
            f.vx *= config.drag;
            f.vy *= config.drag;

            // Update position and clamp to tank bounds
            f.x = (f.x + f.vx).clamp(0.0, config.tank_width);
            f.y = (f.y + f.vy).clamp(0.0, config.tank_height);

            // Update heading
            let spd = (f.vx * f.vx + f.vy * f.vy).sqrt();
            if spd > 0.01 {
                f.heading = f.vy.atan2(f.vx);
            }

            // Depth drift
            let depth_noise = self.perlin.get([f.x as f64 * 0.01, tick as f64 * 0.001]) as f32;
            f.z = (f.z + depth_noise * 0.002).clamp(0.0, 1.0);
        }
    }

    fn compute_forces(
        &self,
        fish_idx: usize,
        fish: &[Fish],
        genomes: &std::collections::HashMap<u32, FishGenome>,
        config: &SimulationConfig,
        tick: u64,
        food_positions: &[(f32, f32)],
        obstacles: &[(f32, f32, f32)],
    ) -> (f32, f32) {
        let me = &fish[fish_idx];
        let my_genome = match genomes.get(&me.genome_id) {
            Some(g) => g,
            None => return (0.0, 0.0),
        };

        let mut fx = 0.0_f32;
        let mut fy = 0.0_f32;

        // Get behavioral modifiers
        let schooling_mult = me.behavior_schooling_multiplier();
        let speed_mult = me.behavior_speed_multiplier();

        // Get neighbors within cohesion radius (the largest)
        let candidates = self.grid.neighbors(me.x, me.y, config.cohesion_radius);

        let mut sep_x = 0.0_f32;
        let mut sep_y = 0.0_f32;
        let mut align_x = 0.0_f32;
        let mut align_y = 0.0_f32;
        let mut align_count = 0_u32;
        let mut coh_x = 0.0_f32;
        let mut coh_y = 0.0_f32;
        let mut coh_weight = 0.0_f32;

        for &j in &candidates {
            if j == fish_idx {
                continue;
            }
            let other = &fish[j];
            let dx = me.x - other.x;
            let dy = me.y - other.y;
            let dist_sq = dx * dx + dy * dy;
            let dist = dist_sq.sqrt();

            if dist < 0.001 {
                continue;
            }

            // Species affinity
            let affinity = if let Some(other_genome) = genomes.get(&other.genome_id) {
                let gd = genome_distance(my_genome, other_genome);
                (1.0 - gd / 10.0).clamp(0.0, 1.0)
            } else {
                0.5
            };

            // Separation
            if dist < config.separation_radius {
                let repulsion = 1.0 / (dist_sq + 0.001);
                sep_x += dx * repulsion;
                sep_y += dy * repulsion;
            }

            // Alignment
            if dist < config.alignment_radius {
                let spd = (other.vx * other.vx + other.vy * other.vy).sqrt();
                if spd > 0.01 {
                    align_x += (other.vx / spd) * affinity;
                    align_y += (other.vy / spd) * affinity;
                    align_count += 1;
                }
            }

            // Cohesion
            if dist < config.cohesion_radius {
                coh_x += other.x * affinity;
                coh_y += other.y * affinity;
                coh_weight += affinity;
            }
        }

        // Apply separation
        let personal_space = 1.0 + my_genome.school_affinity.max(0.0) * 0.5;
        fx += sep_x * config.separation_weight * personal_space;
        fy += sep_y * config.separation_weight * personal_space;

        // Apply alignment (scaled by schooling behavior)
        if align_count > 0 {
            let avg_x = align_x / align_count as f32;
            let avg_y = align_y / align_count as f32;
            let my_spd = (me.vx * me.vx + me.vy * me.vy).sqrt().max(0.01);
            let diff_x = avg_x - me.vx / my_spd;
            let diff_y = avg_y - me.vy / my_spd;
            fx += diff_x * config.alignment_weight * my_genome.school_affinity * schooling_mult;
            fy += diff_y * config.alignment_weight * my_genome.school_affinity * schooling_mult;
        }

        // Apply cohesion (scaled by schooling behavior)
        if coh_weight > 0.001 {
            let center_x = coh_x / coh_weight;
            let center_y = coh_y / coh_weight;
            let toward_x = center_x - me.x;
            let toward_y = center_y - me.y;
            fx += toward_x * config.cohesion_weight * my_genome.school_affinity * schooling_mult * 0.01;
            fy += toward_y * config.cohesion_weight * my_genome.school_affinity * schooling_mult * 0.01;
        }

        // Boundary avoidance
        let margin = config.boundary_margin;
        if me.x < margin {
            let t = 1.0 - me.x / margin;
            fx += t * t * config.base_max_speed;
        }
        if me.x > config.tank_width - margin {
            let t = 1.0 - (config.tank_width - me.x) / margin;
            fx -= t * t * config.base_max_speed;
        }
        if me.y < margin {
            let t = 1.0 - me.y / margin;
            fy += t * t * config.base_max_speed;
        }
        if me.y > config.tank_height - margin {
            let t = 1.0 - (config.tank_height - me.y) / margin;
            fy -= t * t * config.base_max_speed;
        }

        // Obstacle avoidance (decorations)
        for &(ox, oy, radius) in obstacles {
            let avoidance_radius = radius + config.boundary_margin * 0.5;
            let dx = me.x - ox;
            let dy = me.y - oy;
            let dist_sq = dx * dx + dy * dy;
            let avoidance_sq = avoidance_radius * avoidance_radius;
            if dist_sq < avoidance_sq && dist_sq > 0.01 {
                let dist = dist_sq.sqrt();
                let t = 1.0 - dist / avoidance_radius;
                let force = t * t * config.base_max_speed;
                fx += (dx / dist) * force;
                fy += (dy / dist) * force;
            }
        }

        // Wander force (Perlin noise)
        let noise_val = self.perlin.get([
            me.x as f64 * 0.01 + tick as f64 * 0.01 * my_genome.curiosity as f64,
            me.y as f64 * 0.01 + (fish_idx as f64) * 100.0,
        ]) as f32;
        let wander_angle = noise_val * std::f32::consts::TAU;
        fx += wander_angle.cos() * config.wander_strength * my_genome.curiosity;
        fy += wander_angle.sin() * config.wander_strength * my_genome.curiosity;

        // Water current
        if config.current_strength > 0.0 {
            fx += config.current_direction.cos() * config.current_strength;
            fy += config.current_direction.sin() * config.current_strength;
        }

        // Hunger drive — steer toward nearest food
        if me.hunger > 0.6 && !food_positions.is_empty() {
            let mut nearest_dist = f32::MAX;
            let mut nearest_fx = 0.0_f32;
            let mut nearest_fy = 0.0_f32;
            for &(food_x, food_y) in food_positions {
                let dx = food_x - me.x;
                let dy = food_y - me.y;
                let d = (dx * dx + dy * dy).sqrt();
                if d < nearest_dist {
                    nearest_dist = d;
                    nearest_fx = dx;
                    nearest_fy = dy;
                }
            }
            if nearest_dist < 200.0 && nearest_dist > 0.01 {
                let urgency = (me.hunger - 0.6) / 0.4; // 0..1
                fx += (nearest_fx / nearest_dist) * urgency * my_genome.speed * config.base_max_speed;
                fy += (nearest_fy / nearest_dist) * urgency * my_genome.speed * config.base_max_speed;
            }
        }

        // Territory return force: steer back to territory center when outside
        if let Some((tcx, tcy)) = me.territory_center {
            let dx = tcx - me.x;
            let dy = tcy - me.y;
            let dist = (dx * dx + dy * dy).sqrt();
            let radius = me.territory_radius;
            if dist > radius * 0.7 {
                // Proportional pull back toward center
                let urgency = ((dist - radius * 0.7) / (radius * 0.3)).min(2.0);
                fx += (dx / dist.max(0.01)) * urgency * config.base_max_speed * 0.5;
                fy += (dy / dist.max(0.01)) * urgency * config.base_max_speed * 0.5;
            }
        }

        // Hunting: chase force toward target
        if me.behavior == BehaviorState::Hunting {
            if let Some(target_id) = me.hunting_target {
                if let Some(target) = fish.iter().find(|f| f.id == target_id && f.is_alive) {
                    let dx = target.x - me.x;
                    let dy = target.y - me.y;
                    let dist = (dx * dx + dy * dy).sqrt().max(0.01);
                    let chase_strength = my_genome.speed * config.base_max_speed * 1.5;
                    fx += (dx / dist) * chase_strength;
                    fy += (dy / dist) * chase_strength;
                }
            }
        }

        // Prey fleeing with school coordination: coordinated escape heading
        if me.behavior == BehaviorState::Fleeing && me.fleeing_from.is_some() {
            if my_genome.school_affinity > 0.6 {
                // Nearby allies influence flee direction (average heading away from predator)
                let mut flee_dx = 0.0_f32;
                let mut flee_dy = 0.0_f32;
                let mut flee_count = 0_u32;
                if let Some(pred_id) = me.fleeing_from {
                    if let Some(predator) = fish.iter().find(|f| f.id == pred_id) {
                        for &j in &candidates {
                            if j == fish_idx { continue; }
                            let ally = &fish[j];
                            if !ally.is_alive || ally.behavior != BehaviorState::Fleeing { continue; }
                            let adx = ally.x - predator.x;
                            let ady = ally.y - predator.y;
                            let adist = (adx * adx + ady * ady).sqrt().max(0.01);
                            flee_dx += adx / adist;
                            flee_dy += ady / adist;
                            flee_count += 1;
                        }
                        if flee_count > 0 {
                            flee_dx /= flee_count as f32;
                            flee_dy /= flee_count as f32;
                            let coord_strength = my_genome.school_affinity * 0.5;
                            fx += flee_dx * coord_strength * config.base_max_speed;
                            fy += flee_dy * coord_strength * config.base_max_speed;
                        }
                    }
                }
            }
        }

        // Apply speed multiplier from behavior state
        fx *= speed_mult;
        fy *= speed_mult;

        (fx, fy)
    }
}
