use ggez::{Context, ContextBuilder, GameResult};
use ggez::event::{self, EventHandler};
use ggez::{conf, graphics};
use ggez::input::mouse;
use ggez::input::keyboard::KeyCode;
use std::num::NonZeroU8;
mod ship;

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
		    ((960.0.into(), 540.0.into()), 0.0), time,
		    NonZeroU8::new(1),
		),
		ship::specs::Cruiser::gen(
		    ctx,
		    ((100.0.into(), 100.0.into()), 0.0), time,
		    None,
		),
	    ],
        }
    }
}

impl EventHandler for MainState {
    fn update(&mut self, ctx: &mut Context) -> GameResult {
	let time = std::time::Instant::now();

	for index in 1..self.ships.len() {
	    let (first, second) = self.ships.split_at_mut(index);
	    let dest = &mut second[0];
	    for source in first {
		source.gravitate(ctx, dest);
	    }
	}

	let mut index = self.ships.len();
	while index > 0 {
	    index -= 1;
	    if let Some(mut summon) = self.ships[index].update(ctx, time)? {
		self.ships.append(&mut summon);
	    } else {
		self.ships.remove(index);
	    }
	}
        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
	let window = ctx.gfx.window();
	if let Some(monitor) = window.current_monitor() {
	    window.set_inner_size(monitor.size());
	}
	
        let mut canvas = graphics::Canvas::from_frame(ctx, graphics::Color::BLACK);
	for ship in self.ships.iter_mut().rev() {
	    ship.draw(ctx, &mut canvas)?;
	}
        canvas.finish(ctx)
    }
}
