use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{poll, read, Event, KeyCode, KeyEvent},
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{
        disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};
use rand::Rng;
use std::{
    collections::VecDeque,
    io::{self, stdout, Write},
    time::{Duration, Instant},
};
use tokio::time::sleep;

#[derive(Clone, Copy, PartialEq)]
struct Position {
    x: u16,
    y: u16,
}

#[derive(Clone, Copy, PartialEq)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

struct Snake {
    body: VecDeque<Position>,
    direction: Direction,
}

impl Snake {
    fn new(start_pos: Position) -> Self {
        let mut body = VecDeque::new();
        body.push_back(start_pos);
        body.push_back(Position {
            x: start_pos.x - 1,
            y: start_pos.y,
        });
        body.push_back(Position {
            x: start_pos.x - 2,
            y: start_pos.y,
        });

        Snake {
            body,
            direction: Direction::Right,
        }
    }

    fn move_snake(&mut self, grow: bool) {
        let head = *self.body.front().unwrap();
        let new_head = match self.direction {
            Direction::Up => Position {
                x: head.x,
                y: head.y - 1,
            },
            Direction::Down => Position {
                x: head.x,
                y: head.y + 1,
            },
            Direction::Left => Position {
                x: head.x - 1,
                y: head.y,
            },
            Direction::Right => Position {
                x: head.x + 1,
                y: head.y,
            },
        };

        self.body.push_front(new_head);

        if !grow {
            self.body.pop_back();
        }
    }

    fn change_direction(&mut self, new_direction: Direction) {
        match (self.direction, new_direction) {
            (Direction::Up, Direction::Down)
            | (Direction::Down, Direction::Up)
            | (Direction::Left, Direction::Right)
            | (Direction::Right, Direction::Left) => {}
            _ => self.direction = new_direction,
        }
    }

    fn check_self_collision(&self) -> bool {
        let head = *self.body.front().unwrap();
        self.body.iter().skip(1).any(|&pos| pos == head)
    }

    fn check_wall_collision(&self, width: u16, height: u16) -> bool {
        let head = *self.body.front().unwrap();
        head.x == 0 || head.x >= width - 1 || head.y == 0 || head.y >= height - 1
    }
}

struct Game {
    snake: Snake,
    food: Position,
    score: u32,
    width: u16,
    height: u16,
    game_over: bool,
    paused: bool,
}

impl Game {
    fn new(width: u16, height: u16) -> Self {
        let start_pos = Position {
            x: width / 2,
            y: height / 2,
        };
        let mut game = Game {
            snake: Snake::new(start_pos),
            food: Position { x: 0, y: 0 },
            score: 0,
            width,
            height,
            game_over: false,
            paused: false,
        };
        game.spawn_food();
        game
    }

    fn spawn_food(&mut self) {
        let mut rng = rand::thread_rng();
        loop {
            let food_pos = Position {
                x: rng.gen_range(1..self.width - 1),
                y: rng.gen_range(1..self.height - 1),
            };

            if !self.snake.body.contains(&food_pos) {
                self.food = food_pos;
                break;
            }
        }
    }

    fn update(&mut self) {
        if self.game_over || self.paused {
            return;
        }

        let food_eaten = *self.snake.body.front().unwrap() == self.food;
        self.snake.move_snake(food_eaten);

        if food_eaten {
            self.score += 10;
            self.spawn_food();
        }

        if self.snake.check_self_collision()
            || self.snake.check_wall_collision(self.width, self.height)
        {
            self.game_over = true;
        }
    }

    fn restart(&mut self) {
        let start_pos = Position {
            x: self.width / 2,
            y: self.height / 2,
        };
        self.snake = Snake::new(start_pos);
        self.score = 0;
        self.game_over = false;
        self.paused = false;
        self.spawn_food();
    }

    fn toggle_pause(&mut self) {
        if !self.game_over {
            self.paused = !self.paused;
        }
    }

    fn get_speed(&self) -> Duration {
        let base_speed: u64 = 200;
        let speed_increase = (self.score / 50) as u64;
        let current_speed = base_speed.saturating_sub(speed_increase * 10).max(50);
        Duration::from_millis(current_speed)
    }
}

fn draw_game(game: &Game) -> io::Result<()> {
    let mut stdout = stdout();

    execute!(stdout, Clear(ClearType::All), MoveTo(0, 0))?;

    // Draw borders
    for x in 0..game.width {
        execute!(
            stdout,
            MoveTo(x, 0),
            SetForegroundColor(Color::White),
            Print("█")
        )?;
        execute!(
            stdout,
            MoveTo(x, game.height - 1),
            SetForegroundColor(Color::White),
            Print("█")
        )?;
    }
    for y in 0..game.height {
        execute!(
            stdout,
            MoveTo(0, y),
            SetForegroundColor(Color::White),
            Print("█")
        )?;
        execute!(
            stdout,
            MoveTo(game.width - 1, y),
            SetForegroundColor(Color::White),
            Print("█")
        )?;
    }

    // Draw snake
    for (i, &pos) in game.snake.body.iter().enumerate() {
        execute!(stdout, MoveTo(pos.x, pos.y))?;
        if i == 0 {
            execute!(stdout, SetForegroundColor(Color::Yellow), Print("●"))?; // Head
        } else {
            execute!(stdout, SetForegroundColor(Color::Green), Print("■"))?; // Body
        }
    }

    // Draw food
    execute!(
        stdout,
        MoveTo(game.food.x, game.food.y),
        SetForegroundColor(Color::Red),
        Print("♦")
    )?;

    // Draw UI
    execute!(
        stdout,
        MoveTo(0, game.height + 1),
        SetForegroundColor(Color::White),
        Print(format!(
            "Score: {} | Speed Level: {}",
            game.score,
            game.score / 50 + 1
        ))
    )?;

    execute!(
        stdout,
        MoveTo(0, game.height + 2),
        Print("Controls: Arrow Keys/WASD - Move | P - Pause | R - Restart | Q - Quit")
    )?;

    if game.paused {
        execute!(
            stdout,
            MoveTo(game.width / 2 - 3, game.height / 2),
            SetForegroundColor(Color::Yellow),
            Print("PAUSED")
        )?;
    }

    if game.game_over {
        execute!(
            stdout,
            MoveTo(game.width / 2 - 5, game.height / 2),
            SetForegroundColor(Color::Red),
            Print("GAME OVER!")
        )?;
        execute!(
            stdout,
            MoveTo(game.width / 2 - 8, game.height / 2 + 1),
            SetForegroundColor(Color::White),
            Print("Press R to restart")
        )?;
    }

    execute!(stdout, ResetColor)?;
    stdout.flush()?;
    Ok(())
}

fn handle_input(game: &mut Game) -> io::Result<bool> {
    if poll(Duration::from_millis(0))? {
        if let Event::Key(KeyEvent { code, .. }) = read()? {
            match code {
                KeyCode::Up | KeyCode::Char('w') | KeyCode::Char('W') => {
                    game.snake.change_direction(Direction::Up);
                }
                KeyCode::Down | KeyCode::Char('s') | KeyCode::Char('S') => {
                    game.snake.change_direction(Direction::Down);
                }
                KeyCode::Left | KeyCode::Char('a') | KeyCode::Char('A') => {
                    game.snake.change_direction(Direction::Left);
                }
                KeyCode::Right | KeyCode::Char('d') | KeyCode::Char('D') => {
                    game.snake.change_direction(Direction::Right);
                }
                KeyCode::Char('p') | KeyCode::Char('P') => {
                    game.toggle_pause();
                }
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    game.restart();
                }
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                    return Ok(false);
                }
                _ => {}
            }
        }
    }
    Ok(true)
}

#[tokio::main]
async fn main() -> io::Result<()> {
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen, Hide)?;

    let width = 40;
    let height = 20;
    let mut game = Game::new(width, height);
    let mut last_update = Instant::now();

    let result = loop {
        if !handle_input(&mut game)? {
            break Ok(());
        }

        let now = Instant::now();
        if now.duration_since(last_update) >= game.get_speed() {
            game.update();
            last_update = now;
        }

        if let Err(e) = draw_game(&game) {
            break Err(e);
        }

        sleep(Duration::from_millis(10)).await;
    };

    execute!(stdout(), Show, LeaveAlternateScreen)?;
    disable_raw_mode()?;
    result
}
