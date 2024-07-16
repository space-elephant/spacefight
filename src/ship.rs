use std::f32::consts::TAU;
use ggez::{Context, GameResult};
use ggez::{graphics, glam};
use enum_dispatch::enum_dispatch;
use std::time::{Instant, Duration};
use std::num::NonZeroU8;
pub mod specs;
pub mod units;
pub mod constants;

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

#[derive(Debug)]
pub struct ActorSpec {
    maxspeed: units::Speed,// world units per second
    acceleration: units::Acceleration,// world units per second squared
    turnspeed: f32,// radians per second
    mass: f32,// arbitrary unit
    gravity: Gravity,
}

pub struct Actor {
    native: ActorNative,
    generator: ActorGeneratorEnum,
    translator: ActorTranslatorEnum,
}

impl Actor {
    fn new(image: graphics::Image, ((x, y), direction): ((units::Distance, units::Distance), f32), specs: &'static ActorSpec, generator: ActorGeneratorEnum, translator: ActorTranslatorEnum) -> Self {
	Actor {
	    native: ActorNative {
		image,
		x,
		y,
		direction,
		dx: 0.0.into(),
		dy: 0.0.into(),
		specs,
	    },
	    generator,
	    translator,
	}
    }

    pub fn draw(&mut self, _ctx: &mut Context, canvas: &mut graphics::Canvas) -> GameResult {
	canvas.draw(&self.native.image, graphics::DrawParam::default().offset(glam::vec2(0.5, 0.5)).rotation(self.native.direction).dest(glam::vec2(self.native.x.into(), self.native.y.into())));
	Ok(())
    }
    
    pub fn update(&mut self, ctx: &mut Context, time: Instant) -> GameResult<Option<Vec<Actor>>> {
	let input = self.generator.update(&mut self.native, &mut self.translator, ctx)?;
	let request = self.translator.update(&mut self.native, &mut self.generator, ctx, input, time)?;
	if let Some(request) = request {
	    self.native.update(ctx, request.steer, request.throttle)?;
	    Ok(Some(request.summon))
	} else {
	    Ok(None)
	}
    }

    fn with_velocity(mut self, velocity: (units::Speed, units::Speed)) -> Self {
	(self.native.dx, self.native.dy) = velocity;
	self
    }

    pub fn gravitate(&mut self, ctx: &mut Context, other: &mut Actor) {
	let time = ctx.time.delta().as_secs_f32();
	
	if self.native.specs.gravity.supports(Gravity::FIELD) && other.native.specs.gravity.supports(Gravity::ACCELERATE) || self.native.specs.gravity.supports(Gravity::ACCELERATE) && other.native.specs.gravity.supports(Gravity::FIELD) {
	    let distx = f32::from(self.native.x - other.native.x);
	    let disty = f32::from(self.native.y - other.native.y);
	    let distsq = distx*distx + disty*disty;
	    let factor = constants::GRAVITY / distsq * distsq.sqrt() * time;// G / r^3 t
	    
	    if self.native.specs.gravity.supports(Gravity::FIELD) && other.native.specs.gravity.supports(Gravity::ACCELERATE) {// gravitational acceleration of other
		let total = factor * self.native.specs.mass;
		let dx = units::Speed::from(total * distx);
		let dy = units::Speed::from(total * disty);
		other.native.dx += dx;
		other.native.dy += dy;
	    }
	    
	    if self.native.specs.gravity.supports(Gravity::ACCELERATE) && other.native.specs.gravity.supports(Gravity::FIELD) {// gravitational acceleration of self
		let total = factor * other.native.specs.mass;
		let dx = units::Speed::from(total * distx);
		let dy = units::Speed::from(total * disty);
		self.native.dx -= dx;
		self.native.dy -= dy;
	    }
	}
    }
}

#[derive(Debug, Clone)]
struct ActorNative {
    image: graphics::Image,
    x: units::Distance,
    y: units::Distance,
    direction: f32,
    dx: units::Speed,
    dy: units::Speed,
    specs: &'static ActorSpec,
}

impl ActorNative {
    fn update(&mut self, ctx: &mut Context, steer: f32, throttle: f32) -> GameResult {
	let time = units::Time::from(ctx.time.delta().as_secs_f32());
	
	let angular_velocity = self.specs.turnspeed * steer;

	// constant acceleration, so half way between starting and ending velocity is perfect
	let startdx = self.dx;
	let startdy = self.dy;
	// this average will result in slightly too strong acceleration while turning
	// but it's negligable at reasonable frame rates, so who cares
	let centraldirection = self.direction + angular_velocity * f32::from(time * 0.5);
	
	self.direction += angular_velocity * f32::from(time);
	self.direction %= TAU;

	if throttle != 0.0 {
	    let a_x = throttle * self.specs.acceleration * centraldirection.sin();
	    let a_y = throttle * self.specs.acceleration * -centraldirection.cos();

	    self.dx += a_x * time;
	    self.dy += a_y * time;

	    if self.dx*self.dx + self.dy*self.dy > self.specs.maxspeed*self.specs.maxspeed {
		let speed = (self.dx.sqr() + self.dy.sqr()).sqrt();

		// ensure smooth deceleration from overload
		let mut limit = (startdx.sqr() + startdy.sqr()).sqrt() - self.specs.acceleration * time;
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
    fn update(&mut self, native: &mut ActorNative, generator: &mut ActorGeneratorEnum, ctx: &mut Context, input: Input, time: Instant) -> GameResult<Option<Request>>;
}

impl ActorTranslator for Box<dyn ActorTranslator> {
    fn update(&mut self, native: &mut ActorNative, generator: &mut ActorGeneratorEnum, ctx: &mut Context, input: Input, time: Instant) -> GameResult<Option<Request>> {
	(&mut **self).update(native, generator, ctx, input, time)
    }
}

struct NoPower;

impl ActorTranslator for NoPower {
    fn update(&mut self, _native: &mut ActorNative, _generator: &mut ActorGeneratorEnum, _ctx: &mut Context, _input: Input, _time: Instant) -> GameResult<Option<Request>> {
	Ok(Some(
	    Request {
		steer: 0.0,
		throttle: 0.0,
		summon: Vec::new(),
	    }
	))
    }
}

#[enum_dispatch]
enum ActorTranslatorEnum {
    NoPower,
    Cruiser(specs::Cruiser),
    CruiserMissile(specs::CruiserMissile),
    Other(Box<dyn ActorTranslator>),
}

pub fn gen_planet(ctx: &mut Context, position: ((units::Distance, units::Distance), f32), time: Instant, affiliation: Option<NonZeroU8>) -> Actor {
    let image = graphics::Image::from_path(ctx, "/planets/rainbow.png").expect("missing image");

    Actor::new(image, position, &PLANET, NoControl.into(), NoPower.into())
}

pub static PLANET: ActorSpec = ActorSpec {
    maxspeed: units::Speed(0.0),
    acceleration: units::Acceleration(0.0),
    turnspeed: 0.0,
    mass: 1.0e21,
    gravity: Gravity::FIELD,
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
