use std::f32::consts::TAU;
use ggez::{Context, GameResult};
use ggez::{graphics, glam};
pub mod specs;

#[derive(Debug)]
pub struct ActorSpec {
    radius: f32,// world units
    maxspeed: f32,// world units per second
    acceleration: f32,// world units per second squared
    turnspeed: f32,// radians per second
    mass: f32,// arbitrary unit
}

pub struct Actor {
    native: ActorNative,
    generator: Box<dyn ActorGenerator>,
    translator: Box<dyn ActorTranslator>,
}

impl Actor {
    pub fn new(image: graphics::Image, (x, y): (f32, f32), specs: &'static ActorSpec) -> Self {
	Actor {
	    native: ActorNative {
		image,
		x,
		y,
		direction: 0.0,
		dx: 0.0,
		dy: 0.0,
		specs,
	    },
	    generator: Box::new(UserControl),
	    translator: Box::new(Cruiser),
	}
    }

    pub fn draw(&mut self, _ctx: &mut Context, canvas: &mut graphics::Canvas) -> GameResult {
	canvas.draw(&self.native.image, graphics::DrawParam::default().offset(glam::vec2(0.5, 0.5)).rotation(self.native.direction).dest(glam::vec2(self.native.x, self.native.y)));
	Ok(())
    }
    
    pub fn update(&mut self, ctx: &mut Context) -> GameResult {
	let input = self.generator.update(&mut self.native, &mut *self.translator, ctx)?;
	let (steer, throttle) = self.translator.update(&mut self.native, &mut *self.generator, ctx, input)?;
	self.native.update(ctx, steer, throttle)
    }
}

#[derive(Debug, Clone)]
struct ActorNative {
    image: graphics::Image,
    x: f32,
    y: f32,
    direction: f32,
    dx: f32,
    dy: f32,
    specs: &'static ActorSpec,
}

impl ActorNative {
    fn update(&mut self, ctx: &mut Context, steer: f32, throttle: f32) -> GameResult {
	let time = ctx.time.delta().as_secs_f32();
	
	let angular_velocity = self.specs.turnspeed * steer;

	// constant acceleration, so half way between starting and ending velocity is perfect
	let startdx = self.dx;
	let startdy = self.dy;
	// this average will result in slightly too strong acceleration while turning
	// but it's negligable at reasonable frame rates, so who cares
	let centraldirection = self.direction + angular_velocity * time / 2.0;
	
	self.direction += angular_velocity * time;
	self.direction %= TAU;

	if throttle != 0.0 {
	    let a_x = throttle * self.specs.acceleration * centraldirection.sin();
	    let a_y = throttle * self.specs.acceleration * -centraldirection.cos();

	    self.dx += a_x * time;
	    self.dy += a_y * time;

	    if self.dx*self.dx + self.dy*self.dy > self.specs.maxspeed*self.specs.maxspeed {
		let speed = (self.dx*self.dx + self.dy*self.dy).sqrt();

		// ensure smooth deceleration from overload
		let mut limit = (startdx*startdx + startdy*startdy).sqrt() - self.specs.acceleration;
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

	self.x += (startdx + self.dx) / 2.0 * time;
	self.y += (startdy + self.dy) / 2.0 * time;
	
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

trait ActorGenerator {
    fn update(&mut self, native: &mut ActorNative, translator: &mut dyn ActorTranslator, ctx: &mut Context) -> GameResult<Input>;
}

struct UserControl;

impl ActorGenerator for UserControl {
    fn update(&mut self, native: &mut ActorNative, translator: &mut dyn ActorTranslator, ctx: &mut Context) -> GameResult<Input> {
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

trait ActorTranslator {
    fn update(&mut self, native: &mut ActorNative, generator: &mut dyn ActorGenerator, ctx: &mut Context, input: Input) -> GameResult<(f32, f32)>;
}

struct Cruiser;

impl ActorTranslator for Cruiser {
    fn update(&mut self, _native: &mut ActorNative, _generator: &mut dyn ActorGenerator, ctx: &mut Context, input: Input) -> GameResult<(f32, f32)> {
	let steer = if input.right {
	    if input.left {0.0} else {1.0}
	} else {
	    if input.left {-1.0} else {0.0}
	};

	let throttle = if input.thrust {1.0} else {0.0};

	Ok((steer, throttle))
    }
}
