use crossterm::{
    cursor::{Hide, Show},
    event::{poll, read, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    Result,
};
use rand::Rng;
use std::{io::stdout, time::Duration};

const WIDTH: usize = 40;
const HEIGHT: usize = 20;
const GOAL_Y: usize = 1;
const FROG_START_Y: usize = HEIGHT - 2;

struct Game {
    board: [[char; WIDTH]; HEIGHT],
    frog_x: usize,
    frog_y: usize,
    cars: Vec<Car>,
    score: u32,
    lives: u32,
}

struct Car {
    x: usize,
    y: usize,
    direction: i32,
}

impl Game {
    fn new() -> Self {
        let mut game = Game {
            board: [[' '; WIDTH]; HEIGHT],
            frog_x: WIDTH / 2,
            frog_y: FROG_START_Y,
            cars: Vec::new(),
            score: 0,
            lives: 3,
        };
        game.init_board();
        game.spawn_cars();
        game
    }

    fn init_board(&mut self) {
        for x in 0..WIDTH {
            self.board[GOAL_Y][x] = '=';
        }
    }

    fn spawn_cars(&mut self) {
        let mut rng = rand::thread_rng();
        for y in (GOAL_Y + 2..FROG_START_Y).step_by(2) {
            let direction = if rng.gen() { 1 } else { -1 };
            let car = Car {
                x: rng.gen_range(0..WIDTH),
                y,
                direction,
            };
            self.cars.push(car);
        }
    }

    fn update(&mut self) {
        self.move_cars();
        self.check_collision();
    }

    fn move_cars(&mut self) {
        for car in &mut self.cars {
            car.x = (car.x as i32 + car.direction).rem_euclid(WIDTH as i32) as usize;
        }
    }

    fn check_collision(&mut self) {
        if self.cars.iter().any(|car| car.x == self.frog_x && car.y == self.frog_y) {
            self.lives -= 1;
            self.reset_frog();
        }
    }

    fn reset_frog(&mut self) {
        self.frog_x = WIDTH / 2;
        self.frog_y = FROG_START_Y;
    }

    fn move_frog(&mut self, dx: i32, dy: i32) {
        let new_x = (self.frog_x as i32 + dx).clamp(0, (WIDTH - 1) as i32) as usize;
        let new_y = (self.frog_y as i32 + dy).clamp(0, (HEIGHT - 1) as i32) as usize;
        self.frog_x = new_x;
        self.frog_y = new_y;

        if self.frog_y == GOAL_Y {
            self.score += 1;
            self.reset_frog();
        }
    }

    fn draw(&self) {
        let mut board = self.board.clone();
        for car in &self.cars {
            board[car.y][car.x] = '█';
        }
        board[self.frog_y][self.frog_x] = '🐸';

        for row in &board {
            println!("{}", row.iter().collect::<String>());
        }
        println!("Score: {} | Lives: {}", self.score, self.lives);
    }

    fn is_game_over(&self) -> bool {
        self.lives == 0
    }
}

fn main() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, Hide)?;

    let mut game = Game::new();

    loop {
        if game.is_game_over() {
            break;
        }

        game.draw();
        game.update();

        if poll(Duration::from_millis(100))? {
            if let Event::Key(key_event) = read()? {
                match key_event.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('w') => game.move_frog(0, -1),
                    KeyCode::Char('s') => game.move_frog(0, 1),
                    KeyCode::Char('a') => game.move_frog(-1, 0),
                    KeyCode::Char('d') => game.move_frog(1, 0),
                    _ => {}
                }
            }
        }

        execute!(stdout, crossterm::terminal::Clear(crossterm::terminal::ClearType::All))?;
        execute!(stdout, crossterm::cursor::MoveTo(0, 0))?;
    }

    execute!(stdout, Show, LeaveAlternateScreen)?;
    disable_raw_mode()?;

    println!("Game Over! Final Score: {}", game.score);

    Ok(())
}