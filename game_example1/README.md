# Tic-Tac-Toe Game

A simple command-line tic-tac-toe game written in Rust.

## How to Play

1. Build and run the game:
   ```bash
   cd game_example1
   cargo run
   ```

2. The game will display a 3x3 grid with coordinates:
   - Rows are numbered 1-3
   - Columns are numbered 1-3

3. Players take turns entering their moves:
   - Enter row and column numbers separated by a space
   - Example: `1 2` places your mark in row 1, column 2

4. Player X goes first, followed by Player O

5. Win by getting three marks in a row (horizontally, vertically, or diagonally)

6. The game ends when someone wins or the board is full (draw)

## Example Gameplay

```
=== Tic-Tac-Toe ===

  1   2   3
1   |   |
  -----------
2   |   |
  -----------
3   |   |

Player X's turn
Enter row and column (e.g., 1 2): 2 2

  1   2   3
1   |   |
  -----------
2   | X |
  -----------
3   |   |

Player O's turn
...
```

## Requirements

- Rust (edition 2021 or later)
- Cargo

Enjoy the game!
