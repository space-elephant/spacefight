use std::f32::consts::TAU;
use super::{Actor, ActorSpec, Request, Input, ActorNative, ActorTranslator, ActorGeneratorEnum};
use super::{TimeToLive, FireRate, Hitbox};
use super::Gravity;
use super::units;
use ggez::{Context, GameResult};
use ggez::graphics;
use std::time::{Instant, Duration};
use std::num::NonZeroU8;

pub struct Cruiser {
    missileimage: graphics::Image,
    firerate: FireRate,
}

impl Cruiser {
    pub fn gen(ctx: &mut Context, position: ((units::TrueSpaceUnit<f32>, units::TrueSpaceUnit<f32>), f32), time: Instant, affiliation: NonZeroU8) -> Actor {
	const FIRERATE: Duration = Duration::new(0, 416_666_667);
	
	let image = graphics::Image::from_path(ctx, "/ships/cruiser/main.png").expect("missing image");
	let native = ActorNative::new(image, position, &CRUISER, Some(affiliation));
	
	let missileimage = graphics::Image::from_path(ctx, "/ships/cruiser/missile.png").expect("missing image");
	let translator = Self {
	    missileimage,
	    firerate: FireRate::new(time, FIRERATE),
	};

	Actor::new(native, super::UserControl.into(), translator.into())
    }
}

impl ActorTranslator for Cruiser {
    fn update(&mut self, native: &mut ActorNative, _generator: &mut ActorGeneratorEnum, _ctx: &mut Context, input: Input, time: Instant) -> GameResult<Option<Request>> {
	const MISSILETTL: Duration = Duration::new(2, 500_000_000);
	const MISSILESTARTSPEED: units::TrueSpaceUnitPerSecond<f32> = units::TrueSpaceUnitPerSecond::new(960.0);
	const MISSILESTARTOFFSET: units::TrueSpaceUnit<f32> = units::TrueSpaceUnit::new(128.0);
	
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
	    let native = ActorNative::new(
		self.missileimage.clone(),
		((native.x + MISSILESTARTOFFSET * unit.0, native.y + MISSILESTARTOFFSET * unit.1), native.direction),
		&CRUISERMISSILE,
		native.affiliation
	    );
	    summon.push(
		Actor::new(
		    native,
		    super::NoControl.into(),
		    CruiserMissile {
			ttl: TimeToLive::new(time, MISSILETTL),
		    }.into(),
		).with_velocity((dx, dy))
	    );
	}

	Ok(Some(Request{steer, throttle, summon}))
    }
}

pub static CRUISER: ActorSpec = ActorSpec {
    maxspeed: units::TrueSpaceUnitPerSecond::new(576.0),
    acceleration: units::TrueSpaceUnitPerSecond2::new(345.6),
    turnspeed: units::RadianPerSecond::new(0.75 * TAU),
    mass: units::Ton::new(6.0),
    gravity: Gravity::ACCELERATE,
    hitbox: Hitbox::Circle {radius: units::TrueSpaceUnit::new(72.5)},// make line
};

pub struct CruiserMissile {
    ttl: TimeToLive,
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
    maxspeed: units::TrueSpaceUnitPerSecond::new(1920.0),
    acceleration: units::TrueSpaceUnitPerSecond2::new(345.6),
    turnspeed: units::RadianPerSecond::new(0.0),
    mass: units::Ton::new(6.0),
    gravity: Gravity::ACCELERATE,
    hitbox: Hitbox::Circle {radius: units::TrueSpaceUnit::new(44.0)},// make line
};
