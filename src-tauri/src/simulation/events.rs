use rand::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnvironmentalEvent {
    AlgaeBloom,
    ColdSnap,
    Heatwave,
    CurrentSurge,
    PlanktonBloom,
}

impl EnvironmentalEvent {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::AlgaeBloom => "algae_bloom",
            Self::ColdSnap => "cold_snap",
            Self::Heatwave => "heatwave",
            Self::CurrentSurge => "current_surge",
            Self::PlanktonBloom => "plankton_bloom",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::AlgaeBloom => "Algae Bloom",
            Self::ColdSnap => "Cold Snap",
            Self::Heatwave => "Heatwave",
            Self::CurrentSurge => "Current Surge",
            Self::PlanktonBloom => "Plankton Bloom",
        }
    }

    pub fn duration(&self) -> u32 {
        match self {
            Self::AlgaeBloom => 600,
            Self::ColdSnap => 400,
            Self::Heatwave => 400,
            Self::CurrentSurge => 300,
            Self::PlanktonBloom => 500,
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "algae_bloom" => Some(Self::AlgaeBloom),
            "cold_snap" => Some(Self::ColdSnap),
            "heatwave" => Some(Self::Heatwave),
            "current_surge" => Some(Self::CurrentSurge),
            "plankton_bloom" => Some(Self::PlanktonBloom),
            _ => None,
        }
    }

    fn random(rng: &mut impl Rng) -> Self {
        match rng.gen_range(0..5) {
            0 => Self::AlgaeBloom,
            1 => Self::ColdSnap,
            2 => Self::Heatwave,
            3 => Self::CurrentSurge,
            _ => Self::PlanktonBloom,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSystem {
    pub active_event: Option<(EnvironmentalEvent, u32)>, // (type, remaining_ticks)
    pub cooldown: u32,
}

impl EventSystem {
    pub fn new() -> Self {
        Self {
            active_event: None,
            cooldown: 3000,
        }
    }

    pub fn update(&mut self, event_frequency: f32, rng: &mut impl Rng) {
        // Tick active event
        if let Some((_, ref mut remaining)) = self.active_event {
            if *remaining > 0 {
                *remaining -= 1;
            } else {
                self.active_event = None;
                self.cooldown = 3000;
            }
        }

        // Cooldown
        if self.active_event.is_none() && self.cooldown > 0 {
            self.cooldown -= 1;
        }

        // Random event trigger
        if self.active_event.is_none() && self.cooldown == 0 {
            if rng.gen::<f32>() < event_frequency * 0.0001 {
                let event = EnvironmentalEvent::random(rng);
                let duration = event.duration();
                self.active_event = Some((event, duration));
            }
        }
    }

    pub fn trigger(&mut self, event: EnvironmentalEvent) {
        let duration = event.duration();
        self.active_event = Some((event, duration));
    }

    pub fn active_event_name(&self) -> Option<&'static str> {
        self.active_event.as_ref().map(|(e, _)| e.as_str())
    }

    // Event effect modifiers
    pub fn metabolism_multiplier(&self) -> f32 {
        match self.active_event {
            Some((EnvironmentalEvent::ColdSnap, _)) => 0.6,
            Some((EnvironmentalEvent::Heatwave, _)) => 1.5,
            _ => 1.0,
        }
    }

    pub fn speed_multiplier(&self) -> f32 {
        match self.active_event {
            Some((EnvironmentalEvent::ColdSnap, _)) => 0.7,
            _ => 1.0,
        }
    }

    pub fn aggression_bonus(&self) -> f32 {
        match self.active_event {
            Some((EnvironmentalEvent::Heatwave, _)) => 0.2,
            _ => 0.0,
        }
    }

    pub fn current_strength_override(&self) -> Option<f32> {
        match self.active_event {
            Some((EnvironmentalEvent::CurrentSurge, _)) => Some(0.5),
            _ => None,
        }
    }

    pub fn extra_water_degradation(&self) -> f32 {
        match self.active_event {
            Some((EnvironmentalEvent::AlgaeBloom, _)) => 0.001,
            Some((EnvironmentalEvent::PlanktonBloom, _)) => 0.0005,
            _ => 0.0,
        }
    }

    pub fn should_spawn_free_food(&self, tick: u64) -> bool {
        match self.active_event {
            Some((EnvironmentalEvent::PlanktonBloom, _)) => tick % 30 == 0,
            _ => false,
        }
    }

    pub fn energy_drain_multiplier(&self) -> f32 {
        match self.active_event {
            Some((EnvironmentalEvent::Heatwave, _)) => 2.0,
            _ => 1.0,
        }
    }
}
