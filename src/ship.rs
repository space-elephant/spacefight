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
use super::Camera;
use core::slice::Iter;
use std::iter::Chain;
use std::ops::BitOr;

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
    maxspeed: units::TrueSpaceUnitPerSecond<f32>,// 24 times MAX_THRUST
    acceleration: units::TrueSpaceUnitPerSecond2<f32>,// 576 times THRUST_INCREMENT / (THRUST_WAIT + 1)
    mass: units::Ton<f32>,// SHIP_MASS
    turnspeed: units::RadianPerSecond<f32>,
    turnacceleration: units::RadianPerSecond2<f32>,
    inertia: units::TrueSpaceUnit2<f32>,// Moment divided by mass
    gravity: Gravity,
    hitbox: Hitbox,
    objecttype: ObjectType,
    takesdamage: bool,
    pub maxcrew: u8,
    pub maxbattery: u8,
    chargetime: Duration,
    chargevalue: u8,
    pub species: &'static str,
    pub captainsrc: Option<&'static str>,
}

pub struct Actor {
    native: ActorNative,
    generator: ActorGeneratorEnum,
    translator: ActorTranslatorEnum,
}

impl Actor {
    fn damage(&mut self, damage: u8) {
	if self.native.specs.takesdamage {
	    self.native.crew = self.native.crew.saturating_sub(damage);
	    if self.native.crew == 0 {
		self.native.dead = true;
	    }
	}
    }
    
    pub fn get_pos(&self) -> (units::TrueSpaceUnit<f32>, units::TrueSpaceUnit<f32>) {
	(self.native.x, self.native.y)
    }
    
    pub fn with_camera(mut self, camera: bool) -> Self {
	self.native.maintaincamera = camera;
	self
    }

    pub fn has_camera(&self) -> bool {
	self.native.maintaincamera
    }
    
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

    pub fn draw(&mut self, ctx: &mut Context, canvas: &mut graphics::Canvas, camera: Camera, time: Instant, others: Chain<Iter<Actor>, Iter<Actor>>) -> GameResult {
	if !self.dead() {
	    // units::TSU because a pixel in the images is the same as a TSU
	    let scale = *(camera.scale * units::TSU).value();
	    canvas.draw(
		&self.native.image,
		graphics::DrawParam::default()
		    .offset(glam::vec2(0.5, 0.5))
		    .rotation(self.native.direction)
		    .dest(glam::vec2(
			*(self.native.x * camera.scale).value() - camera.left,
			*(self.native.y * camera.scale).value() - camera.top
		    ))
		    .scale(glam::vec2(scale, scale))
	    );
	    
	    let input = self.generator.update(&mut self.native, &mut self.translator, ctx, others.clone())?.normalize();
	    self.translator.update_captain(&mut self.native, &mut self.generator, ctx, input, time, others)?;
	}
	Ok(())
    }
    
    pub fn update(&mut self, ctx: &mut Context, time: Instant, others: Chain<Iter<Actor>, Iter<Actor>>) -> GameResult<Vec<Actor>> {
	let input = self.generator.update(&mut self.native, &mut self.translator, ctx, others.clone())?.normalize();
	let request = self.translator.update(&mut self.native, &mut self.generator, ctx, input, time, others)?;
	self.native.update(ctx, request.steer, request.throttle, time)?;
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

	if let Some((normal, angularlocal, angularremote)) = self.contacting(other) {
	    let local = self.translator.collide(&mut self.native, &mut self.generator, ctx, other);
	    let remote = other.translator.collide(&mut other.native, &mut other.generator, ctx, self);
	    if matches!(local, CollisionType::Kinetic) && matches!(remote, CollisionType::Kinetic) {
		// reverse the frame that pushed us into the block
		let time = ctx.time.delta().as_secs_f32() * units::S;
		self.native.x -= self.native.dx * time;
		self.native.y -= self.native.dy * time;
		self.native.direction -= (self.native.angularvelocity * time).value();
		self.native.direction %= TAU;
		other.native.x -= other.native.dx * time;
		other.native.y -= other.native.dy * time;
		other.native.direction -= (other.native.angularvelocity * time).value();
		other.native.direction %= TAU;
		
		collision::reflect(&mut self.native, &mut other.native, normal, angularlocal, angularremote);
	    }
	}
    }

    fn contacting(&self, other: &Actor) -> Option<(nalgebra::Vector2<f32>, units::TrueSpaceUnit<f32>, units::TrueSpaceUnit<f32>)> {
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
			return Some((Vector2::new(distx.value_unsafe, disty.value_unsafe), 0.0 * units::TSU, 0.0 * units::TSU));
		    }
		},
		Hitbox::Line{..} => unreachable!(),
	    },
	    Hitbox::Line {length, radius} => match other.native.specs.hitbox {
		Hitbox::None => unreachable!(),
		Hitbox::Circle {radius: remote} => if let Some((normal, angularlocal)) = self.line_contacting_circle((other.native.x, other.native.y), length, radius + remote) {
		    return Some((normal, angularlocal, 0.0 * units::TSU));
		},
		Hitbox::Line {length: remote, radius: remoteradius} => {
		    let totalradius = radius + remoteradius;

		    let cos = self.native.direction.cos();
		    let sin = self.native.direction.sin();
		    let offsetx = cos * length * 0.5;
		    let offsety = sin * length * 0.5;
		    if let Some((normal, angularremote)) = other.line_contacting_circle((self.native.x + offsetx, self.native.y + offsety), remote, totalradius) {
			let unit = Vector2::new(sin, -cos);
			let product = unit.dot(&normal.normalize());
			let angularlocal = product * length * 0.5;
			return Some((normal, angularlocal, angularremote));
		    }
		    if let Some((normal, angularremote)) = other.line_contacting_circle((self.native.x - offsetx, self.native.y - offsety), remote, totalradius) {
			let unit = Vector2::new(-sin, cos);
			let product = unit.dot(&normal.normalize());
			let angularlocal = product * length * 0.5;
			return Some((normal, angularlocal, angularremote));
		    }
		    
		    let cos = other.native.direction.cos();
		    let sin = other.native.direction.sin();
		    let offsetx = cos * remote * 0.5;
		    let offsety = sin * remote * 0.5;
		    if let Some((normal, angularlocal)) = self.line_contacting_circle((other.native.x + offsetx, other.native.y + offsety), length, totalradius) {
			let unit = Vector2::new(sin, -cos);
			let product = unit.dot(&normal.normalize());
			let angularremote = product * remote * 0.5;
			return Some((normal, angularlocal, angularremote));
		    }
		    if let Some((normal, angularlocal)) = self.line_contacting_circle((other.native.x - offsetx, other.native.y - offsety), length, totalradius) {
			let unit = Vector2::new(-sin, cos);
			let product = unit.dot(&normal.normalize());
			let angularremote = product * remote * 0.5;
			return Some((normal, angularlocal, angularremote));
		    }
		},
	    },
	}
	None
    }

    fn line_contacting_circle(&self, (otherx, othery): (units::TrueSpaceUnit<f32>, units::TrueSpaceUnit<f32>), length: units::TrueSpaceUnit<f32>, totalradius: units::TrueSpaceUnit<f32>) -> Option<(nalgebra::Vector2<f32>, units::TrueSpaceUnit<f32>)> {
	use nalgebra::{Vector2, Matrix2, Rotation2};
	let dist = Vector2::new((self.native.x - otherx).value_unsafe, (self.native.y - othery).value_unsafe);
	let toaxis = Matrix2::from(Rotation2::new(-self.native.direction)) / length.value_unsafe;
	let inline = toaxis * dist;// self is horizontal, from -0.5 to 0.5
	if inline.x.abs() <= 0.5 {
	    let targetradius = *(totalradius / length).value();
	    if inline.y.abs() < targetradius {
		// right angles to the direction, sign does not matter
		return Some((
		    Vector2::new(-self.native.direction.sin(), self.native.direction.cos()),
		    -inline.x * length,
		));
	    }
	} else {
	    // maybe use transformed coords, which could be duplicated
	    let factor = 0.5f32.copysign(-inline.x) * length;
	    let offsetx = self.native.direction.cos() * factor;
	    let offsety = self.native.direction.sin() * factor;
	    
	    let srcx = self.native.x + offsetx;
	    let srcy = self.native.y + offsety;
	    let distx = srcx - otherx;
	    let disty = srcy - othery;
	    
	    let distsq = distx*distx + disty*disty;
	    
	    if distsq < totalradius*totalradius {
		let inlinelength = (inline.x*inline.x + inline.y*inline.y).sqrt();
		return Some((
		    Vector2::new(distx.value_unsafe, disty.value_unsafe),
		    factor * inline.y / inlinelength
		));
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
    maintaincamera: bool,
    crew: u8,
    battery: u8,
    recharge: FireRate,
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
	    maintaincamera: false,
	    crew: specs.maxcrew,
	    battery: specs.maxbattery,
	    recharge: FireRate::new(specs.chargetime),
	}
    }
    
    fn update(&mut self, ctx: &mut Context, steer: f32, throttle: f32, now: Instant) -> GameResult {
	if self.battery != self.specs.maxbattery && self.recharge.try_fire(now) {
	    self.battery = self.battery.saturating_add(self.specs.chargevalue);
	    if self.battery > self.specs.maxbattery {
		self.battery = self.specs.maxbattery;
	    }
	}
	
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

#[derive(Clone, Copy, Default)]
pub struct Input (u8);

impl Input {
    pub const LEFT: Input = Input(1 << 0);
    pub const RIGHT: Input = Input(1 << 1);
    pub const THRUST: Input = Input(1 << 2);
    pub const FIRE: Input = Input(1 << 3);
    pub const SECONDARY: Input = Input(1 << 4);

    pub fn normalize(mut self) -> Self {
	if self.0 & (Self::LEFT.0 | Self::RIGHT.0) == (Self::LEFT.0 | Self::RIGHT.0) {
	    self.0 &= !(Self::LEFT.0 | Self::RIGHT.0);
	}
	self
    }

    pub fn new(left: bool, right: bool, thrust: bool, fire: bool, secondary: bool) -> Self {
	Input((left as u8) << 0 | (right as u8) << 1 | (thrust as u8) << 2 | (fire as u8) << 3 | (secondary as u8) << 4)
    }

    pub fn is(self, other: Input) -> bool {
	self.0 & other.0 != 0
    }
}

impl BitOr for Input {
    // see bitflags crate
    type Output = Input;
    
    fn bitor(self, other: Input) -> Input {
	Input(self.0 | other.0)
    }
}

#[enum_dispatch(ActorGeneratorEnum)]
trait ActorGenerator {
    fn update(&mut self, native: &mut ActorNative, translator: &mut ActorTranslatorEnum, ctx: &mut Context, others: Chain<Iter<Actor>, Iter<Actor>>) -> GameResult<Input>;
}

impl ActorGenerator for Box<dyn ActorGenerator> {
    fn update(&mut self, native: &mut ActorNative, translator: &mut ActorTranslatorEnum, ctx: &mut Context, others: Chain<Iter<Actor>, Iter<Actor>>) -> GameResult<Input> {
	(&mut **self).update(native, translator, ctx, others)
    }
}

#[enum_dispatch]
#[allow(private_interfaces)]
pub enum ActorGeneratorEnum {
    NoControl,
    UserControl,
    Other(Box<dyn ActorGenerator>),
}

pub struct NoControl;

impl ActorGenerator for NoControl {
    fn update(&mut self, _native: &mut ActorNative, _translator: &mut ActorTranslatorEnum, _ctx: &mut Context, _others: Chain<Iter<Actor>, Iter<Actor>>) -> GameResult<Input> {
	Ok(Input::new(false, false, false, false, false))
    }
}

pub struct UserControl;

impl ActorGenerator for UserControl {
    fn update(&mut self, _native: &mut ActorNative, _translator: &mut ActorTranslatorEnum, ctx: &mut Context, _others: Chain<Iter<Actor>, Iter<Actor>>) -> GameResult<Input> {
	let left = ctx.keyboard.is_key_pressed(crate::KeyCode::Left);
	let right = ctx.keyboard.is_key_pressed(crate::KeyCode::Right);
	let thrust = ctx.keyboard.is_key_pressed(crate::KeyCode::Up);
	let fire = ctx.keyboard.is_key_pressed(crate::KeyCode::Return);
	let secondary = ctx.keyboard.is_key_pressed(crate::KeyCode::RShift);
	Ok(Input::new(
	    left,
	    right,
	    thrust,
	    fire,
	    secondary,
	))
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
    fn update(&mut self, native: &mut ActorNative, generator: &mut ActorGeneratorEnum, ctx: &mut Context, input: Input, time: Instant, others: Chain<Iter<Actor>, Iter<Actor>>) -> GameResult<Request>;
    fn collide(&mut self, native: &mut ActorNative, generator: &mut ActorGeneratorEnum, ctx: &mut Context, other: &mut Actor) -> CollisionType;
    fn update_captain(&mut self, native: &mut ActorNative, generator: &mut ActorGeneratorEnum, ctx: &mut Context, input: Input, time: Instant, others: Chain<Iter<Actor>, Iter<Actor>>) -> GameResult;
}

impl ActorTranslator for Box<dyn ActorTranslator> {
    fn update(&mut self, native: &mut ActorNative, generator: &mut ActorGeneratorEnum, ctx: &mut Context, input: Input, time: Instant, others: Chain<Iter<Actor>, Iter<Actor>>) -> GameResult<Request> {
	(&mut **self).update(native, generator, ctx, input, time, others)
    }
    fn collide(&mut self, native: &mut ActorNative, generator: &mut ActorGeneratorEnum, ctx: &mut Context, other: &mut Actor) -> CollisionType {
	(&mut **self).collide(native, generator, ctx, other)
    }
    fn update_captain(&mut self, native: &mut ActorNative, generator: &mut ActorGeneratorEnum, ctx: &mut Context, input: Input, time: Instant, others: Chain<Iter<Actor>, Iter<Actor>>) -> GameResult {
	(&mut **self).update_captain(native, generator, ctx, input, time, others)
    }
}

#[enum_dispatch]
enum ActorTranslatorEnum {
    Planet,
    Avenger(specs::Avenger),
    Cruiser(specs::Cruiser),
    CruiserMissile(specs::CruiserMissile),
    Other(Box<dyn ActorTranslator>),
}

struct Planet;

impl ActorTranslator for Planet {
    fn update(&mut self, _native: &mut ActorNative, _generator: &mut ActorGeneratorEnum, _ctx: &mut Context, _input: Input, _time: Instant, _others: Chain<Iter<Actor>, Iter<Actor>>) -> GameResult<Request> {
	Ok(
	    Request {
		steer: 0.0,
		throttle: 0.0,
		summon: Vec::new(),
	    }
	)
    }

    fn collide(&mut self, _native: &mut ActorNative, _generator: &mut ActorGeneratorEnum, _ctx: &mut Context, other: &mut Actor) -> CollisionType {
	let mut damage = other.native.crew >> 2;
	if damage == 0 {
	    damage = 1;
	}
	other.damage(damage);
	CollisionType::Kinetic
    }
    
    fn update_captain(&mut self, _native: &mut ActorNative, _generator: &mut ActorGeneratorEnum, _ctx: &mut Context, _input: Input, _time: Instant, _others: Chain<Iter<Actor>, Iter<Actor>>) -> GameResult {
	Ok(())
    }
}

pub fn gen_planet(ctx: &mut Context, position: ((units::TrueSpaceUnit<f32>, units::TrueSpaceUnit<f32>), f32), _time: Instant) -> Actor {
    let image = graphics::Image::from_path(ctx, "/scenery/planets/rainbow.png").expect("missing image");

    let native = ActorNative::new(image, position, &PLANET, None);
    Actor::new(native, NoControl.into(), Planet.into())
}

pub static PLANET: ActorSpec = ActorSpec {
    maxspeed: units::TrueSpaceUnitPerSecond::new(0.0),
    acceleration: units::TrueSpaceUnitPerSecond2::new(0.0),
    mass: units::Ton::new(1.0e23),
    turnspeed: units::RadianPerSecond::new(0.0),
    turnacceleration: units::RadianPerSecond2::new(0.0),
    inertia: units::TrueSpaceUnit2::new(9000.0),
    gravity: Gravity::FIELD,
    hitbox: Hitbox::Circle {radius: units::TrueSpaceUnit::new(150.0)},
    objecttype: ObjectType::Planet,
    takesdamage: false,
    maxcrew: 1,
    maxbattery: 0,
    chargetime: Duration::new(0, 0),
    chargevalue: 0,
    species: "",
    captainsrc: None,
};

#[derive(Debug, Clone, Copy)]
pub struct Timer {
    endtime: Instant,
}

impl Timer {
    pub fn new(now: Instant, ttl: Duration) -> Self {
	Self {
	    endtime: now + ttl,
	}
    }

    pub fn done(self, now: Instant) -> bool {
	now > self.endtime
    }
}

impl Default for Timer {
    fn default() -> Self {
	Timer {
	    endtime: Instant::now(),
	}
    }
}

#[derive(Debug, Clone, Copy)]
struct FireRate {
    nextshot: Instant,
    cooldown: Duration,// which is really static, but it's small
}

impl FireRate {
    fn new(cooldown: Duration) -> Self {
	Self {
	    nextshot: Instant::now(),// maybe make start at 0
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
