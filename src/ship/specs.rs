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
    pub fn gen(ctx: &mut Context, position: ((units::TrueSpaceUnit<f32>, units::TrueSpaceUnit<f32>), f32), _time: Instant, affiliation: NonZeroU8, generator: ActorGeneratorEnum) -> Actor {
	const FIRERATE: Duration = Duration::new(0, 416_666_667);
	
	let image = graphics::Image::from_path(ctx, "/ships/cruiser/main.png").expect("missing image");
	let native = ActorNative::new(image, position, &CRUISER, Some(affiliation));
	
	let missileimage = graphics::Image::from_path(ctx, "/ships/cruiser/missile.png").expect("missing image");
	let translator = Self {
	    missileimage,
	    firerate: FireRate::new(FIRERATE),
	};

	Actor::new(native, generator, translator.into())
    }
}

impl ActorTranslator for Cruiser {
    fn update(&mut self, native: &mut ActorNative, _generator: &mut ActorGeneratorEnum, _ctx: &mut Context, input: Input, time: Instant, _others: Chain<Iter<Actor>, Iter<Actor>>) -> GameResult<Request> {
	let steer = if input.right {
	    if input.left {0.0} else {1.0}
	} else {
	    if input.left {-1.0} else {0.0}
	};

	let throttle = if input.thrust {1.0} else {0.0};

	let mut summon = Vec::new();
	if input.fire && native.battery >= CruiserMissile::CHARGECOST && self.firerate.try_fire(time) {
	    native.battery -= CruiserMissile::CHARGECOST;
	    let unit = (native.direction.cos(), native.direction.sin());
	    let dx = CruiserMissile::STARTSPEED * unit.0;
	    let dy = CruiserMissile::STARTSPEED * unit.1;
	    let native = ActorNative::new(
		self.missileimage.clone(),
		((native.x + CruiserMissile::STARTOFFSET * unit.0, native.y + CruiserMissile::STARTOFFSET * unit.1), native.direction),
		&CRUISERMISSILE,
		native.affiliation
	    );
	    summon.push(
		Actor::new(
		    native,
		    super::NoControl.into(),
		    CruiserMissile {
			ttl: TimeToLive::new(time, CruiserMissile::TTL),
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
    inertia: units::TrueSpaceUnit2::new(1872.4),
    gravity: Gravity::ACCELERATE,
    hitbox: Hitbox::Line {
	length: units::TrueSpaceUnit::new(107.0),
	radius: units::TrueSpaceUnit::new(19.0),
    },
    objecttype: ObjectType::Ship,
    takesdamage: true,
    maxcrew: 18,
    maxbattery: 18,
    chargetime: Duration::new(0, 375_000_000),
    chargevalue: 1,
};

pub struct CruiserMissile {
    ttl: TimeToLive,
}

impl CruiserMissile {
    const TTL: Duration = Duration::new(2, 500_000_000);
    const STARTSPEED: units::TrueSpaceUnitPerSecond<f32> = units::TrueSpaceUnitPerSecond::new(960.0);
    const STARTOFFSET: units::TrueSpaceUnit<f32> = units::TrueSpaceUnit::new(128.0);
    const DAMAGE: u8 = 4;
    const CHARGECOST: u8 = 9;
}

impl ActorTranslator for CruiserMissile {
    fn update(&mut self, native: &mut ActorNative, _generator: &mut ActorGeneratorEnum, _ctx: &mut Context, _input: Input, time: Instant, others: Chain<Iter<Actor>, Iter<Actor>>) -> GameResult<Request> {
	if self.ttl.done(time) {
	    native.dead = true;
	    return Ok(Request::new(0.0, 0.0));
	}
	
	let mut target: Option<(&Actor, units::TrueSpaceUnit2<f32>)> = None;
	for ship in others {
	    if let Some(affiliation) = ship.native.affiliation {
		if native.affiliation != Some(affiliation) {
		    // Try to chase this one, if better
		    match target {
			None => {
			    let distx = native.x - ship.native.x;
			    let disty = native.y - ship.native.y;
			    let distsq = distx*distx + disty*disty;
			    target = Some((&ship, distsq));
			},
			Some((prev, prevdistsq)) => {
			    let distx = native.x - ship.native.x;
			    let disty = native.y - ship.native.y;
			    let distsq = distx*distx + disty*disty;
			    if distsq < prevdistsq {
				target = Some((&ship, distsq));
			    }
			}
		    }
		}
	    }
	}
	
	let mut steering: f32 = 0.0;
	if let Some((ship, distsq)) = target {
	    let distx = native.x - ship.native.x;
	    let disty = native.y - ship.native.y;

	    let offset = native.direction.sin() * distx - native.direction.cos() * disty;

	    const FULLTURN: f32 = 0.05;// less than this will have proportionally less
	    let offsetsq = offset * offset;
	    let factorsq = offsetsq / distsq;
	    if *factorsq > FULLTURN*FULLTURN {
		steering = 1.0;
	    } else {
		steering = *factorsq.sqrt() / FULLTURN;
	    }

	    if offset < 0.0 * units::TSU {
		steering = -steering;
	    }
	}
	
	Ok(Request::new(steering, 1.0))
    }
    
    fn collide(&mut self, native: &mut ActorNative, _generator: &mut ActorGeneratorEnum, _ctx: &mut Context, other: &mut Actor) -> CollisionType {
	other.damage(CruiserMissile::DAMAGE);
	native.dead = true;
	CollisionType::Silent
    }
}

pub static CRUISERMISSILE: ActorSpec = ActorSpec {
    maxspeed: units::TrueSpaceUnitPerSecond::new(1920.0),
    acceleration: units::TrueSpaceUnitPerSecond2::new(30720.0),
    mass: units::Ton::new(1.0),
    turnspeed: units::RadianPerSecond::new(0.167 * TAU),
    turnacceleration: units::RadianPerSecond2::new(2.67 * TAU),
    inertia: units::TrueSpaceUnit2::new(1291.0),
    gravity: Gravity::ACCELERATE,
    hitbox: Hitbox::Line {
	length: units::TrueSpaceUnit::new(76.0),
	radius: units::TrueSpaceUnit::new(6.0),
    },
    objecttype: ObjectType::Projectile,
    takesdamage: true,
    maxcrew: 4,
    maxbattery: 0,
    chargetime: Duration::new(0, 0),
    chargevalue: 0,
};

pub struct Avenger;

impl Avenger {
    pub fn gen(ctx: &mut Context, position: ((units::TrueSpaceUnit<f32>, units::TrueSpaceUnit<f32>), f32), _time: Instant, affiliation: NonZeroU8, generator: ActorGeneratorEnum) -> Actor {
	let image = graphics::Image::from_path(ctx, "/ships/avenger/main.png").expect("missing image");
	let native = ActorNative::new(image, position, &AVENGER, Some(affiliation));
	let translator = Self;
	
	Actor::new(native, generator, translator.into())
    }
}

impl ActorTranslator for Avenger {
    fn update(&mut self, _native: &mut ActorNative, _generator: &mut ActorGeneratorEnum, _ctx: &mut Context, input: Input, _time: Instant, _others: Chain<Iter<Actor>, Iter<Actor>>) -> GameResult<Request> {	
	let steer = if input.right {
	    if input.left {0.0} else {1.0}
	} else {
	    if input.left {-1.0} else {0.0}
	};

	let throttle = if input.thrust {1.0} else {0.0};

	Ok(Request::new(steer, throttle))
    }
    
    fn collide(&mut self, _native: &mut ActorNative, _generator: &mut ActorGeneratorEnum, _ctx: &mut Context, _other: &mut Actor) -> CollisionType {
	CollisionType::Kinetic
    }
}

pub static AVENGER: ActorSpec = ActorSpec {
    maxspeed: units::TrueSpaceUnitPerSecond::new(600.0),
    acceleration: units::TrueSpaceUnitPerSecond2::new(2880.0),
    mass: units::Ton::new(7.0),
    turnspeed: units::RadianPerSecond::new(0.495 * TAU),
    turnacceleration: units::RadianPerSecond2::new(7.92 * TAU),
    inertia: units::TrueSpaceUnit2::new(2880.0),
    gravity: Gravity::ACCELERATE,
    hitbox: Hitbox::Circle {
	radius: units::TrueSpaceUnit::new(76.0),
    },
    objecttype: ObjectType::Ship,
    takesdamage: true,
    maxcrew: 22,
    maxbattery: 16,
    chargetime: Duration::new(0, 208_333_333),
    chargevalue: 4,
};
