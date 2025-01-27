use ggez::{graphics, glam};
use ggez::{Context, GameResult};
use crate::ship::{Input, Timer};
use std::time::{Instant, Duration};

// held by the ActorTranslator, but only if it is a ship
pub struct Captain<const N: usize> {
    display: graphics::Image,
    activity: super::Animation<N>,
    species: &'static str,
    previnput: Input,
    turntimer: Timer,
    thrusttimer: Timer,
    firetimer: Timer,
    secondarytimer: Timer,
}

impl<const N: usize> Captain<N> {
    const INTERTIME: Duration = Duration::new(0, 15_625_000);
    
    pub fn new(ctx: &mut Context, spec: &'static crate::ship::ActorSpec, name: &str) -> Self {
	ctx.gfx.begin_frame().expect("image init frame");// needed to use canvas
	let activity = super::Animation::new(ctx, spec.captainsrc.expect("no activity image path provided"));
	let base = graphics::Image::from_path(ctx, "/ships/captain-base.png").expect("captain base image");
	let display = graphics::Image::new_canvas_image(ctx, graphics::ImageFormat::Rgba8UnormSrgb, 256, 476, 1);
	let mut canvas = graphics::Canvas::from_image(ctx, display.clone(), graphics::Color::from_rgb(82, 82, 82));
	// TODO: add basic specs
	canvas.draw(
	    &base,
	    graphics::DrawParam::default()
	);
	canvas.draw(
	    &activity.fields[0].image,
	    graphics::DrawParam::default()
		.dest(glam::vec2(16.0, 260.0))
	);
	canvas.finish(ctx).expect("initializing captain");
	ctx.gfx.end_frame().expect("image init frame");
	
	Captain {
	    display,
	    activity,
	    species: spec.species,
	    previnput: Default::default(),
	    turntimer: Default::default(),
	    thrusttimer: Default::default(),
	    firetimer: Default::default(),
	    secondarytimer: Default::default(),
	}
    }

    pub fn extract_display(&self) -> graphics::Image {
	self.display.clone()
    }

    pub fn update_input(&mut self, ctx: &mut Context, new: Input, time: Instant, native: &crate::ship::ActorNative) -> GameResult {
	let mut canvas = graphics::Canvas::from_image(ctx, self.display.clone(), None);
	
	if new.is(Input::RIGHT) != self.previnput.is(Input::RIGHT) {
	    if !new.is(Input::LEFT) {
		self.add_image(ctx, &mut canvas, 2)?;
		self.turntimer = Timer::new(time, Self::INTERTIME);
	    }
	}
	if new.is(Input::LEFT) != self.previnput.is(Input::LEFT) {
	    if !new.is(Input::RIGHT) {
		self.add_image(ctx, &mut canvas, 4)?;
		self.turntimer = Timer::new(time, Self::INTERTIME);
	    }
	}
	if self.turntimer.done(time) {
	    if new.is(Input::RIGHT) {
		self.add_image(ctx, &mut canvas, 1)?;
	    } else if new.is(Input::LEFT) {
		self.add_image(ctx, &mut canvas, 5)?;
	    } else {
		self.add_image(ctx, &mut canvas, 3)?;
	    }
	}
	
	if new.is(Input::THRUST) != self.previnput.is(Input::THRUST) {
	    self.add_image(ctx, &mut canvas, 7)?;
	    self.thrusttimer = Timer::new(time, Self::INTERTIME);
	} else if self.thrusttimer.done(time) {
	    if new.is(Input::THRUST) {
		self.add_image(ctx, &mut canvas, 8)?;
	    } else {
		self.add_image(ctx, &mut canvas, 6)?;
	    }
	}
	
	if new.is(Input::FIRE) != self.previnput.is(Input::FIRE) {
	    self.add_image(ctx, &mut canvas, 10)?;
	    self.firetimer = Timer::new(time, Self::INTERTIME);
	} else if self.firetimer.done(time) {
	    if new.is(Input::FIRE) {
		self.add_image(ctx, &mut canvas, 11)?;
	    } else {
		self.add_image(ctx, &mut canvas, 9)?;
	    }
	}
	
	if new.is(Input::SECONDARY) != self.previnput.is(Input::SECONDARY) {
	    self.add_image(ctx, &mut canvas, 13)?;
	    self.secondarytimer = Timer::new(time, Self::INTERTIME);
	} else if self.secondarytimer.done(time) {
	    if new.is(Input::SECONDARY) {
		self.add_image(ctx, &mut canvas, 14)?;
	    } else {
		self.add_image(ctx, &mut canvas, 12)?;
	    }
	}

	self.draw_property(ctx, &mut canvas, native.specs.maxcrew, native.crew, 16.0, graphics::Color::GREEN);
	self.draw_property(ctx, &mut canvas, native.specs.maxbattery, native.battery, 208.0, graphics::Color::RED);
	    
	canvas.finish(ctx)?;
	self.previnput = new;
	Ok(())
    }

    fn add_image(&mut self, ctx: &mut Context, canvas: &mut graphics::Canvas, index: usize) -> GameResult {
	let field = &self.activity.fields[index];
	canvas.draw(
	    &field.image,
	    field.get_drawparam()
		.dest(glam::vec2(16.0, 260.0))
	);
	Ok(())
    }

    pub fn draw_property(&mut self, ctx: &mut Context, canvas: &mut graphics::Canvas, max: u8, value: u8, cornerx: f32, color: graphics::Color) {
	let rows = (max + 1) >> 1;// (truncating)
	let height = ((rows << 3) + 4) as f32;
	let rect = graphics::Rect {
	    x: 0.0,
	    y: 0.0,
	    w: 28.0,
	    h: height,
	};
	let mesh = graphics::Mesh::new_rectangle(
	    ctx,
	    graphics::DrawMode::Fill(Default::default()),
	    rect,
	    graphics::Color::BLACK,
	).unwrap();
	canvas.draw(
	    &mesh,
	    graphics::DrawParam::default()
		.dest(glam::vec2(cornerx, 220.0 - height))
	);

	for value in 0..value {
	    let x = cornerx + 16.0 - (value & 1) as f32 * 12.0;
	    let y = 212.0 - (value >> 1) as f32 * 8.0;
	    let rect = graphics::Rect {
		x: 0.0,
		y: 0.0,
		w: 8.0,
		h: 4.0,
	    };
	    let mesh = graphics::Mesh::new_rectangle(
		ctx,
		graphics::DrawMode::Fill(Default::default()),
		rect,
		color,
	    ).unwrap();
	    canvas.draw(
		&mesh,
		graphics::DrawParam::default()
		    .dest(glam::vec2(x, y))
	    );
	}
    }
}
