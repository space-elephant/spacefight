use ggez::{Context, ContextBuilder, GameResult};
use ggez::event::{self, EventHandler};
use ggez::{conf, graphics};
use ggez::input::mouse;
use ggez::input::keyboard::KeyCode;
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
    /*shipimg: graphics::Image,
    angle: f32,
    x: f32,
    y: f32,*/
}

impl MainState {
    pub fn new(ctx: &mut Context) -> MainState {
	let window = ctx.gfx.window();
	let monitor = window.current_monitor();
	window.set_fullscreen(Some(ggez::winit::window::Fullscreen::Borderless(monitor)));
	mouse::set_cursor_hidden(ctx, true);

	let cruiserimg = graphics::Image::from_path(ctx, "/cruiser.png").expect("image loading");
	
        MainState {
	    ships: vec![ship::Actor::new(cruiserimg, (100.0, 100.0), &ship::specs::CRUISER)],
        }
    }
}

impl EventHandler for MainState {
    fn update(&mut self, ctx: &mut Context) -> GameResult {
	for ship in &mut self.ships {
	    ship.update(ctx)?;
	}
        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
	let window = ctx.gfx.window();
	if let Some(monitor) = window.current_monitor() {
	    window.set_inner_size(monitor.size());
	}
	
        let mut canvas = graphics::Canvas::from_frame(ctx, graphics::Color::BLACK);
	for ship in &mut self.ships {
	    ship.draw(ctx, &mut canvas)?;
	}
        canvas.finish(ctx)
    }
}
