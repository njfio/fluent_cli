use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{poll, read, Event, KeyCode},
    execute,
    style::{Color, Print, SetForegroundColor},
    terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType},
    Result,
};
use std::{
    io::stdout,
    time::{Duration, Instant},
};

const WIDTH: u16 = 40;
const HEIGHT: u16 = 20;
const CAR_ROWS: &[u16] = &[5, 8, 11, 14, 17];
const GOAL_ROW: u16 = 2;

struct Game {
    frog_x: u16,
    frog_y: u16,
    cars: Vec<Vec<u16>>,
    score: u32,
    lives: u32,
    game_over: bool,
}

impl Game {
    fn new() -> Self {
        Game {
            frog_x: WIDTH / 2,
            frog_y: HEIGHT - 2,
            cars: CAR_ROWS.iter().map(|_| Vec::new()).collect(),
            score: 0,
            lives: 3,
            game_over: false,
        }
    }

    fn reset_frog(&mut self) {
        self.frog_x = WIDTH / 2;
        self.frog_y = HEIGHT - 2;
    }

    fn update(&mut self) {
        // Update car positions
        for (i, row) in self.cars.iter_mut().enumerate() {
            // Spawn new cars
            if rand::random::<f32>() < 0.1 {
                row.push(if i % 2 == 0 { 0 } else { WIDTH - 1 });
            }

            // Move cars
            for car in row.iter_mut() {
                if i % 2 == 0 {
                    *car = (*car + 1).min(WIDTH);
                } else {
                    *car = car.saturating_sub(1);
                }
            }

            // Remove cars that are off screen
            row.retain(|&x| x < WIDTH && x > 0);
        }

        // Check collisions
        if self.check_collision() {
            self.lives -= 1;
            if self.lives == 0 {
                self.game_over = true;
            }
            self.reset_frog();
        }

        // Check if frog reached goal
        if self.frog_y == GOAL_ROW {
            self.score += 100;
            self.reset_frog();
        }
    }

    fn check_collision(&self) -> bool {
        for (i, row) in self.cars.iter().enumerate() {
            if self.frog_y == CAR_ROWS[i] {
                for &car_x in row {
                    if (self.frog_x as i32 - car_x as i32).abs() < 2 {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn draw(&self) -> Result<()> {
        execute!(stdout(), Clear(ClearType::All))?;

        // Draw score and lives
        execute!(
            stdout(),
            MoveTo(0, 0),
            SetForegroundColor(Color::White),
            Print(format!("Score: {} Lives: {}", self.score, self.lives))
        )?;

        // Draw goal area
        for x in 0..WIDTH {
            execute!(
                stdout(),
                MoveTo(x, GOAL_ROW),
                SetForegroundColor(Color::Green),
                Print("=")
            )?;
        }

        // Draw cars
        for (i, row) in self.cars.iter().enumerate() {
            for &car_x in row {
                execute!(
                    stdout(),
                    MoveTo(car_x, CAR_ROWS[i]),
                    SetForegroundColor(Color::Red),
                    Print("🚗")
                )?;
            }
        }

        // Draw frog
        execute!(
            stdout(),
            MoveTo(self.frog_x, self.frog_y),
            SetForegroundColor(Color::Green),
            Print("🐸")
        )?;

        if self.game_over {
            execute!(
                stdout(),
                MoveTo(WIDTH / 2 - 5, HEIGHT / 2),
                SetForegroundColor(Color::Red),
                Print("GAME OVER!")
            )?;
        }

        Ok(())
    }
}

fn main() -> Result<()> {
    enable_raw_mode()?;
    execute!(stdout(), Hide)?;

    let mut game = Game::new();
    let mut last_update = Instant::now();

    while !game.game_over {
        // Handle input
        if poll(Duration::from_millis(100))? {
            if let Event::Key(event) = read()? {
                match event.code {
                    KeyCode::Char('w') => game.frog_y = game.frog_y.saturating_sub(1),
                    KeyCode::Char('s') => {
                        if game.frog_y < HEIGHT - 1 {
                            game.frog_y += 1
                        }
                    }
                    KeyCode::Char('a') => game.frog_x = game.frog_x.saturating_sub(1),
                    KeyCode::Char('d') => {
                        if game.frog_x < WIDTH - 1 {
                            game.frog_x += 1
                        }
                    }
                    KeyCode::Char('q') => break,
                    _ => {}
                }
            }
        }

        // Update game state
        if last_update.elapsed() >= Duration::from_millis(100) {
            game.update();
            last_update = Instant::now();
        }

        // Draw game state
        game.draw()?;
    }

    execute!(stdout(), Show)?;
    disable_raw_mode()?;
    Ok(())
}