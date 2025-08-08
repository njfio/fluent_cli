use std::io::{self, Write};
use std::thread;
use std::time::Duration;

const GAME_WIDTH: usize = 20;
const GAME_HEIGHT: usize = 15;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Cell {
    Empty,
    Frog,
    Car,
    Water,
    Log,
    Goal,
}

struct Game {
    board: [[Cell; GAME_WIDTH]; GAME_HEIGHT],
    frog_x: usize,
    frog_y: usize,
    score: u32,
    lives: u32,
    game_over: bool,
}

impl Game {
    fn new() -> Self {
        let mut game = Game {
            board: [[Cell::Empty; GAME_WIDTH]; GAME_HEIGHT],
            frog_x: GAME_WIDTH / 2,
            frog_y: GAME_HEIGHT - 1,
            score: 0,
            lives: 3,
            game_over: false,
        };
        game.initialize_board();
        game
    }

    fn initialize_board(&mut self) {
        // Clear board
        for row in &mut self.board {
            for cell in row {
                *cell = Cell::Empty;
            }
        }

        // Set up goal area (top row)
        for x in 0..GAME_WIDTH {
            self.board[0][x] = Cell::Goal;
        }

        // Set up water areas (rows 1-5)
        for y in 1..6 {
            for x in 0..GAME_WIDTH {
                self.board[y][x] = Cell::Water;
            }
        }

        // Add some logs in water
        for y in 1..6 {
            for x in (2..GAME_WIDTH).step_by(4) {
                if x < GAME_WIDTH {
                    self.board[y][x] = Cell::Log;
                }
                if x + 1 < GAME_WIDTH {
                    self.board[y][x + 1] = Cell::Log;
                }
            }
        }

        // Set up road areas (rows 8-12)
        for y in 8..13 {
            for x in 0..GAME_WIDTH {
                self.board[y][x] = Cell::Empty;
            }
        }

        // Add some cars on roads
        for y in 8..13 {
            for x in (1..GAME_WIDTH).step_by(5) {
                if x < GAME_WIDTH {
                    self.board[y][x] = Cell::Car;
                }
            }
        }

        // Place frog at starting position
        self.board[self.frog_y][self.frog_x] = Cell::Frog;
    }

    fn move_frog(&mut self, dx: i32, dy: i32) -> bool {
        let new_x = (self.frog_x as i32 + dx) as usize;
        let new_y = (self.frog_y as i32 + dy) as usize;

        // Check bounds
        if new_x >= GAME_WIDTH || new_y >= GAME_HEIGHT {
            return false;
        }

        // Clear old position
        self.board[self.frog_y][self.frog_x] = Cell::Empty;

        // Check new position
        match self.board[new_y][new_x] {
            Cell::Car => {
                // Hit by car - lose life
                self.lives -= 1;
                if self.lives == 0 {
                    self.game_over = true;
                }
                // Reset frog position
                self.frog_x = GAME_WIDTH / 2;
                self.frog_y = GAME_HEIGHT - 1;
            }
            Cell::Water => {
                // Fell in water - lose life
                self.lives -= 1;
                if self.lives == 0 {
                    self.game_over = true;
                }
                // Reset frog position
                self.frog_x = GAME_WIDTH / 2;
                self.frog_y = GAME_HEIGHT - 1;
            }
            Cell::Goal => {
                // Reached goal - score points
                self.score += 100;
                // Reset frog position for next round
                self.frog_x = GAME_WIDTH / 2;
                self.frog_y = GAME_HEIGHT - 1;
            }
            _ => {
                // Valid move
                self.frog_x = new_x;
                self.frog_y = new_y;
            }
        }

        // Place frog at new position
        self.board[self.frog_y][self.frog_x] = Cell::Frog;
        true
    }

    fn update(&mut self) {
        // Move cars (simple animation)
        for y in 8..13 {
            let mut new_row = [Cell::Empty; GAME_WIDTH];
            for x in 0..GAME_WIDTH {
                if self.board[y][x] == Cell::Car {
                    let new_x = (x + 1) % GAME_WIDTH;
                    new_row[new_x] = Cell::Car;
                }
            }
            // Update the row, but preserve frog if it's there
            for x in 0..GAME_WIDTH {
                if self.board[y][x] != Cell::Frog {
                    self.board[y][x] = new_row[x];
                }
            }
        }

        // Check if frog is hit by car after movement
        if self.board[self.frog_y][self.frog_x] == Cell::Car {
            self.lives -= 1;
            if self.lives == 0 {
                self.game_over = true;
            }
            // Reset frog position
            self.frog_x = GAME_WIDTH / 2;
            self.frog_y = GAME_HEIGHT - 1;
            self.board[self.frog_y][self.frog_x] = Cell::Frog;
        }
    }

    fn render(&self) {
        // Clear screen
        print!("\x1B[2J\x1B[1;1H");

        println!("🐸 Frogger Game - Score: {} Lives: {}", self.score, self.lives);
        println!("Use WASD to move, Q to quit");
        println!();

        for row in &self.board {
            for &cell in row {
                let symbol = match cell {
                    Cell::Empty => " ",
                    Cell::Frog => "🐸",
                    Cell::Car => "🚗",
                    Cell::Water => "🌊",
                    Cell::Log => "🪵",
                    Cell::Goal => "🏁",
                };
                print!("{symbol}");
            }
            println!();
        }

        if self.game_over {
            println!("\n💀 Game Over! Final Score: {}", self.score);
        }

        let _ = io::stdout().flush(); // Ignore flush errors in game display
    }
}

/// Run the Frogger game
pub fn run_frogger_game() -> io::Result<()> {
    println!("🐸 Frogger Game - Created by Agentic System");
    println!("Use WASD to move, Q to quit");
    println!("Press Enter to start...");

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let mut game = Game::new();

    loop {
        game.render();

        if game.game_over {
            println!("Press Enter to play again or Q to quit...");
            input.clear();
            io::stdin().read_line(&mut input)?;
            if input.trim().to_lowercase() == "q" {
                break;
            }
            game = Game::new();
            continue;
        }

        // Get input (non-blocking would be better, but this is a simple implementation)
        input.clear();
        io::stdin().read_line(&mut input)?;
        let command = input.trim().to_lowercase();

        match command.as_str() {
            "w" => { game.move_frog(0, -1); }
            "s" => { game.move_frog(0, 1); }
            "a" => { game.move_frog(-1, 0); }
            "d" => { game.move_frog(1, 0); }
            "q" => break,
            _ => {}
        }

        game.update();
        thread::sleep(Duration::from_millis(100));
    }

    println!("Thanks for playing! 🐸");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_game_creation() {
        let game = Game::new();
        assert_eq!(game.lives, 3);
        assert_eq!(game.score, 0);
        assert!(!game.game_over);
        assert_eq!(game.frog_x, GAME_WIDTH / 2);
        assert_eq!(game.frog_y, GAME_HEIGHT - 1);
    }

    #[test]
    fn test_frog_movement() {
        let mut game = Game::new();
        let initial_x = game.frog_x;
        let initial_y = game.frog_y;

        // Test valid movement
        game.move_frog(1, 0);
        assert_eq!(game.frog_x, initial_x + 1);
        assert_eq!(game.frog_y, initial_y);

        // Test boundary checking
        game.frog_x = GAME_WIDTH - 1;
        game.move_frog(1, 0); // Should not move beyond boundary
        assert_eq!(game.frog_x, GAME_WIDTH - 1);
    }

    #[test]
    fn test_game_initialization() {
        let game = Game::new();
        
        // Check that goal area is set up
        for x in 0..GAME_WIDTH {
            assert_eq!(game.board[0][x], Cell::Goal);
        }

        // Check that frog is placed correctly
        assert_eq!(game.board[game.frog_y][game.frog_x], Cell::Frog);
    }
}
