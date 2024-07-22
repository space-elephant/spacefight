use std::f32::consts::TAU;
use super::*;
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
    fn update(&mut self, native: &mut ActorNative, _generator: &mut ActorGeneratorEnum, _ctx: &mut Context, input: Input, time: Instant) -> GameResult<Request> {
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
	    let unit = (native.direction.cos(), native.direction.sin());
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

	Ok(Request{steer, throttle, summon})
    }
    
    fn collide(&mut self, _native: &mut ActorNative, _generator: &mut ActorGeneratorEnum, _ctx: &mut Context, _other: &mut Actor) -> CollisionType {
	CollisionType::Kinetic
    }
}

pub static CRUISER: ActorSpec = ActorSpec {
    maxspeed: units::TrueSpaceUnitPerSecond::new(576.0),
    acceleration: units::TrueSpaceUnitPerSecond2::new(345.6),
    mass: units::Ton::new(6.0),
    turnspeed: units::RadianPerSecond::new(0.75 * TAU),
    turnacceleration: units::RadianPerSecond2::new(12.0 * TAU),
    moment: units::TonTrueSpaceUnit2::new(11234.5),
    gravity: Gravity::ACCELERATE,
    hitbox: Hitbox::Line {
	length: units::TrueSpaceUnit::new(107.0),
	radius: units::TrueSpaceUnit::new(19.0),
    },
    objecttype: ObjectType::Ship
};

pub struct CruiserMissile {
    ttl: TimeToLive,
}

impl ActorTranslator for CruiserMissile {
    fn update(&mut self, native: &mut ActorNative, _generator: &mut ActorGeneratorEnum, _ctx: &mut Context, _input: Input, time: Instant) -> GameResult<Request> {
	if self.ttl.done(time) {
	    native.dead = true;
	    Ok(Request::new(0.0, 0.0))
	} else {
	    Ok(Request::new(0.0, 1.0))
	}
    }
    
    fn collide(&mut self, native: &mut ActorNative, _generator: &mut ActorGeneratorEnum, _ctx: &mut Context, _other: &mut Actor) -> CollisionType {
	native.dead = true;
	CollisionType::Silent
    }
}

pub static CRUISERMISSILE: ActorSpec = ActorSpec {
    maxspeed: units::TrueSpaceUnitPerSecond::new(1920.0),
    acceleration: units::TrueSpaceUnitPerSecond2::new(345.6),
    mass: units::Ton::new(1.0),
    turnspeed: units::RadianPerSecond::new(0.0),
    turnacceleration: units::RadianPerSecond2::new(0.0),
    moment: units::TonTrueSpaceUnit2::new(1291.0),
    gravity: Gravity::ACCELERATE,
    hitbox: Hitbox::Line {
	length: units::TrueSpaceUnit::new(76.0),
	radius: units::TrueSpaceUnit::new(6.0),
    },
    objecttype: ObjectType::Projectile,
};
