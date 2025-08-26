use std::collections::VecDeque;
use std::io::{self, Write};

#[derive(Debug, Clone, Copy, PartialEq)]
enum CardSuit {
    Hearts,
    Diamonds,
    Clubs,
    Spades,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum CardRank {
    Ace,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Ten,
    Jack,
    Queen,
    King,
}

#[derive(Debug, Clone)]
struct Card {
    suit: CardSuit,
    rank: CardRank,
    is_face_up: bool,
}

impl Card {
    fn new(suit: CardSuit, rank: CardRank) -> Self {
        Card {
            suit,
            rank,
            is_face_up: false,
        }
    }

    fn symbol(&self) -> &'static str {
        if !self.is_face_up {
            return "🂠";
        }
        match (self.suit, self.rank) {
            (CardSuit::Hearts, CardRank::Ace) => "🂱",
            (CardSuit::Hearts, CardRank::Two) => "🂲",
            (CardSuit::Hearts, CardRank::Three) => "🂳",
            (CardSuit::Hearts, CardRank::Four) => "🂴",
            (CardSuit::Hearts, CardRank::Five) => "🂵",
            (CardSuit::Hearts, CardRank::Six) => "🂶",
            (CardSuit::Hearts, CardRank::Seven) => "🂷",
            (CardSuit::Hearts, CardRank::Eight) => "🂸",
            (CardSuit::Hearts, CardRank::Nine) => "🂹",
            (CardSuit::Hearts, CardRank::Ten) => "🂺",
            (CardSuit::Hearts, CardRank::Jack) => "🂻",
            (CardSuit::Hearts, CardRank::Queen) => "🂼",
            (CardSuit::Hearts, CardRank::King) => "🂽",
            (CardSuit::Diamonds, CardRank::Ace) => "🃁",
            (CardSuit::Diamonds, CardRank::Two) => "🃂",
            (CardSuit::Diamonds, CardRank::Three) => "🃃",
            (CardSuit::Diamonds, CardRank::Four) => "🃄",
            (CardSuit::Diamonds, CardRank::Five) => "🃅",
            (CardSuit::Diamonds, CardRank::Six) => "🃆",
            (CardSuit::Diamonds, CardRank::Seven) => "🃇",
            (CardSuit::Diamonds, CardRank::Eight) => "🃈",
            (CardSuit::Diamonds, CardRank::Nine) => "🃉",
            (CardSuit::Diamonds, CardRank::Ten) => "🃊",
            (CardSuit::Diamonds, CardRank::Jack) => "🃋",
            (CardSuit::Diamonds, CardRank::Queen) => "🃍",
            (CardSuit::Diamonds, CardRank::King) => "🃎",
            (CardSuit::Clubs, CardRank::Ace) => "🃑",
            (CardSuit::Clubs, CardRank::Two) => "🃒",
            (CardSuit::Clubs, CardRank::Three) => "🃓",
            (CardSuit::Clubs, CardRank::Four) => "🃔",
            (CardSuit::Clubs, CardRank::Five) => "🃕",
            (CardSuit::Clubs, CardRank::Six) => "🃖",
            (CardSuit::Clubs, CardRank::Seven) => "🃗",
            (CardSuit::Clubs, CardRank::Eight) => "🃘",
            (CardSuit::Clubs, CardRank::Nine) => "🃙",
            (CardSuit::Clubs, CardRank::Ten) => "🃚",
            (CardSuit::Clubs, CardRank::Jack) => "🃛",
            (CardSuit::Clubs, CardRank::Queen) => "🃝",
            (CardSuit::Clubs, CardRank::King) => "🃞",
            (CardSuit::Spades, CardRank::Ace) => "🂡",
            (CardSuit::Spades, CardRank::Two) => "🂢",
            (CardSuit::Spades, CardRank::Three) => "🂣",
            (CardSuit::Spades, CardRank::Four) => "🂤",
            (CardSuit::Spades, CardRank::Five) => "🂥",
            (CardSuit::Spades, CardRank::Six) => "🂦",
            (CardSuit::Spades, CardRank::Seven) => "🂧",
            (CardSuit::Spades, CardRank::Eight) => "🂨",
            (CardSuit::Spades, CardRank::Nine) => "🂩",
            (CardSuit::Spades, CardRank::Ten) => "🂪",
            (CardSuit::Spades, CardRank::Jack) => "🂫",
            (CardSuit::Spades, CardRank::Queen) => "🂭",
            (CardSuit::Spades, CardRank::King) => "🂮",
        }
    }

    fn is_red(&self) -> bool {
        matches!(self.suit, CardSuit::Hearts | CardSuit::Diamonds)
    }

    fn is_black(&self) -> bool {
        matches!(self.suit, CardSuit::Clubs | CardSuit::Spades)
    }

    fn can_place_on(&self, other: &Card) -> bool {
        if !other.is_face_up {
            return false;
        }
        if self.is_red() && other.is_red() {
            return false;
        }
        if self.is_black() && other.is_black() {
            return false;
        }
        match (self.rank, other.rank) {
            (CardRank::King, CardRank::Ace) => true,
            (CardRank::Queen, CardRank::Two) => true,
            (CardRank::Jack, CardRank::Three) => true,
            (CardRank::Ten, CardRank::Four) => true,
            (CardRank::Nine, CardRank::Five) => true,
            (CardRank::Eight, CardRank::Six) => true,
            (CardRank::Seven, CardRank::Seven) => true,
            (CardRank::Six, CardRank::Eight) => true,
            (CardRank::Five, CardRank::Nine) => true,
            (CardRank::Four, CardRank::Ten) => true,
            (CardRank::Three, CardRank::Jack) => true,
            (CardRank::Two, CardRank::Queen) => true,
            (CardRank::Ace, CardRank::King) => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum CellType {
    Empty,
    Mine,
    Flagged,
    Revealed,
}

#[derive(Debug, Clone)]
struct GameCell {
    cell_type: CellType,
    adjacent_mines: u8,
    card: Option<Card>,
}

impl GameCell {
    fn new() -> Self {
        GameCell {
            cell_type: CellType::Empty,
            adjacent_mines: 0,
            card: None,
        }
    }

    fn is_mine(&self) -> bool {
        matches!(self.cell_type, CellType::Mine)
    }

    fn is_revealed(&self) -> bool {
        matches!(self.cell_type, CellType::Revealed)
    }

    fn is_flagged(&self) -> bool {
        matches!(self.cell_type, CellType::Flagged)
    }
}

struct MineSweeperSolitaire {
    grid: Vec<Vec<GameCell>>,
    width: usize,
    height: usize,
    mine_count: usize,
    game_over: bool,
    won: bool,
    deck: Vec<Card>,
    foundation: Vec<Vec<Card>>,
}

impl MineSweeperSolitaire {
    fn new(width: usize, height: usize, mine_count: usize) -> Self {
        let mut game = MineSweeperSolitaire {
            grid: vec![vec![GameCell::new(); width]; height],
            width,
            height,
            mine_count,
            game_over: false,
            won: false,
            deck: Vec::new(),
            foundation: vec![Vec::new(); 4],
        };
        game.initialize_deck();
        game.place_mines();
        game.calculate_adjacent_mines();
        game.deal_cards();
        game
    }

    fn initialize_deck(&mut self) {
        let suits = [
            CardSuit::Hearts,
            CardSuit::Diamonds,
            CardSuit::Clubs,
            CardSuit::Spades,
        ];
        let ranks = [
            CardRank::Ace,
            CardRank::Two,
            CardRank::Three,
            CardRank::Four,
            CardRank::Five,
            CardRank::Six,
            CardRank::Seven,
            CardRank::Eight,
            CardRank::Nine,
            CardRank::Ten,
            CardRank::Jack,
            CardRank::Queen,
            CardRank::King,
        ];

        for &suit in &suits {
            for &rank in &ranks {
                self.deck.push(Card::new(suit, rank));
            }
        }
        self.shuffle_deck();
    }

    fn shuffle_deck(&mut self) {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        for i in (1..self.deck.len()).rev() {
            let j = rng.gen_range(0..=i);
            self.deck.swap(i, j);
        }
    }

    fn place_mines(&mut self) {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let mut mines_placed = 0;

        while mines_placed < self.mine_count {
            let x = rng.gen_range(0..self.width);
            let y = rng.gen_range(0..self.height);

            if !self.grid[y][x].is_mine() {
                self.grid[y][x].cell_type = CellType::Mine;
                mines_placed += 1;
            }
        }
    }

    fn calculate_adjacent_mines(&mut self) {
        for y in 0..self.height {
            for x in 0..self.width {
                if self.grid[y][x].is_mine() {
                    continue;
                }

                let mut count = 0;
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        if dx == 0 && dy == 0 {
                            continue;
                        }
                        let nx = x as i32 + dx;
                        let ny = y as i32 + dy;
                        if nx >= 0
                            && nx < self.width as i32
                            && ny >= 0
                            && ny < self.height as i32
                        {
                            if self.grid[ny as usize][nx as usize].is_mine() {
                                count += 1;
                            }
                        }
                    }
                }
                self.grid[y][x].adjacent_mines = count;
            }
        }
    }

    fn deal_cards(&mut self) {
        let mut card_index = 0;
        for y in 0..self.height {
            for x in 0..self.width {
                if !self.grid[y][x].is_mine() && card_index < self.deck.len() {
                    self.grid[y][x].card = Some(self.deck[card_index].clone());
                    card_index += 1;
                }
            }
        }
    }

    fn reveal_cell(&mut self, x: usize, y: usize) -> bool {
        if self.game_over || x >= self.width || y >= self.height {
            return false;
        }

        let cell = &mut self.grid[y][x];
        if cell.is_revealed() || cell.is_flagged() {
            return false;
        }

        if cell.is_mine() {
            cell.cell_type = CellType::Revealed;
            self.game_over = true;
            return false;
        }

        cell.cell_type = CellType::Revealed;
        cell.card.as_mut().map(|card| card.is_face_up = true);

        // Auto-reveal adjacent cells if this cell has no adjacent mines
        if cell.adjacent_mines == 0 {
            self.reveal_adjacent_cells(x, y);
        }

        self.check_win_condition();
        true
    }

    fn reveal_adjacent_cells(&mut self, x: usize, y: usize) {
        let mut queue = VecDeque::new();
        queue.push_back((x, y));

        while let Some((cx, cy)) = queue.pop_front() {
            for dy in -1..=1 {
                for dx in -1..=1 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let nx = cx as i32 + dx;
                    let ny = cy as i32 + dy;
                    if nx >= 0
                        && nx < self.width as i32
                        && ny >= 0
                        && ny < self.height as i32
                    {
                        let nx = nx as usize;
                        let ny = ny as usize;
                        let cell = &mut self.grid[ny][nx];
                        if !cell.is_revealed() && !cell.is_flagged() && !cell.is_mine() {
                            cell.cell_type = CellType::Revealed;
                            cell.card.as_mut().map(|card| card.is_face_up = true);
                            if cell.adjacent_mines == 0 {
                                queue.push_back((nx, ny));
                            }
                        }
                    }
                }
            }
        }
    }

    fn toggle_flag(&mut self, x: usize, y: usize) {
        if self.game_over || x >= self.width || y >= self.height {
            return;
        }

        let cell = &mut self.grid[y][x];
        if cell.is_revealed() {
            return;
        }

        cell.cell_type = match cell.cell_type {
            CellType::Empty => CellType::Flagged,
            CellType::Flagged => CellType::Empty,
            _ => cell.cell_type,
        };
    }

    fn move_card_to_foundation(&mut self, x: usize, y: usize) -> bool {
        if self.game_over || x >= self.width || y >= self.height {
            return false;
        }

        let cell = &mut self.grid[y][x];
        if !cell.is_revealed() || cell.card.is_none() {
            return false;
        }

        let card = cell.card.as_ref().unwrap();
        let suit_index = match card.suit {
            CardSuit::Hearts => 0,
            CardSuit::Diamonds => 1,
            CardSuit::Clubs => 2,
            CardSuit::Spades => 3,
        };

        let foundation = &mut self.foundation[suit_index];
        let can_place = if foundation.is_empty() {
            matches!(card.rank, CardRank::Ace)
        } else {
            let top_card = foundation.last().unwrap();
            match (top_card.rank, card.rank) {
                (CardRank::Ace, CardRank::Two) => true,
                (CardRank::Two, CardRank::Three) => true,
                (CardRank::Three, CardRank::Four) => true,
                (CardRank::Four, CardRank::Five) => true,
                (CardRank::Five, CardRank::Six) => true,
                (CardRank::Six, CardRank::Seven) => true,
                (CardRank::Seven, CardRank::Eight) => true,
                (CardRank::Eight, CardRank::Nine) => true,
                (CardRank::Nine, CardRank::Ten) => true,
                (CardRank::Ten, CardRank::Jack) => true,
                (CardRank::Jack, CardRank::Queen) => true,
                (CardRank::Queen, CardRank::King) => true,
                _ => false,
            }
        };

        if can_place {
            foundation.push(cell.card.take().unwrap());
            cell.cell_type = CellType::Empty;
            self.check_win_condition();
            return true;
        }

        false
    }

    fn move_card_to_cell(&mut self, from_x: usize, from_y: usize, to_x: usize, to_y: usize) -> bool {
        if self.game_over 
            || from_x >= self.width || from_y >= self.height
            || to_x >= self.width || to_y >= self.height {
            return false;
        }

        // Check if from cell has a card and is revealed
        if !self.grid[from_y][from_x].is_revealed() || self.grid[from_y][from_x].card.is_none() {
            return false;
        }

        // Check if to cell is valid for placement
        if self.grid[to_y][to_x].is_revealed() || self.grid[to_y][to_x].is_flagged() || self.grid[to_y][to_x].is_mine() {
            return false;
        }

        // Check card placement rules
        if let Some(to_card) = &self.grid[to_y][to_x].card {
            let from_card = self.grid[from_y][from_x].card.as_ref().unwrap();
            if !from_card.can_place_on(to_card) {
                return false;
            }
        }

        if from_y == to_y {
            // Same row - use split_at_mut to avoid borrowing issues
            let row = &mut self.grid[from_y];
            let (left, right) = row.split_at_mut(to_x.max(from_x));
            let (from_cell, to_cell) = if from_x < to_x {
                (&mut left[from_x], &mut right[0])
            } else {
                (&mut right[from_x - to_x], &mut left[to_x])
            };
            
            // Perform the move
            let card = from_cell.card.take().unwrap();
            to_cell.card = Some(card);
            to_cell.cell_type = CellType::Revealed;
            to_cell.card.as_mut().map(|card| card.is_face_up = true);
            
            from_cell.cell_type = CellType::Empty;
        } else {
            // Different rows - need to handle borrowing carefully by using indices
            // Perform the move
            let card = self.grid[from_y][from_x].card.take().unwrap();
            self.grid[to_y][to_x].card = Some(card);
            self.grid[to_y][to_x].cell_type = CellType::Revealed;
            self.grid[to_y][to_x].card.as_mut().map(|card| card.is_face_up = true);
            
            self.grid[from_y][from_x].cell_type = CellType::Empty;
        }

        self.check_win_condition();
        true
    }

    fn check_win_condition(&mut self) {

        let foundation_complete = self.foundation.iter().all(|pile| {
            pile.len() == 13 // All 13 cards in sequence
        });

        // Check if all non-mine cells are revealed or all foundation sequences are complete
        let all_revealed = self.grid.iter().enumerate().all(|(_y, row)| {
            row.iter().enumerate().all(|(_x, cell)| {
                if cell.is_mine() {
                    true // Mines don't need to be revealed
                } else if let Some(card) = &cell.card {
                    cell.is_revealed() || matches!(card.rank, CardRank::King)
                } else {
                    true // Empty cells are fine
                }
            })
        });

        if all_revealed || foundation_complete {
            self.won = true;
            self.game_over = true;
        }
    }

    fn display(&self) {
        println!("\n=== MineSweeper Solitaire ===");
        println!("Mines: {} | Game Over: {} | Won: {}", self.mine_count, self.game_over, self.won);
        println!();

        // Display column headers
        print!("  ");
        for x in 0..self.width {
            print!(" {} ", x);
        }
        println!();

        // Display grid
        for y in 0..self.height {
            print!("{} ", y);
            for x in 0..self.width {
                let cell = &self.grid[y][x];
                if cell.is_flagged() {
                    print!(" 🚩");
                } else if !cell.is_revealed() {
                    print!(" ■ ");
                } else if cell.is_mine() {
                    print!(" 💣");
                } else if let Some(card) = &cell.card {
                    print!(" {} ", card.symbol());
                } else {
                    print!("   ");
                }
            }
            println!();
        }

        println!();
        println!("Foundations:");
        for (i, foundation) in self.foundation.iter().enumerate() {
            print!("{}: ", i);
            if foundation.is_empty() {
                print!("[empty]");
            } else {
                let top_card = foundation.last().unwrap();
                print!("{}", top_card.symbol());
                if foundation.len() > 1 {
                    print!(" (+{})", foundation.len() - 1);
                }
            }
            println!();
        }
    }
}

fn get_user_input() -> Option<(usize, usize, String)> {
    print!("Enter command (r x y = reveal, f x y = flag, m x y = move to foundation, c fx fy tx ty = move card between cells, q = quit): ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    let input = input.trim();

    if input == "q" || input == "quit" {
        return None;
    }

    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.is_empty() {
        return Some((0, 0, "invalid".to_string()));
    }

    match parts[0] {
        "r" | "reveal" if parts.len() == 3 => {
            let x = parts[1].parse().unwrap_or(0);
            let y = parts[2].parse().unwrap_or(0);
            Some((x, y, "reveal".to_string()))
        }
        "f" | "flag" if parts.len() == 3 => {
            let x = parts[1].parse().unwrap_or(0);
            let y = parts[2].parse().unwrap_or(0);
            Some((x, y, "flag".to_string()))
        }
        "m" | "move" if parts.len() == 3 => {
            let x = parts[1].parse().unwrap_or(0);
            let y = parts[2].parse().unwrap_or(0);
            Some((x, y, "move".to_string()))
        }
        "c" | "cell" if parts.len() == 5 => {
            let fx = parts[1].parse().unwrap_or(0);
            let fy = parts[2].parse().unwrap_or(0);
            let tx = parts[3].parse().unwrap_or(0);
            let ty = parts[4].parse().unwrap_or(0);
            Some((fx, fy, format!("cell {} {}", tx, ty)))
        }
        _ => Some((0, 0, "invalid".to_string())),
    }
}

fn main() {
    println!("Welcome to MineSweeper Solitaire!");
    println!("This game combines Minesweeper grid mechanics with Solitaire card gameplay.");
    println!("Rules:");
    println!("- Reveal cells to find playing cards");
    println!("- Avoid mines (💣) - they end the game!");
    println!("- Move cards to foundations in sequence (A, 2, 3, ..., K) by suit");
    println!("- Move cards between cells following Solitaire rules (alternating colors, descending rank pairs)");
    println!("- Flag suspected mines with 'f x y'");
    println!("- Move cards to foundation with 'm x y'");
    println!("- Move cards between cells with 'c from_x from_y to_x to_y'");
    println!();

    let mut game = MineSweeperSolitaire::new(8, 8, 10);
    
    loop {
        game.display();
        
        if game.game_over {
            if game.won {
                println!("🎉 Congratulations! You won the game!");
            } else {
                println!("💥 Game Over! You hit a mine!");
            }
            break;
        }

        match get_user_input() {
            None => {
                println!("Thanks for playing!");
                break;
            }
            Some((x, y, command)) => {
                match command.as_str() {
                    "reveal" => {
                        if !game.reveal_cell(x, y) {
                            println!("Invalid move or mine hit!");
                        }
                    }
                    "flag" => {
                        game.toggle_flag(x, y);
                    }
                    "move" => {
                        if game.move_card_to_foundation(x, y) {
                            println!("Card moved to foundation!");
                        } else {
                            println!("Invalid move to foundation!");
                        }
                    }
                    cmd if cmd.starts_with("cell") => {
                        let parts: Vec<&str> = cmd.split_whitespace().collect();
                        if parts.len() == 3 {
                            let to_x = parts[1].parse().unwrap_or(0);
                            let to_y = parts[2].parse().unwrap_or(0);
                            if game.move_card_to_cell(x, y, to_x, to_y) {
                                println!("Card moved between cells!");
                            } else {
                                println!("Invalid card move between cells!");
                            }
                        }
                    }
                    "invalid" => {
                        println!("Invalid command! Please try again.");
                    }
                    _ => {
                        println!("Unknown command! Please try again.");
                    }
                }
            }
        }
    }
}
