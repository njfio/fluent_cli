use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug)]
struct Todo {
    id: usize,
    task: String,
    done: bool,
}

struct TodoList {
    todos: Vec<Todo>,
    file_path: PathBuf,
}

impl TodoList {
    fn new() -> Self {
        let file_path = PathBuf::from("todos.json");
        let todos = Self::load_from_file(&file_path).unwrap_or_default();
        Self { todos, file_path }
    }

    fn load_from_file(path: &PathBuf) -> Result<Vec<Todo>, Box<dyn std::error::Error>> {
        let contents = fs::read_to_string(path)?;
        let todos = serde_json::from_str(&contents)?;
        Ok(todos)
    }

    fn save_to_file(&self) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(&self.todos)?;
        fs::write(&self.file_path, json)?;
        Ok(())
    }

    fn add(&mut self, task: String) {
        let id = self.todos.iter().map(|t| t.id).max().unwrap_or(0) + 1;
        self.todos.push(Todo {
            id,
            task,
            done: false,
        });
        self.save_to_file().ok();
        println!("Added task #{}", id);
    }

    fn list(&self) {
        if self.todos.is_empty() {
            println!("No todos yet!");
            return;
        }
        println!("\nTodos:");
        for todo in &self.todos {
            let status = if todo.done { "✓" } else { " " };
            println!("  [{}] {} - {}", status, todo.id, todo.task);
        }
        println!();
    }

    fn done(&mut self, id: usize) {
        if let Some(todo) = self.todos.iter_mut().find(|t| t.id == id) {
            todo.done = true;
            self.save_to_file().ok();
            println!("Marked #{} as done!", id);
        } else {
            println!("Todo #{} not found", id);
        }
    }

    fn delete(&mut self, id: usize) {
        let len_before = self.todos.len();
        self.todos.retain(|t| t.id != id);
        if self.todos.len() < len_before {
            self.save_to_file().ok();
            println!("Deleted todo #{}", id);
        } else {
            println!("Todo #{} not found", id);
        }
    }
}

fn main() {
    let mut todo_list = TodoList::new();

    println!("Tiny Todo List");
    println!("Commands: add, list, done, delete, quit");

    loop {
        print!("\n> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim();

        let parts: Vec<&str> = input.splitn(2, ' ').collect();
        let command = parts[0];

        match command {
            "add" => {
                if parts.len() < 2 {
                    println!("Usage: add <task>");
                } else {
                    todo_list.add(parts[1].to_string());
                }
            }
            "list" => todo_list.list(),
            "done" => {
                if parts.len() < 2 {
                    println!("Usage: done <id>");
                } else if let Ok(id) = parts[1].parse() {
                    todo_list.done(id);
                } else {
                    println!("Invalid id");
                }
            }
            "delete" => {
                if parts.len() < 2 {
                    println!("Usage: delete <id>");
                } else if let Ok(id) = parts[1].parse() {
                    todo_list.delete(id);
                } else {
                    println!("Invalid id");
                }
            }
            "quit" | "exit" => {
                println!("Goodbye!");
                break;
            }
            "" => continue,
            _ => println!("Unknown command. Available: add, list, done, delete, quit"),
        }
    }
}
