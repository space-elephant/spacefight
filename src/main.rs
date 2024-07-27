use ggez::{Context, ContextBuilder, GameResult};
use ggez::event::{self, EventHandler};
use ggez::{conf, graphics};
use ggez::input::mouse;
use ggez::input::keyboard::KeyCode;
use std::num::NonZeroU8;
mod ship;
use ship::units;
#[macro_use]
extern crate dimensioned as dim;
use dim::Dimensionless;

fn main() {
    let (mut ctx, event_loop) = ContextBuilder::new("spacefight", "Russell VA3BSP <rmorland@tutanota.com>")
	.window_setup(conf::WindowSetup {
	    title: "spacefight".to_owned(),
	    samples: conf::NumSamples::One,
	    vsync: true,
	    icon: "".to_owned(),
	    srgb: true,
	})
	.window_mode(
	    conf::WindowMode::default()
		.dimensions(1920.0, 1080.0)
	)
	.add_resource_path("./resources")
        .build()
        .expect("could not create ggez context");

    let my_game = MainState::new(&mut ctx);

    event::run(ctx, event_loop, my_game);
}

struct MainState {
    ships: Vec<ship::Actor>,
}

impl MainState {
    pub fn new(ctx: &mut Context) -> MainState {
	let window = ctx.gfx.window();
	let monitor = window.current_monitor();
	window.set_fullscreen(Some(ggez::winit::window::Fullscreen::Borderless(monitor)));
	mouse::set_cursor_hidden(ctx, true);

	let time = std::time::Instant::now();
        MainState {
	    ships: vec![
		ship::gen_planet(
		    ctx,
		    ((960.0 * units::TSU, 540.0 * units::TSU), 0.0), time,
		),
		ship::specs::Cruiser::gen(
		    ctx,
		    ((100.0 * units::TSU, 100.0 * units::TSU), 0.0), time,
		    NonZeroU8::new(1).unwrap(),
		    ship::UserControl.into(),
		).with_camera(true),
		ship::specs::Avenger::gen(
		    ctx,
		    ((1820.0 * units::TSU, 980.0 * units::TSU), 0.0), time,
		    NonZeroU8::new(2).unwrap(),
		    ship::NoControl.into(),
		).with_camera(true),
	    ],
        }
    }
}

impl EventHandler for MainState {
    fn update(&mut self, ctx: &mut Context) -> GameResult {
	let time = std::time::Instant::now();

	let mut extra = Vec::new();
	for index in 0..self.ships.len() {
	    let (before, notbefore) = self.ships.split_at_mut(index);
	    let (main, after) = notbefore.split_at_mut(1);
	    let mut summon = main[0].update(ctx, time, before.iter().chain(after.iter()))?;
	    extra.append(&mut summon);
	}
	self.ships.append(&mut extra);

	let mut index = self.ships.len();
	while index > 0 {
	    index -= 1;
	    if self.ships[index].dead() {
		self.ships.remove(index);
	    }
	}

	for index in 1..self.ships.len() {
	    let (left, right) = self.ships.split_at_mut(index);
	    let dest = &mut right[0];
	    for source in left {
		source.interact(ctx, dest);
	    }
	}

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
	let window = ctx.gfx.window();
	if let Some(monitor) = window.current_monitor() {
	    window.set_inner_size(monitor.size());
	}

	let camera = Camera::new(&self.ships);
	
        let mut canvas = graphics::Canvas::from_frame(ctx, graphics::Color::BLACK);
	for ship in self.ships.iter_mut().rev() {
	    ship.draw(ctx, &mut canvas, camera)?;
	}
        canvas.finish(ctx)
    }
}

#[derive(Debug, Clone, Copy)]
struct Camera {
    left: f32,
    top: f32,
    scale: units::TrueSpaceUnitInv<f32>,
}

impl Camera {
    fn new(ships: &[ship::Actor]) -> Self {
	const MARGIN: units::TrueSpaceUnit<f32> = units::TrueSpaceUnit::new(360.0);

	let mut left = f32::INFINITY * units::TSU;
	let mut top = f32::INFINITY * units::TSU;
	let mut right = -f32::INFINITY * units::TSU;
	let mut bottom = -f32::INFINITY * units::TSU;

	for ship in ships {
	    if ship.has_camera() {
		let (x, y) = ship.get_pos();
		if x < left {
		    left = x;
		}
		if x > right {
		    right = x;
		}
		if y < top {
		    top = y;
		}
		if y > bottom {
		    bottom = y;
		}
	    }
	}

	if left == f32::INFINITY * units::TSU {
	    // no ships of signifigance found, make default
	    return Camera {
		left: 0.0,
		top: 0.0,
		scale: units::TSUI,
	    }
	}

	left -= MARGIN;
	top -= MARGIN;
	right += MARGIN;
	bottom += MARGIN;

	let screenwidth = 1920.0f32;// TODO: determine programatically
	let screenheight = 1080.0f32;

	let scalex = screenwidth / (right - left);
	let scaley = screenheight / (bottom - top);
	let scale = if scalex < scaley {scalex} else {scaley};

	let x = (left + right) * 0.5 * scale;
	let y = (top + bottom) * 0.5 * scale;

	let screenleft = *(x - screenwidth * 0.5).value();
	let screentop = *(y - screenheight * 0.5).value();

	Camera {
	    left: screenleft,
	    top: screentop,
	    scale,
	}
    }
}
