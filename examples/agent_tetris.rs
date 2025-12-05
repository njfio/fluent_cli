```rust
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent},
    execute, queue,
    style::{self, Color, Print, SetForegroundColor},
    terminal::{self, ClearType},
};
use std::collections::HashMap;
use std::io::{self, Write};
use std::time::{Duration, Instant};

const GRID_WIDTH: usize = 10;
const GRID_HEIGHT: usize = 20;
const PREVIEW_SIZE: usize = 4;

#[derive(Clone, Copy, PartialEq, Debug)]
enum TetrominoType {
    I, O, T, S, Z, J, L,
}

#[derive(Clone, Copy, PartialEq)]
struct Cell {
    filled: bool,
    color: Color,
}

impl Default for Cell {
    fn default() -> Self {
        Cell {
            filled: false,
            color: Color::White,
        }
    }
}

#[derive(Clone)]
struct Tetromino {
    shape: Vec<Vec<bool>>,
    color: Color,
    tetromino_type: TetrominoType,
}

struct Piece {
    tetromino: Tetromino,
    x: i32,
    y: i32,
}

struct Game {
    grid: [[Cell; GRID_WIDTH]; GRID_HEIGHT],
    current_piece: Option<Piece>,
    next_piece: Tetromino,
    held_piece: Option<Tetromino>,
    can_hold: bool,
    score: u32,
    level: u32,
    lines_cleared: u32,
    last_fall: Instant,
    fall_speed: Duration,
}

impl Tetromino {
    fn new(tetromino_type: TetrominoType) -> Self {
        let (shape, color) = match tetromino_type {
            TetrominoType::I => (vec![
                vec![false, false, false, false],
                vec![true, true, true, true],
                vec![false, false, false, false],
                vec![false, false, false, false],
            ], Color::Cyan),
            TetrominoType::O => (vec![
                vec![true, true],
                vec![true, true],
            ], Color::Yellow),
            TetrominoType::T => (vec![
                vec![false, true, false],
                vec![true, true, true],
                vec![false, false, false],
            ], Color::Magenta),
            TetrominoType::S => (vec![
                vec![false, true, true],
                vec![true, true, false],
                vec![false, false, false],
            ], Color::Green),
            TetrominoType::Z => (vec![
                vec![true, true, false],
                vec![false, true, true],
                vec![false, false, false],
            ], Color::Red),
            TetrominoType::J => (vec![
                vec![true, false, false],
                vec![true, true, true],
                vec![false, false, false],
            ], Color::Blue),
            TetrominoType::L => (vec![
                vec![false, false, true],
                vec![true, true, true],
                vec![false, false, false],
            ], Color::DarkYellow),
        };
        
        Tetromino {
            shape,
            color,
            tetromino_type,
        }
    }

    fn rotate(&self) -> Self {
        let size = self.shape.len();
        let mut new_shape = vec![vec![false; size]; size];
        
        for i in 0..size {
            for j in 0..size {
                new_shape[j][size - 1 - i] = self.shape[i][j];
            }
        }
        
        Tetromino {
            shape: new_shape,
            color: self.color,
            tetromino_type: self.tetromino_type,
        }
    }
}

impl Game {
    fn new() -> Self {
        let mut game = Game {
            grid: [[Cell::default(); GRID_WIDTH]; GRID_HEIGHT],
            current_piece: None,
            next_piece: Self::random_tetromino(),
            held_piece: None,
            can_hold: true,
            score: 0,
            level: 1,
            lines_cleared: 0,
            last_fall: Instant::now(),
            fall_speed: Duration::from_millis(1000),
        };
        game.spawn_piece();
        game
    }

    fn random_tetromino() -> Tetromino {
        let types = [
            TetrominoType::I, TetrominoType::O, TetrominoType::T, TetrominoType::S,
            TetrominoType::Z, TetrominoType::J, TetrominoType::L,
        ];
        let index = (std::ptr::addr_of!(types) as usize / 8) % types.len();
        let index = (Instant::now().elapsed().as_nanos() as usize) % types.len();
        Tetromino::new(types[index])
    }

    fn spawn_piece(&mut self) {
        let tetromino = self.next_piece.clone();
        self.next_piece = Self::random_tetromino();
        
        let piece = Piece {
            tetromino,
            x: (GRID_WIDTH as i32 - 4) / 2,
            y: 0,
        };
        
        if self.is_valid_position(&piece) {
            self.current_piece = Some(piece);
            self.can_hold = true;
        } else {
            // Game over
            self.current_piece = None;
        }
    }

    fn is_valid_position(&self, piece: &Piece) -> bool {
        for (i, row) in piece.tetromino.shape.iter().enumerate() {
            for (j, &cell) in row.iter().enumerate() {
                if cell {
                    let x = piece.x + j as i32;
                    let y = piece.y + i as i32;
                    
                    if x < 0 || x >= GRID_WIDTH as i32 || y >= GRID_HEIGHT as i32 {
                        return false;
                    }
                    
                    if y >= 0 && self.grid[y as usize][x as usize].filled {
                        return false;
                    }
                }
            }
        }
        true
    }

    fn move_piece(&mut self, dx: i32, dy: i32) -> bool {
        if let Some(ref mut piece) = self.current_piece {
            let new_piece = Piece {
                tetromino: piece.tetromino.clone(),
                x: piece.x + dx,
                y: piece.y + dy,
            };
            
            if self.is_valid_position(&new_piece) {
                *piece = new_piece;
                return true;
            }
        }
        false
    }

    fn rotate_piece(&mut self) {
        if let Some(ref mut piece) = self.current_piece {
            let rotated_tetromino = piece.tetromino.rotate();
            let new_piece = Piece {
                tetromino: rotated_tetromino,
                x: piece.x,
                y: piece.y,
            };
            
            if self.is_valid_position(&new_piece) {
                piece.tetromino = new_piece.tetromino;
            }
        }
    }

    fn hard_drop(&mut self) {
        while self.move_piece(0, 1) {}
        self.lock_piece();
    }

    fn hold_piece(&mut self) {
        if !self.can_hold {
            return;
        }
        
        if let Some(current) = self.current_piece.take() {
            match self.held_piece.take() {
                Some(held) => {
                    self.held_piece = Some(current.tetromino);
                    self.current_piece = Some(Piece {
                        tetromino: held,
                        x: (GRID_WIDTH as i32 - 4) / 2,
                        y: 0,
                    });
                }
                None => {
                    self.held_piece = Some(current.tetromino);
                    self.spawn_piece();
                }
            }
            self.can_hold = false;
        }
    }

    fn lock_piece(&mut self) {
        if let Some(piece) = &self.current_piece {
            for (i, row) in piece.tetromino.shape.iter().enumerate() {
                for (j, &cell) in row.iter().enumerate() {
                    if cell {
                        let x = piece.x + j as i32;
                        let y = piece.y + i as i32;
                        
                        if y >= 0 && y < GRID_HEIGHT as i32 && x >= 0 && x < GRID_WIDTH as i32 {
                            self.grid[y as usize][x as usize] = Cell {
                                filled: true,
                                color: piece.tetromino.color,
                            };
                        }
                    }
                }
            }
        }
        
        self.current_piece = None;
        self.clear_lines();
        self.spawn_piece();
    }

    fn clear_lines(&mut self) {
        let mut lines_to_clear = Vec::new();
        
        for y in 0..GRID_HEIGHT {
            if self.grid[y].iter().all(|cell| cell.filled) {
                lines_to_clear.push(y);
            }
        }
        
        for &y in lines_to_clear.iter().rev() {
            for row in (1..=y).rev() {
                self.grid[row] = self.grid[row - 1];
            }
            self.grid[0] = [Cell::default(); GRID_WIDTH];
        }
        
        let lines_cleared = lines_to_clear.len() as u32;
        self.lines_cleared += lines_cleared;
        
        // Scoring
        let line_score = match lines_cleared {
            1 => 100,
            2 => 300,
            3 => 500,
            4 => 800,
            _ => 0,
        };
        self.score += line_score * self.level;
        
        // Level progression
        self.level = (self.lines_cleared / 10) + 1;
        self.fall_speed = Duration::from_millis(std::cmp::max(50, 1000 - (self.level - 1) * 50) as u64);
    }

    fn update(&mut self) {
        if self.last_fall.elapsed() >= self.fall_speed {
            if !self.move_piece(0, 1) {
                self.lock_piece();
            }
            self.last_fall = Instant::now();
        }
    }

    fn is_game_over(&self) -> bool {
        self.current_piece.is_none() && 
        self.grid[0].iter().any(|cell| cell.filled)
    }

    fn render(&self) -> io::Result<()> {
        let mut stdout = io::stdout();
        
        queue!(stdout, cursor::MoveTo(0, 0))?;
        
        // Render game area
        for y in 0..GRID_HEIGHT {
            queue!(stdout, Print("|"))?;
            
            for x in 0..GRID_WIDTH {
                let mut cell = self.grid[y][x];
                
                // Check if current piece occupies this position
                if let Some(ref piece) = self.current_piece {
                    for (i, row) in piece.tetromino.shape.iter().enumerate() {
                        for (j, &shape_cell) in row.iter().enumerate() {
                            if shape_cell {
                                let px = piece.x + j as i32;
                                let py = piece.y + i as i32;
                                
                                if px == x as i32 && py == y as i32 {
                                    cell = Cell {
                                        filled: true,
                                        color: piece.tetromino.color,
                                    };
                                }
                            }
                        }
                    }
                }
                
                if cell.filled {
                    queue!(stdout, SetForegroundColor(cell.color), Print("█"), SetForegroundColor(Color::White))?;
                } else {
                    queue!(stdout, Print(" "))?;
                }
            }
            
            queue!(stdout, Print("|"))?;
            
            // Side panel info
            match y {
                1 => queue!(stdout, Print(&format!("  Score: {}", self.score)))?,
                2 => queue!(stdout, Print(&format!("  Level: {}", self.level)))?,
                3 => queue!(stdout, Print(&format!("  Lines: {}", self.lines_cleared)))?,
                5 => queue!(stdout, Print("  Next:"))?,
                6..=9 => {
                    queue!(stdout, Print("  "))?;
                    let row = y - 6;
                    if row < self.next_piece.shape.len() {
                        for &cell in &self.next_piece.shape[row] {
                            if cell {
                                queue!(stdout, SetForegroundColor(self.next_piece.color), Print("█"), SetForegroundColor(Color::White))?;
                            } else {
                                queue!(stdout, Print(" "))?;
                            }
                        }
                    }
                }
                11 => queue!(stdout, Print("  Hold:"))?,
                12..=15 => {
                    queue!(stdout, Print("  "))?;
                    if let Some(ref held) = self.held_piece {
                        let row = y - 12;
                        if row < held.shape.len() {
                            for &cell in &held.shape[row] {
                                if cell {
                                    queue!(stdout, SetForegroundColor(held.color), Print("█"), SetForegroundColor(Color::White))?;
                                } else {
                                    queue!(stdout, Print(" "))?;
                                }
                            }
                        }
                    }
                }
                17 => queue!(stdout, Print("  Controls:"))?,
                18 => queue!(stdout, Print("  ←→↓ Move, ↑ Rotate"))?,
                19 => queue!(stdout, Print("  Space: Drop, C: Hold"))?,
                _ => {}
            }
            
            queue!(stdout, Print("\n"))?;
        }
        
        // Bottom border
        queue!(stdout, Print("+"))?;
        for _ in 0..GRID_WIDTH {
            queue!(stdout, Print("-"))?;
        }
        queue!(stdout, Print("+\n"))?;
        
        stdout.flush()?;
        Ok(())
    }
}

fn main() -> io::Result<()> {
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, terminal::Clear(ClearType::All), cursor::Hide)?;
    
    let mut game = Game::new();
    
    loop {
        game.update();
        game.render()?;
        
        if game.is_game_over() {
            queue!(stdout, cursor::MoveTo(0, GRID_HEIGHT as u16 + 2), Print("Game Over! Press any key to exit..."))?;
            stdout.flush()?;
            event::read()?;
            break;
        }
        
        if event::poll(Duration::from_millis(16