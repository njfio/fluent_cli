use std::io::{self, Write};

#[derive(Clone, Copy, PartialEq)]
enum Cell {
    Empty,
    X,
    O,
}

impl Cell {
    fn to_char(&self) -> char {
        match self {
            Cell::Empty => ' ',
            Cell::X => 'X',
            Cell::O => 'O',
        }
    }
}

struct Board {
    cells: [[Cell; 3]; 3],
}

impl Board {
    fn new() -> Self {
        Board {
            cells: [[Cell::Empty; 3]; 3],
        }
    }

    fn display(&self) {
        println!("\n  1   2   3");
        for (i, row) in self.cells.iter().enumerate() {
            print!("{} ", i + 1);
            for (j, cell) in row.iter().enumerate() {
                print!("{}", cell.to_char());
                if j < 2 {
                    print!(" | ");
                }
            }
            println!();
            if i < 2 {
                println!("  -----------");
            }
        }
        println!();
    }

    fn place(&mut self, row: usize, col: usize, player: Cell) -> bool {
        if row >= 3 || col >= 3 {
            return false;
        }
        if self.cells[row][col] == Cell::Empty {
            self.cells[row][col] = player;
            true
        } else {
            false
        }
    }

    fn check_winner(&self) -> Option<Cell> {
        // Check rows
        for row in &self.cells {
            if row[0] != Cell::Empty && row[0] == row[1] && row[1] == row[2] {
                return Some(row[0]);
            }
        }

        // Check columns
        for col in 0..3 {
            if self.cells[0][col] != Cell::Empty
                && self.cells[0][col] == self.cells[1][col]
                && self.cells[1][col] == self.cells[2][col]
            {
                return Some(self.cells[0][col]);
            }
        }

        // Check diagonals
        if self.cells[0][0] != Cell::Empty
            && self.cells[0][0] == self.cells[1][1]
            && self.cells[1][1] == self.cells[2][2]
        {
            return Some(self.cells[0][0]);
        }

        if self.cells[0][2] != Cell::Empty
            && self.cells[0][2] == self.cells[1][1]
            && self.cells[1][1] == self.cells[2][0]
        {
            return Some(self.cells[0][2]);
        }

        None
    }

    fn is_full(&self) -> bool {
        self.cells.iter().all(|row| row.iter().all(|&cell| cell != Cell::Empty))
    }
}

fn get_move() -> Result<(usize, usize), String> {
    print!("Enter row and column (e.g., 1 2): ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).map_err(|e| e.to_string())?;

    let parts: Vec<&str> = input.trim().split_whitespace().collect();
    if parts.len() != 2 {
        return Err("Please enter two numbers separated by space".to_string());
    }

    let row = parts[0].parse::<usize>().map_err(|_| "Invalid row number".to_string())?;
    let col = parts[1].parse::<usize>().map_err(|_| "Invalid column number".to_string())?;

    if row < 1 || row > 3 || col < 1 || col > 3 {
        return Err("Numbers must be between 1 and 3".to_string());
    }

    Ok((row - 1, col - 1))
}

fn main() {
    println!("=== Tic-Tac-Toe ===\n");

    let mut board = Board::new();
    let mut current_player = Cell::X;

    loop {
        board.display();

        println!("Player {}'s turn", current_player.to_char());

        match get_move() {
            Ok((row, col)) => {
                if board.place(row, col, current_player) {
                    if let Some(winner) = board.check_winner() {
                        board.display();
                        println!("🎉 Player {} wins!", winner.to_char());
                        break;
                    }

                    if board.is_full() {
                        board.display();
                        println!("It's a draw!");
                        break;
                    }

                    current_player = if current_player == Cell::X { Cell::O } else { Cell::X };
                } else {
                    println!("That cell is already taken!");
                }
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    }

    println!("\nThanks for playing!");
}
