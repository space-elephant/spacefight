use ggez::{Context, ContextBuilder, GameResult};
use ggez::event::{self, EventHandler};
use ggez::{conf, graphics, glam};
use ggez::input::mouse;
use ggez::input::keyboard::KeyCode;
use std::num::NonZeroU8;
mod stats;
mod ship;
use ship::units;
use std::path::PathBuf;
use std::io::{BufRead, BufReader};
use rand::{RngCore, SeedableRng};
#[macro_use]
extern crate dimensioned as dim;
use dim::Dimensionless;

fn screensize() -> (f32, f32) {
    (1920.0, 1080.0)
}

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
    captains: Vec<graphics::Image>,
    stars: Starfield,
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
	let time = std::time::Instant::now();
	let window = ctx.gfx.window();
	if let Some(monitor) = window.current_monitor() {
	    window.set_inner_size(monitor.size());
	}

	let camera = Camera::new(&self.ships);

	
        let mut canvas = graphics::Canvas::from_frame(ctx, graphics::Color::BLACK);
	
	self.stars.draw(ctx, &mut canvas, camera);
	
	for index in (0..self.ships.len()).rev() {
	    let (before, notbefore) = self.ships.split_at_mut(index);
	    let (main, after) = notbefore.split_at_mut(1);
	    main[0].draw(ctx, &mut canvas, camera, time, before.iter().chain(after.iter()))?;
	}

	static POSITIONS: [(f32, f32); 2] = [(1664.0, 0.0), (1664.0, 600.0)];
	for (captain, (x, y)) in self.captains.iter().zip(&POSITIONS) {
	    canvas.draw(
		captain,
		graphics::DrawParam::default()
		    .dest(glam::vec2(*x, *y))
	    );
	}
	
        canvas.finish(ctx)
    }
}

impl MainState {    
    pub fn new(ctx: &mut Context) -> MainState {
	let window = ctx.gfx.window();
	let monitor = window.current_monitor();
	window.set_fullscreen(Some(ggez::winit::window::Fullscreen::Borderless(monitor)));
	mouse::set_cursor_hidden(ctx, true);

	let time = std::time::Instant::now();
	let (cruiser, cruisercaptain) = ship::specs::Cruiser::gen(
	    ctx,
	    ((-860.0 * units::TSU, -440.0 * units::TSU), 0.0), time,
	    NonZeroU8::new(1).unwrap(),
	    ship::UserControl.into(),
	);
	let (avenger, avengercaptain) = ship::specs::Avenger::gen(
	    ctx,
	    ((860.0 * units::TSU, 440.0 * units::TSU), 0.0), time,
	    NonZeroU8::new(2).unwrap(),
	    ship::NoControl.into(),
	);
        MainState {
	    ships: vec![
		ship::gen_planet(
		    ctx,
		    ((0.0 * units::TSU, 0.0 * units::TSU), 0.0), time,
		),
		cruiser.with_camera(true),
		avenger.with_camera(true),
	    ],
	    captains: vec![cruisercaptain, avengercaptain],
	    stars: Starfield {stars: Animation::new(ctx, "/scenery/stars.ani")},
        }
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

	let screenwidth: f32 = 1920.0 - 288.0 * 2.0;// TODO: determine programatically
	let screenheight: f32 = 1080.0f32;

	let scalex = screenwidth / (right - left);
	let scaley = screenheight / (bottom - top);
	let scale = if scalex < scaley {scalex} else {scaley};

	let x = (left + right) * 0.5 * scale - 288.0f32;
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

#[derive(Debug, Clone)]
struct Animation<const N: usize> {
    fields: [Image; N],
}

impl<const N: usize> Animation<N> {
    fn new(ctx: &mut Context, src: impl Into<PathBuf>) -> Animation<N> {
	let mut src = src.into();
	let spec = ctx.fs.open(&src).expect("missing animation file");
	let reader = BufReader::new(spec);
	let mut fields = Vec::new();
	for line in reader.lines() {
	    fields.push(Image::from_line(ctx, &mut src, &line.expect("valid_animation_file")));
	}
	Animation {
	    fields: fields.try_into().expect("wrong length of animation"),
	}
    }
}

/*#[derive(Debug, Clone)]
struct AnimationVar {
    fields: Box<[Image]>,
}

impl AnimationVar {
    fn new(ctx: &mut Context, src: impl Into<PathBuf>) -> AnimationVar {
	let mut src = src.into();
	let spec = ctx.fs.open(&src).expect("missing animation file");
	let reader = BufReader::new(spec);
	let mut fields = Vec::new();
	for line in reader.lines() {
	    fields.push(Image::from_line(ctx, &mut src, &line.expect("valid_animation_file")));
	}
	AnimationVar {
	    fields: fields.into_boxed_slice(),
	}
    }
}*/

#[derive(Debug, Clone)]
struct Image {
    image: graphics::Image,
    offsetx: f32,
    offsety: f32,
}

impl Image {
    // will not affect any except last of src
    fn from_line(ctx: &mut Context, src: &mut PathBuf, line: &str) -> Image {
	let mut elements = line.split_whitespace();
	let filename = elements.next().ok_or(line).expect("missing image");
	src.set_file_name(filename);
	let image = graphics::Image::from_path(ctx, &src).expect("missing image");
	
	// always 0 and 1 respectively
	elements.next();
	elements.next();

	let absolutex: f32 = elements.next().ok_or(line).expect("missing element").parse().expect("invalid value");
	let absolutey: f32 = elements.next().ok_or(line).expect("missing element").parse().expect("invalid value");
	let offsetx = absolutex / image.width() as f32;
	let offsety = absolutey / image.height() as f32;

	Image {
	    image,
	    offsetx,
	    offsety,
	}
    }

    fn get_drawparam(&self) -> graphics::DrawParam {
	graphics::DrawParam::default()
	    .offset(glam::vec2(self.offsetx, self.offsety))
    }
}
    
struct Starfield {
    stars: Animation<3>,
}

impl Starfield {
    fn draw(&self, ctx: &mut Context, canvas: &mut graphics::Canvas, camera: Camera) {
	Self::draw_plane(&self.stars.fields[2].image, 0.5625, 0x300, ctx, canvas, camera);
	Self::draw_plane(&self.stars.fields[1].image, 0.75, 0x200, ctx, canvas, camera);
	Self::draw_plane(&self.stars.fields[0].image, 1.0, 0x100, ctx, canvas, camera);
    }
    fn draw_plane(image: &graphics::Image, paralax: f32, frequency: u32, ctx: &mut Context, canvas: &mut graphics::Canvas, camera: Camera) {
	let (screenwidth, screenheight) = screensize();
	
	let left = camera.left * paralax - screenwidth * 0.5 * (1.0 - paralax);
	let top = camera.top * paralax - screenheight * 0.5 * (1.0 - paralax);
	const FACTOR: f32 = 0.00390625;// 1/256
	let scale = *(camera.scale * units::TSU).value();
	let invscale = 1.0 / scale;
	
	let minx = (left * invscale * FACTOR).floor() as i32;
	let maxx = ((left + screenwidth) * invscale * FACTOR).ceil() as i32;
	let miny = (top * invscale * FACTOR).floor() as i32;
	let maxy = ((top + screenheight) * invscale * FACTOR).ceil() as i32;

	for y in miny..maxy {
	    for x in minx..maxx {
		let seed = (y as u32 as u64) << 32 | (x as u32 ^ frequency) as u64;
		let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(seed);
		
		/*{
		    let mut dest = [0; 3];
		    rng.fill_bytes(&mut dest);
		    let rect = graphics::Rect {
			x: (x << 8) as f32 * scale - left,
			y: (y << 8) as f32 * scale - top,
			w: 256.0 * scale,
			h: 256.0 * scale,
		    };
		    let mesh = graphics::Mesh::new_rectangle(
			ctx,
			graphics::DrawMode::Fill(Default::default()),
			rect,
			graphics::Color::from_rgb(dest[0], dest[1], dest[2]),
		    ).unwrap();
		    canvas.draw(&mesh, graphics::DrawParam::new());
		}*/

		let mut params = rng.next_u32();
		while params & 0x700 <= frequency {
		    let subx = (params & 255) as i32;
		    let suby = (params >> 24) as i32;
		    canvas.draw(
			image,
			graphics::DrawParam::default()
			    .offset(glam::vec2(0.5, 0.5))
			    .dest(glam::vec2(
				(x << 8 | subx) as f32 * scale - left,
				(y << 8 | suby) as f32 * scale - top
			    ))
			    .scale(glam::vec2(scale, scale))
		    );
		    params = rng.next_u32();
		}
	    }
	}
    }
}
