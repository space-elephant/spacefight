use std::f32::consts::TAU;

pub static CRUISER: super::ActorSpec = super::ActorSpec {
    radius: 88.5,
    maxspeed: 576.0,
    acceleration: 345.6,
    turnspeed: 0.75 * TAU,
    mass: 6.0,
};
