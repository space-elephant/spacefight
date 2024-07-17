use std::f32::consts::TAU;
use super::{Actor, ActorSpec, Request, Input, ActorNative, ActorTranslator, ActorGeneratorEnum};
use super::{TimeToLive, FireRate};
use super::Gravity;
use super::units::*;
use ggez::{Context, GameResult};
use ggez::graphics;
use std::time::{Instant, Duration};
use std::num::NonZeroU8;
use crate::make_static;

pub struct Cruiser {
    missileimage: graphics::Image,
    firerate: FireRate,
    affiliation: Option<NonZeroU8>,
}

impl Cruiser {
    pub fn gen(ctx: &mut Context, position: ((Length, Length), f32), time: Instant, affiliation: Option<NonZeroU8>) -> Actor {
	const FIRERATE: Duration = Duration::new(0, 416_666_667);
	
	let image = graphics::Image::from_path(ctx, "/ships/cruiser/main.png").expect("missing image");
	let missileimage = graphics::Image::from_path(ctx, "/ships/cruiser/missile.png").expect("missing image");

	let translator = Self {
	    missileimage,
	    firerate: FireRate::new(time, FIRERATE),
	    affiliation,
	};

	Actor::new(image, position, &CRUISER, super::UserControl.into(), translator.into())
    }
}

impl ActorTranslator for Cruiser {
    fn update(&mut self, native: &mut ActorNative, _generator: &mut ActorGeneratorEnum, _ctx: &mut Context, input: Input, time: Instant) -> GameResult<Option<Request>> {
	const MISSILETTL: Duration = Duration::new(2, 500_000_000);
	const MISSILESTARTSPEED: Velocity = make_static!(Velocity, 960.0);
	const MISSILESTARTOFFSET: Length = make_static!(Length, 128.0);
	
	let steer = if input.right {
	    if input.left {0.0} else {1.0}
	} else {
	    if input.left {-1.0} else {0.0}
	};

	let throttle = if input.thrust {1.0} else {0.0};

	let mut summon = Vec::new();
	if input.fire && self.firerate.try_fire(time) {
	    let unit = (native.direction.sin(), -native.direction.cos());
	    let dx = MISSILESTARTSPEED * unit.0;
	    let dy = MISSILESTARTSPEED * unit.1;
	    summon.push(
		Actor::new(
		    self.missileimage.clone(),
		    ((native.x + MISSILESTARTOFFSET * unit.0, native.y + MISSILESTARTOFFSET * unit.1), native.direction),
		    &CRUISERMISSILE,
		    super::NoControl.into(),
		    CruiserMissile {
			ttl: TimeToLive::new(time, MISSILETTL),
			affiliation: self.affiliation,
		    }.into(),
		).with_velocity((dx, dy))
	    );
	}

	Ok(Some(Request{steer, throttle, summon}))
    }
}

pub static CRUISER: ActorSpec = ActorSpec {
    maxspeed: make_static!(Velocity, 576.0),
    acceleration: make_static!(Acceleration, 345.6),
    turnspeed: make_static!(AngularVelocity, 0.75 * TAU),
    mass: make_static!(Mass, 6.0),
    gravity: Gravity::ACCELERATE,
};

pub struct CruiserMissile {
    ttl: TimeToLive,
    affiliation: Option<NonZeroU8>,
}

impl ActorTranslator for CruiserMissile {
    fn update(&mut self, _native: &mut ActorNative, _generator: &mut ActorGeneratorEnum, _ctx: &mut Context, _input: Input, time: Instant) -> GameResult<Option<Request>> {
	if self.ttl.done(time) {
	    Ok(None)
	} else {
	    Ok(Some(Request::new(0.0, 1.0)))
	}
    }
}

pub static CRUISERMISSILE: ActorSpec = ActorSpec {
    maxspeed: make_static!(Velocity, 1920.0),
    acceleration: make_static!(Acceleration, 345.6),
    turnspeed: make_static!(AngularVelocity, 0.0),
    mass: make_static!(Mass, 6.0),
    gravity: Gravity::ACCELERATE,
};
