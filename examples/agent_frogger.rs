use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{poll, read, Event, KeyCode},
    execute,
    style::{Color, Print, SetForegroundColor},
    terminal::{Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
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
            cars: CAR_ROWS.iter().map(|_| vec![5]).collect(),
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
            let direction = if i % 2 == 0 { 1 } else { -1 };
            for car in row.iter_mut() {
                *car = (*car as i32 + direction).rem_euclid(WIDTH as i32) as u16;
            }

            // Spawn new cars randomly
            if rand::random::<f32>() < 0.02 {
                row.push(if direction > 0 { 0 } else { WIDTH - 1 });
            }
        }

        // Check collisions
        for (i, row) in self.cars.iter().enumerate() {
            if self.frog_y == CAR_ROWS[i] {
                for &car in row {
                    if (self.frog_x as i32 - car as i32).abs() < 2 {
                        self.lives -= 1;
                        if self.lives == 0 {
                            self.game_over = true;
                        }
                        self.reset_frog();
                        return;
                    }
                }
            }
        }

        // Check if frog reached goal
        if self.frog_y == GOAL_ROW {
            self.score += 100;
            self.reset_frog();
        }
    }

    fn draw(&self) -> Result<()> {
        execute!(stdout(), Clear(ClearType::All))?;

        // Draw border
        for y in 0..HEIGHT {
            execute!(
                stdout(),
                MoveTo(0, y),
                SetForegroundColor(Color::White),
                Print("|"),
                MoveTo(WIDTH, y),
                Print("|")
            )?;
        }

        // Draw goal
        execute!(
            stdout(),
            MoveTo(0, GOAL_ROW),
            SetForegroundColor(Color::Green),
            Print("G".repeat(WIDTH as usize + 1))
        )?;

        // Draw cars
        for (i, row) in self.cars.iter().enumerate() {
            for &car in row {
                execute!(
                    stdout(),
                    MoveTo(car, CAR_ROWS[i]),
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

        // Draw score and lives
        execute!(
            stdout(),
            MoveTo(2, HEIGHT + 1),
            SetForegroundColor(Color::White),
            Print(format!("Score: {} Lives: {}", self.score, self.lives))
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
    execute!(stdout(), EnterAlternateScreen, Hide)?;

    let mut game = Game::new();
    let frame_duration = Duration::from_millis(50);

    loop {
        let frame_start = Instant::now();

        if !game.game_over {
            game.update();
        }

        game.draw()?;

        if poll(Duration::from_millis(0))? {
            match read()? {
                Event::Key(event) => match event.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Char('w') if !game.game_over && game.frog_y > 1 => game.frog_y -= 1,
                    KeyCode::Char('s') if !game.game_over && game.frog_y < HEIGHT - 1 => {
                        game.frog_y += 1
                    }
                    KeyCode::Char('a') if !game.game_over && game.frog_x > 1 => game.frog_x -= 1,
                    KeyCode::Char('d') if !game.game_over && game.frog_x < WIDTH - 1 => {
                        game.frog_x += 1
                    }
                    KeyCode::Char('r') if game.game_over => game = Game::new(),
                    _ => {}
                },
                _ => {}
            }
        }

        let elapsed = frame_start.elapsed();
        if elapsed < frame_duration {
            std::thread::sleep(frame_duration - elapsed);
        }
    }

    execute!(stdout(), Show, LeaveAlternateScreen)?;
    Ok(())
}