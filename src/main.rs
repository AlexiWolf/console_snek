use std::collections::VecDeque;
use std::process::exit;

use console_engine::pixel::Pixel;
use console_engine::*;
use log::*;
use rand::prelude::*;
use wolf_engine::*;

const BOARD_WIDTH: usize = 80;
const BOARD_HEIGHT: usize = 20;

fn main() {
    logging::initialize_logging(LevelFilter::Info);

    let (width, height) = term_size::dimensions().expect("could not determine terminal size");

    if BOARD_WIDTH > width || BOARD_HEIGHT > height {
        error!(
            "Your screen is too small, it must be at least {} x {} characters.",
            BOARD_WIDTH, BOARD_HEIGHT
        );
        exit(1)
    }

    let mut context = Context::new();
    context
        .add(ConsoleContext::new(BOARD_WIDTH as u32, BOARD_HEIGHT as u32, 10))
        .expect("failed to add ConsoleContext");

    EngineBuilder::new()
        .with_scheduler(Box::from(SimpleScheduler))
        .build(context)
        .run(Box::from(GameState::new()));
}

struct GameState {
    rng: ThreadRng,
    player: Snake,
    score: u32,
    food: Food,
}

impl State for GameState {
    fn setup(&mut self, _context: &mut Context) {
        self.move_food();
        self.player.velocity.x = 0;
        self.player.velocity.y = 0;
    }

    fn update(&mut self, context: &mut Context) -> OptionalTransition {
        let console = get_console(context);
        console.wait_for_frame();

        if self.player.location == self.food.location {
            self.score += 1;
            self.player.grow();
            self.move_food();
        }

        for body_segment in self.player.body.iter() {
            if self.player.location == body_segment.location {
                return Some(Transition::Push(Box::from(LoseState::new(self.score))));
            }
        }

        if console.is_key_pressed(KeyCode::Up) {
            self.player.velocity.y = -1;
            self.player.velocity.x = 0;
        }
        if console.is_key_pressed(KeyCode::Down) {
            self.player.velocity.y = 1;
            self.player.velocity.x = 0;
        }
        if console.is_key_pressed(KeyCode::Left) {
            self.player.velocity.x = -1;
            self.player.velocity.y = 0;
        }
        if console.is_key_pressed(KeyCode::Right) {
            self.player.velocity.x = 1;
            self.player.velocity.y = 0;
        }
        if console.is_key_pressed(KeyCode::Char('q')) {
            return Some(Transition::Quit);
        }
        if console.is_key_pressed(KeyCode::Char('g')) {
            self.player.grow();
        }

        self.player.update();

        None
    }

    fn render(&mut self, context: &mut Context) -> RenderResult {
        let console = get_console(context);

        console.fill(pixel::pxl_fg('.', Color::DarkGrey));
        console.print(0, 0, format!("Score: {}", self.score).as_str());
        self.player.draw(console); self.food.draw(console);
        console.draw();
    }
}

impl GameState {
    pub fn new() -> Self {
        Self {
            rng: thread_rng(),
            player: Snake::new(0, 1),
            score: 0,
            food: Food::new(0, 0),
        }
    }

    fn move_food(&mut self) {
        self.food.location = self.get_random_location();
    }

    fn get_random_location(&mut self) -> Vector2 {
        let x = self.rng.gen_range(1..BOARD_WIDTH);
        let y = self.rng.gen_range(1..BOARD_HEIGHT);
        Vector2::new(x as i32, y as i32)
    }
}

pub struct LoseState {
    score: u32,
}

impl State for LoseState {
    fn update(&mut self, context: &mut Context) -> OptionalTransition {
        let console = get_console(context);

        if console.is_key_pressed(KeyCode::Char('y')) {
            return Some(Transition::CleanPush(Box::from(GameState::new())));
        }
        if console.is_key_pressed(KeyCode::Char('n')) || console.is_key_pressed(KeyCode::Char('q')) {
            return Some(Transition::Quit);
        }

        None
    }

    fn render(&mut self, context: &mut Context) -> RenderResult {
        let console = get_console(context);
        console.wait_for_frame();
        console.print(
            0,
            0,
            format!("You died!  You got {} points!", self.score).as_str(),
        );
        console.print(0, 1, "Play again? (y / n)");
        console.draw();
    }
}

impl LoseState {
    pub fn new(score: u32) -> Self {
        Self { score }
    }
}

fn get_console(context: &mut Context) -> &mut ConsoleContext {
    context
        .get_mut::<ConsoleContext>()
        .expect("no ConsoleContext")
}

pub struct Snake {
    pub location: Vector2,
    pub previous_location: Option<Vector2>,
    pub velocity: Vector2,
    pub body: VecDeque<BodySegment>,
}

impl Snake {
    pub fn new(x: i32, y: i32) -> Self {
        Self {
            location: Vector2::new(x, y),
            previous_location: None,
            velocity: Vector2::new(0, 0),
            body: VecDeque::new(),
        }
    }

    pub fn update(&mut self) {
        if self.velocity.x != 0 || self.velocity.y != 0 {
            self.previous_location = Some(self.location.clone());
        }
        self.location.add(self.velocity);
        if self.location.x > BOARD_WIDTH as i32 {
            self.location.x = 0;
        }
        if self.location.x < 0 {
            self.location.x = BOARD_WIDTH as i32;
        }
        if self.location.y > BOARD_HEIGHT as i32 {
            self.location.y = 0;
        }
        if self.location.y < 0 {
            self.location.y = BOARD_HEIGHT as i32;
        }
        if let Some(mut segment) = self.body.pop_back() {
            let previous_location = self.previous_location.clone().unwrap();
            segment.location.x = previous_location.x;
            segment.location.y = previous_location.y;
            self.body.push_front(segment);
        }
    }

    pub fn draw(&mut self, console: &mut ConsoleContext) {
        console.set_pixel(
            self.location.x,
            self.location.y,
            pixel::pxl_fg('@', Color::DarkGreen),
        );
        self.body
            .iter()
            .for_each(|body_segment| body_segment.draw(console));
    }

    pub fn grow(&mut self) {
        if let Some(previous_location) = self.previous_location {
            self.body
                .push_front(BodySegment::new(previous_location.x, previous_location.y));
        }
    }
}

pub struct BodySegment {
    pub location: Vector2,
}

impl BodySegment {
    pub fn new(x: i32, y: i32) -> Self {
        Self {
            location: Vector2::new(x, y),
        }
    }

    pub fn draw(&self, console: &mut ConsoleContext) {
        console.set_pixel(
            self.location.x,
            self.location.y,
            pixel::pxl_fg('#', Color::Green),
        );
    }
}

pub struct Food {
    location: Vector2,
}

impl Food {
    pub fn new(x: i32, y: i32) -> Self {
        Self {
            location: Vector2::new(x, y),
        }
    }

    pub fn draw(&self, console: &mut ConsoleContext) {
        console.set_pixel(
            self.location.x,
            self.location.y,
            pixel::pxl_fg('*', Color::Red),
        );
    }
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub struct Vector2 {
    pub x: i32,
    pub y: i32,
}

impl Vector2 {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    pub fn add(&mut self, vector: Vector2) {
        self.x += vector.x;
        self.y += vector.y;
    }
}

pub struct ConsoleContext {
    pub console: ConsoleEngine,
}

impl ConsoleContext {
    pub fn new(width: u32, height: u32, target_fps: u32) -> Self {
        Self {
            console: Self::initialize_console_engine(width, height, target_fps),
        }
    }

    fn initialize_console_engine(width: u32, height: u32, target_fps: u32) -> ConsoleEngine {
        ConsoleEngine::init(width, height, target_fps).expect("Failed to initialize the console")
    }

    pub fn wait_for_frame(&mut self) {
        self.console.wait_frame();
    }

    pub fn clear_screen(&mut self) {
        self.console.clear_screen();
    }

    pub fn set_pixel(&mut self, x: i32, y: i32, character: Pixel) {
        self.console.set_pxl(x, y, character);
    }

    pub fn draw(&mut self) {
        self.console.draw();
    }

    pub fn is_key_pressed(&self, key: KeyCode) -> bool {
        self.console.is_key_pressed(key)
    }

    pub fn fill(&mut self, pixel: Pixel) {
        self.console.fill(pixel);
    }

    pub fn print(&mut self, x: i32, y: i32, string: &str) {
        self.console.print(x, y, string);
    }
}

impl Subcontext for ConsoleContext {}

pub struct SimpleScheduler;

impl Scheduler for SimpleScheduler {
    fn update(&mut self, context: &mut Context, state: &mut dyn State) {
        state.update(context);
    }

    fn render(&mut self, context: &mut Context, state: &mut dyn State) {
        state.render(context);
    }
}
