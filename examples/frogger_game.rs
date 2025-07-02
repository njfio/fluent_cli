// Frogger-like Game in Rust - Terminal Based
// This demonstrates what the agentic system would create

use std::io::{self, stdout, Write};
use std::time::{Duration, Instant};
use std::thread;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{self, ClearType},
};

const GAME_WIDTH: usize = 40;
const GAME_HEIGHT: usize = 20;
const GOAL_ROW: usize = 1;
const START_ROW: usize = GAME_HEIGHT - 2;

#[derive(Clone, Copy, PartialEq)]
struct Position {
    x: usize,
    y: usize,
}

#[derive(Clone)]
struct Car {
    pos: Position,
    direction: i32, // 1 for right, -1 for left
    speed: u64,     // milliseconds between moves
    last_move: Instant,
}

struct Game {
    frog: Position,
    cars: Vec<Car>,
    score: u32,
    lives: u32,
    game_over: bool,
    won: bool,
    last_update: Instant,
}

impl Game {
    fn new() -> Self {
        let mut cars = Vec::new();
        
        // Create cars on different rows with different speeds and directions
        for row in 3..GAME_HEIGHT-3 {
            if row % 2 == 0 {
                // Cars moving right
                for i in 0..3 {
                    cars.push(Car {
                        pos: Position { x: i * 15, y: row },
                        direction: 1,
                        speed: 200 + (row as u64 * 50),
                        last_move: Instant::now(),
                    });
                }
            } else {
                // Cars moving left
                for i in 0..3 {
                    cars.push(Car {
                        pos: Position { x: GAME_WIDTH - 1 - (i * 15), y: row },
                        direction: -1,
                        speed: 150 + (row as u64 * 30),
                        last_move: Instant::now(),
                    });
                }
            }
        }

        Game {
            frog: Position { x: GAME_WIDTH / 2, y: START_ROW },
            cars,
            score: 0,
            lives: 3,
            game_over: false,
            won: false,
            last_update: Instant::now(),
        }
    }

    fn update(&mut self) {
        let now = Instant::now();
        
        // Update car positions
        for car in &mut self.cars {
            if now.duration_since(car.last_move).as_millis() >= car.speed as u128 {
                car.pos.x = ((car.pos.x as i32 + car.direction) as usize) % GAME_WIDTH;
                car.last_move = now;
            }
        }

        // Check collision with cars
        for car in &self.cars {
            if self.frog.x == car.pos.x && self.frog.y == car.pos.y {
                self.lives -= 1;
                if self.lives == 0 {
                    self.game_over = true;
                } else {
                    // Reset frog position
                    self.frog = Position { x: GAME_WIDTH / 2, y: START_ROW };
                }
                break;
            }
        }

        // Check if frog reached the goal
        if self.frog.y == GOAL_ROW {
            self.score += 100;
            self.won = true;
        }

        self.last_update = now;
    }

    fn move_frog(&mut self, dx: i32, dy: i32) {
        let new_x = (self.frog.x as i32 + dx).max(0).min(GAME_WIDTH as i32 - 1) as usize;
        let new_y = (self.frog.y as i32 + dy).max(1).min(GAME_HEIGHT as i32 - 2) as usize;
        
        self.frog.x = new_x;
        self.frog.y = new_y;
    }

    fn draw(&self) -> io::Result<()> {
        execute!(stdout(), terminal::Clear(ClearType::All), cursor::MoveTo(0, 0))?;

        // Draw game border and field
        for y in 0..GAME_HEIGHT {
            for x in 0..GAME_WIDTH {
                let mut char_to_draw = ' ';
                let mut color = Color::White;

                // Draw borders
                if y == 0 || y == GAME_HEIGHT - 1 || x == 0 || x == GAME_WIDTH - 1 {
                    char_to_draw = '#';
                    color = Color::Yellow;
                }
                // Draw goal area
                else if y == GOAL_ROW {
                    char_to_draw = '=';
                    color = Color::Green;
                }
                // Draw start area
                else if y == START_ROW {
                    char_to_draw = '-';
                    color = Color::Blue;
                }
                // Draw road
                else if y > 2 && y < GAME_HEIGHT - 3 {
                    char_to_draw = '.';
                    color = Color::DarkGrey;
                }

                execute!(stdout(), SetForegroundColor(color), Print(char_to_draw))?;
            }
            println!();
        }

        // Draw cars
        for car in &self.cars {
            execute!(
                stdout(),
                cursor::MoveTo(car.pos.x as u16, car.pos.y as u16),
                SetForegroundColor(Color::Red),
                Print('█')
            )?;
        }

        // Draw frog
        execute!(
            stdout(),
            cursor::MoveTo(self.frog.x as u16, self.frog.y as u16),
            SetForegroundColor(Color::Green),
            Print('🐸')
        )?;

        // Draw UI
        execute!(
            stdout(),
            cursor::MoveTo(0, GAME_HEIGHT as u16 + 1),
            SetForegroundColor(Color::White),
            Print(format!("Score: {} | Lives: {} | Use WASD to move, Q to quit", self.score, self.lives))
        )?;

        if self.game_over {
            execute!(
                stdout(),
                cursor::MoveTo(GAME_WIDTH as u16 / 2 - 5, GAME_HEIGHT as u16 / 2),
                SetForegroundColor(Color::Red),
                Print("GAME OVER!")
            )?;
        }

        if self.won {
            execute!(
                stdout(),
                cursor::MoveTo(GAME_WIDTH as u16 / 2 - 4, GAME_HEIGHT as u16 / 2),
                SetForegroundColor(Color::Green),
                Print("YOU WIN!")
            )?;
        }

        execute!(stdout(), ResetColor)?;
        stdout().flush()?;
        Ok(())
    }
}

fn main() -> io::Result<()> {
    println!("🎮 Frogger-like Game - Created by Agentic System Demo");
    println!("Press any key to start...");
    
    // Wait for user input to start
    let _ = io::stdin().read_line(&mut String::new());

    // Setup terminal
    terminal::enable_raw_mode()?;
    execute!(stdout(), terminal::Clear(ClearType::All))?;

    let mut game = Game::new();
    let mut last_frame = Instant::now();
    let frame_duration = Duration::from_millis(50);

    loop {
        // Handle input
        if event::poll(Duration::from_millis(1))? {
            if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                match code {
                    KeyCode::Char('q') | KeyCode::Char('Q') => break,
                    KeyCode::Char('w') | KeyCode::Char('W') | KeyCode::Up => {
                        if !game.game_over && !game.won {
                            game.move_frog(0, -1);
                        }
                    }
                    KeyCode::Char('s') | KeyCode::Char('S') | KeyCode::Down => {
                        if !game.game_over && !game.won {
                            game.move_frog(0, 1);
                        }
                    }
                    KeyCode::Char('a') | KeyCode::Char('A') | KeyCode::Left => {
                        if !game.game_over && !game.won {
                            game.move_frog(-1, 0);
                        }
                    }
                    KeyCode::Char('d') | KeyCode::Char('D') | KeyCode::Right => {
                        if !game.game_over && !game.won {
                            game.move_frog(1, 0);
                        }
                    }
                    KeyCode::Char('r') | KeyCode::Char('R') => {
                        if game.game_over || game.won {
                            game = Game::new();
                        }
                    }
                    _ => {}
                }
            }
        }

        // Update game logic
        if !game.game_over && !game.won {
            game.update();
        }

        // Render at consistent frame rate
        if last_frame.elapsed() >= frame_duration {
            game.draw()?;
            last_frame = Instant::now();
        }

        thread::sleep(Duration::from_millis(10));
    }

    // Cleanup
    terminal::disable_raw_mode()?;
    execute!(stdout(), terminal::Clear(ClearType::All), cursor::MoveTo(0, 0))?;
    println!("Thanks for playing! Game created by the Agentic System.");

    Ok(())
}
