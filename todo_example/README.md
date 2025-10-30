# Tiny Todo List

A simple command-line todo list application written in Rust.

## Features

- Add tasks
- List all tasks
- Mark tasks as done
- Delete tasks
- Persistent storage (saved to `todos.json`)

## Usage

Run the application:

```bash
cargo run
```

### Commands

- `add <task>` - Add a new task
- `list` - Display all tasks
- `done <id>` - Mark a task as completed
- `delete <id>` - Remove a task
- `quit` or `exit` - Exit the application

### Example Session

```
> add Buy groceries
Added task #1

> add Write report
Added task #2

> list
Todos:
  [ ] 1 - Buy groceries
  [ ] 2 - Write report

> done 1
Marked #1 as done!

> list
Todos:
  [✓] 1 - Buy groceries
  [ ] 2 - Write report

> delete 1
Deleted todo #1

> quit
Goodbye!
```

## Data Storage

Tasks are automatically saved to `todos.json` in the current directory.
