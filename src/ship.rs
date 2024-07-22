use std::f32::consts::TAU;
use ggez::{Context, GameResult};
use ggez::{graphics, glam};
use enum_dispatch::enum_dispatch;
use std::time::{Instant, Duration};
use std::num::NonZeroU8;
pub mod specs;
pub mod units;
mod collision;
use crate::dim::{Sqrt, Dimensionless};

#[derive(Debug, Clone, Copy)]
struct Gravity(u8);

impl Gravity {
    const NONE: Self = Self(0);
    const ACCELERATE: Self = Self(1);
    const FIELD: Self = Self(2);
    const FULL: Self = Self(3);

    fn supports(self, prop: Gravity) -> bool {
	self.0 & prop.0 != 0
    }
}

#[derive(Debug, Clone, Copy)]
enum Hitbox {
    None,
    Circle {radius: units::TrueSpaceUnit<f32>},
    Line {length: units::TrueSpaceUnit<f32>, radius: units::TrueSpaceUnit<f32>},
}

#[derive(Debug, Clone, Copy)]
enum ObjectType {
    Planet,
    Asteroid,
    Ship,
    Projectile,
}

// Silent takes priority
enum CollisionType {
    Silent,
    Kinetic,
}

#[derive(Debug)]
pub struct ActorSpec {
    maxspeed: units::TrueSpaceUnitPerSecond<f32>,
    acceleration: units::TrueSpaceUnitPerSecond2<f32>,
    mass: units::Ton<f32>,
    turnspeed: units::RadianPerSecond<f32>,
    turnacceleration: units::RadianPerSecond2<f32>,
    moment: units::TonTrueSpaceUnit2<f32>,
    gravity: Gravity,
    hitbox: Hitbox,
    objecttype: ObjectType,
}

pub struct Actor {
    native: ActorNative,
    generator: ActorGeneratorEnum,
    translator: ActorTranslatorEnum,
}

impl Actor {
    pub fn dead(&self) -> bool {
	self.native.dead
    }
    
    fn new(native: ActorNative, generator: ActorGeneratorEnum, translator: ActorTranslatorEnum) -> Self {
	Actor {
	    native,
	    generator,
	    translator,
	}
    }

    pub fn draw(&mut self, _ctx: &mut Context, canvas: &mut graphics::Canvas) -> GameResult {
	if !self.dead() {
	    canvas.draw(
		&self.native.image,
		graphics::DrawParam::default()
		    .offset(glam::vec2(0.5, 0.5))
		    .rotation(self.native.direction)
		    .dest(glam::vec2(
			*(self.native.x / units::TSU).value(),
			*(self.native.y / units::TSU).value()
		    ))
	    );
	}
	Ok(())
    }
    
    pub fn update(&mut self, ctx: &mut Context, time: Instant) -> GameResult<Vec<Actor>> {
	let input = self.generator.update(&mut self.native, &mut self.translator, ctx)?;
	let request = self.translator.update(&mut self.native, &mut self.generator, ctx, input, time)?;
	self.native.update(ctx, request.steer, request.throttle)?;
	Ok(request.summon)
    }

    fn with_velocity(mut self, velocity: (units::TrueSpaceUnitPerSecond<f32>, units::TrueSpaceUnitPerSecond<f32>)) -> Self {
	(self.native.dx, self.native.dy) = velocity;
	self
    }
    
    pub fn interact(&mut self, ctx: &mut Context, other: &mut Actor) {
	self.gravitate(ctx, other);
	self.collide(ctx, other);
    }
    
    fn collide<'a>(mut self: &'a mut Self, ctx: &mut Context, mut other: &'a mut Actor) {
	if matches!(self.native.specs.hitbox, Hitbox::None) || matches!(other.native.specs.hitbox, Hitbox::None) {
	    return;
	}

	if matches!(self.native.specs.hitbox, Hitbox::Circle{..}) && matches!(other.native.specs.hitbox, Hitbox::Line{..}) {
	    // swap the pointers, affects this function only
	    std::mem::swap(&mut self, &mut other);
	}

	if let Some(normal) = self.contacting(other) {
	    let local = self.translator.collide(&mut self.native, &mut self.generator, ctx, other);
	    let remote = other.translator.collide(&mut other.native, &mut other.generator, ctx, self);
	    if matches!(local, CollisionType::Kinetic) && matches!(remote, CollisionType::Kinetic) {
		// reverse the frame that pushed us into the block
		let time = ctx.time.delta().as_secs_f32() * units::S;
		self.native.x -= self.native.dx * time;
		self.native.y -= self.native.dy * time;
		other.native.x -= other.native.dx * time;
		other.native.y -= other.native.dy * time;
		
		collision::reflect(&mut self.native, &mut other.native, normal);
	    }
	}
    }

    fn contacting(&self, other: &Actor) -> Option<nalgebra::Vector2<f32>> {
	use nalgebra::Vector2;
	match self.native.specs.hitbox {
	    Hitbox::None => unreachable!(),
	    Hitbox::Circle {radius: local} => match other.native.specs.hitbox {
		Hitbox::None => unreachable!(),
		Hitbox::Circle {radius: remote} => {
		    let distx = self.native.x - other.native.x;
		    let disty = self.native.y - other.native.y;
		    
		    let distsq = distx*distx + disty*disty;
		    let collisiondist = local + remote;
		    
		    if distsq < collisiondist*collisiondist {
			return Some(Vector2::new(distx.value_unsafe, disty.value_unsafe));
		    }
		},
		Hitbox::Line{..} => unreachable!(),
	    },
	    Hitbox::Line {length, radius} => match other.native.specs.hitbox {
		Hitbox::None => unreachable!(),
		Hitbox::Circle {radius: remote} => return self.line_contacting_circle((other.native.x, other.native.y), length, radius + remote),
		Hitbox::Line {length: remote, radius: remoteradius} => {
		    let totalradius = radius + remoteradius;
		    
		    let offsetx = self.native.direction.cos() * length * 0.5;
		    let offsety = self.native.direction.sin() * length * 0.5;
		    if let Some(normal) = other.line_contacting_circle((self.native.x + offsetx, self.native.y + offsety), remote, totalradius) {
			return Some(normal);
		    }
		    if let Some(normal) = other.line_contacting_circle((self.native.x - offsetx, self.native.y - offsety), remote, totalradius) {
			return Some(normal);
		    }
		    
		    let offsetx = other.native.direction.cos() * remote * 0.5;
		    let offsety = other.native.direction.sin() * remote * 0.5;
		    if let Some(normal) = self.line_contacting_circle((other.native.x + offsetx, other.native.y + offsety), length, totalradius) {
			return Some(normal);
		    }
		    if let Some(normal) = self.line_contacting_circle((other.native.x - offsetx, other.native.y - offsety), length, totalradius) {
			return Some(normal);
		    }
		},
	    },
	}
	None
    }

    fn line_contacting_circle(&self, (otherx, othery): (units::TrueSpaceUnit<f32>, units::TrueSpaceUnit<f32>), length: units::TrueSpaceUnit<f32>, totalradius: units::TrueSpaceUnit<f32>) -> Option<nalgebra::Vector2<f32>> {
	use nalgebra::{Vector2, Matrix2, Rotation2};
	let dist = Vector2::new((self.native.x - otherx).value_unsafe, (self.native.y - othery).value_unsafe);
	let toaxis = Matrix2::from(Rotation2::new(-self.native.direction)) / length.value_unsafe;
	let inline = toaxis * dist;
	if inline.x.abs() <= 0.5 {
	    let targetradius = *(totalradius / length).value();
	    if inline.y.abs() < targetradius {
		// right angles to the direction, sign does not matter
		return Some(Vector2::new(-self.native.direction.sin(), self.native.direction.cos()));
	    }
	} else {
	    let factor = 0.5f32.copysign(-inline.x) * length;
	    let offsetx = self.native.direction.cos() * factor;
	    let offsety = self.native.direction.sin() * factor;
	    
	    let srcx = self.native.x + offsetx;
	    let srcy = self.native.y + offsety;
	    let distx = srcx - otherx;
	    let disty = srcy - othery;
	    
	    let distsq = distx*distx + disty*disty;
	    
	    if distsq < totalradius*totalradius {
		return Some(Vector2::new(distx.value_unsafe, disty.value_unsafe));
	    }
	}
	None
    }

    fn gravitate(&mut self, ctx: &mut Context, other: &mut Actor) {
	let time = ctx.time.delta().as_secs_f32() * units::S;
	
	if self.native.specs.gravity.supports(Gravity::FIELD) && other.native.specs.gravity.supports(Gravity::ACCELERATE) || self.native.specs.gravity.supports(Gravity::ACCELERATE) && other.native.specs.gravity.supports(Gravity::FIELD) {
	    let distx = self.native.x - other.native.x;
	    let disty = self.native.y - other.native.y;
	    let distsq = distx*distx + disty*disty;
	    let dist = distsq.sqrt();
	    let factor = units::G / (distsq * dist) * time;// G t / r^3: kg^-1 s^-1
	    
	    if self.native.specs.gravity.supports(Gravity::FIELD) && other.native.specs.gravity.supports(Gravity::ACCELERATE) {// gravitational acceleration of other
		let total = factor * self.native.specs.mass;
		let dx = total * distx;
		let dy = total * disty;
		other.native.dx += dx;
		other.native.dy += dy;
	    }
	    
	    if self.native.specs.gravity.supports(Gravity::ACCELERATE) && other.native.specs.gravity.supports(Gravity::FIELD) {// gravitational acceleration of self
		let total = factor * other.native.specs.mass;
		let dx = total * distx;
		let dy = total * disty;
		self.native.dx -= dx;
		self.native.dy -= dy;
	    }
	}
    }
}

#[derive(Debug, Clone)]
pub struct ActorNative {
    image: graphics::Image,
    x: units::TrueSpaceUnit<f32>,
    y: units::TrueSpaceUnit<f32>,
    direction: f32,
    angularvelocity: units::RadianPerSecond<f32>,
    dx: units::TrueSpaceUnitPerSecond<f32>,
    dy: units::TrueSpaceUnitPerSecond<f32>,
    specs: &'static ActorSpec,
    affiliation: Option<NonZeroU8>,
    dead: bool,
}

impl ActorNative {
    pub fn new(image: graphics::Image, ((x, y), direction): ((units::TrueSpaceUnit<f32>, units::TrueSpaceUnit<f32>), f32), specs: &'static ActorSpec, affiliation: Option<NonZeroU8>) -> Self {
	Self {
	    image,
	    x,
	    y,
	    direction,
	    angularvelocity: 0.0 * units::RADpS,
	    dx: 0.0 * units::TSUpS,
	    dy: 0.0 * units::TSUpS,
	    specs,
	    affiliation,
	    dead: false,
	}
    }
    
    fn update(&mut self, ctx: &mut Context, steer: f32, throttle: f32) -> GameResult {
	let time = ctx.time.delta().as_secs_f32() * units::S;
	
	let targetangularvelocity = self.specs.turnspeed * steer;
	let startangularvelocity = self.angularvelocity;
	if startangularvelocity < targetangularvelocity {
	    self.angularvelocity += self.specs.turnacceleration * time;
	    if self.angularvelocity > targetangularvelocity {
		self.angularvelocity = targetangularvelocity;
	    }
	} else {
	    self.angularvelocity -= self.specs.turnacceleration * time;
	    if self.angularvelocity < targetangularvelocity {
		self.angularvelocity = targetangularvelocity;
	    }
	}
	let centerangularvelocity = (startangularvelocity + self.angularvelocity) * 0.5;

	// constant acceleration, so half way between starting and ending velocity is perfect
	let startdx = self.dx;
	let startdy = self.dy;

	// not quite perfect, but close enough
	let centraldirection = self.direction + *(centerangularvelocity * time * 0.5).value();
	
	self.direction += *(centerangularvelocity * time).value();
	self.direction %= TAU;

	if throttle != 0.0 {
	    let a_x = throttle * self.specs.acceleration * centraldirection.cos();
	    let a_y = throttle * self.specs.acceleration * centraldirection.sin();

	    self.dx += a_x * time;
	    self.dy += a_y * time;

	    if self.dx*self.dx + self.dy*self.dy > self.specs.maxspeed*self.specs.maxspeed {
		let speed = (self.dx*self.dx + self.dy*self.dy).sqrt();

		// ensure smooth deceleration from overload
		let mut limit = (startdx*startdx + startdy*startdy).sqrt() - self.specs.acceleration * time;
		if speed > limit {
		    if limit < self.specs.maxspeed {
			limit = self.specs.maxspeed;
		    }
		    
		    let factor = limit / speed;
		    self.dx *= factor;
		    self.dy *= factor;
		}
	    }
	}

	self.x += (startdx + self.dx) * 0.5 * time;
	self.y += (startdy + self.dy) * 0.5 * time;
	
        Ok(())
    }
}

struct Input {
    left: bool,
    right: bool,
    thrust: bool,
    fire: bool,
    secondary: bool,
}

#[enum_dispatch(ActorGeneratorEnum)]
trait ActorGenerator {
    fn update(&mut self, native: &mut ActorNative, translator: &mut ActorTranslatorEnum, ctx: &mut Context) -> GameResult<Input>;
}

impl ActorGenerator for Box<dyn ActorGenerator> {
    fn update(&mut self, native: &mut ActorNative, translator: &mut ActorTranslatorEnum, ctx: &mut Context) -> GameResult<Input> {
	(&mut **self).update(native, translator, ctx)
    }
}

#[enum_dispatch]
enum ActorGeneratorEnum {
    NoControl,
    UserControl,
    Other(Box<dyn ActorGenerator>),
}

struct NoControl;

impl ActorGenerator for NoControl {
    fn update(&mut self, _native: &mut ActorNative, _translator: &mut ActorTranslatorEnum, _ctx: &mut Context) -> GameResult<Input> {
	Ok(Input {
	    left: false,
	    right: false,
	    thrust: false,
	    fire: false,
	    secondary: false,
	})
    }
}

struct UserControl;

impl ActorGenerator for UserControl {
    fn update(&mut self, _native: &mut ActorNative, _translator: &mut ActorTranslatorEnum, ctx: &mut Context) -> GameResult<Input> {
	let left = ctx.keyboard.is_key_pressed(crate::KeyCode::Left);
	let right = ctx.keyboard.is_key_pressed(crate::KeyCode::Right);
	let thrust = ctx.keyboard.is_key_pressed(crate::KeyCode::Up);
	let fire = ctx.keyboard.is_key_pressed(crate::KeyCode::Return);
	let secondary = ctx.keyboard.is_key_pressed(crate::KeyCode::RShift);
	Ok(Input {
	    left,
	    right,
	    thrust,
	    fire,
	    secondary,
	})
    }
}

struct Request {
    steer: f32,
    throttle: f32,
    summon: Vec<Actor>,
}

impl Request {
    fn new(steer: f32, throttle: f32) -> Self {
	Self {
	    steer,
	    throttle,
	    summon: Vec::new(),
	}
    }
}

#[enum_dispatch(ActorTranslatorEnum)]
trait ActorTranslator {
    fn update(&mut self, native: &mut ActorNative, generator: &mut ActorGeneratorEnum, ctx: &mut Context, input: Input, time: Instant) -> GameResult<Request>;
    fn collide(&mut self, native: &mut ActorNative, generator: &mut ActorGeneratorEnum, ctx: &mut Context, other: &mut Actor) -> CollisionType;
}

impl ActorTranslator for Box<dyn ActorTranslator> {
    fn update(&mut self, native: &mut ActorNative, generator: &mut ActorGeneratorEnum, ctx: &mut Context, input: Input, time: Instant) -> GameResult<Request> {
	(&mut **self).update(native, generator, ctx, input, time)
    }
    fn collide(&mut self, native: &mut ActorNative, generator: &mut ActorGeneratorEnum, ctx: &mut Context, other: &mut Actor) -> CollisionType {
	(&mut **self).collide(native, generator, ctx, other)
    }
}

struct Planet;

impl ActorTranslator for Planet {
    fn update(&mut self, _native: &mut ActorNative, _generator: &mut ActorGeneratorEnum, _ctx: &mut Context, _input: Input, _time: Instant) -> GameResult<Request> {
	Ok(
	    Request {
		steer: 0.0,
		throttle: 0.0,
		summon: Vec::new(),
	    }
	)
    }

    fn collide(&mut self, _native: &mut ActorNative, _generator: &mut ActorGeneratorEnum, _ctx: &mut Context, _other: &mut Actor) -> CollisionType {
	CollisionType::Kinetic
    }
}

#[enum_dispatch]
enum ActorTranslatorEnum {
    Planet,
    Cruiser(specs::Cruiser),
    CruiserMissile(specs::CruiserMissile),
    Other(Box<dyn ActorTranslator>),
}

pub fn gen_planet(ctx: &mut Context, position: ((units::TrueSpaceUnit<f32>, units::TrueSpaceUnit<f32>), f32), _time: Instant) -> Actor {
    let image = graphics::Image::from_path(ctx, "/planets/rainbow.png").expect("missing image");

    let native = ActorNative::new(image, position, &PLANET, None);
    Actor::new(native, NoControl.into(), Planet.into())
}

pub static PLANET: ActorSpec = ActorSpec {
    maxspeed: units::TrueSpaceUnitPerSecond::new(0.0),
    acceleration: units::TrueSpaceUnitPerSecond2::new(0.0),
    mass: units::Ton::new(1.0e23),
    turnspeed: units::RadianPerSecond::new(0.0),
    turnacceleration: units::RadianPerSecond2::new(0.0),
    moment: units::TonTrueSpaceUnit2::new(9.0e26),
    gravity: Gravity::FIELD,
    hitbox: Hitbox::Circle {radius: units::TrueSpaceUnit::new(150.0)},
    objecttype: ObjectType::Planet,
};

#[derive(Debug, Clone, Copy)]
struct TimeToLive {
    endtime: Instant,
}

impl TimeToLive {
    fn new(now: Instant, ttl: Duration) -> Self {
	Self {
	    endtime: now + ttl,
	}
    }

    fn done(self, now: Instant) -> bool {
	now > self.endtime
    }
}

#[derive(Debug, Clone, Copy)]
struct FireRate {
    nextshot: Instant,
    cooldown: Duration,// which is really static, but it's small
}

impl FireRate {
    fn new(now: Instant, cooldown: Duration) -> Self {
	Self {
	    nextshot: now,
	    cooldown,
	}
    }

    fn try_fire(&mut self, now: Instant) -> bool {
	if now > self.nextshot {
	    self.nextshot = now + self.cooldown;
	    true
	} else {
	    false
	}
    }
}
