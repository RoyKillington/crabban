# 🦀📋 Crabban

A tiny CLI kanban board, written in Rust. Crab + Kanban = Crabban.

Born as a learning project, kept because it's actually useful.

## Install

```bash
cargo install --path .
```

## Usage

```bash
# Add tasks
crabban add "Build the thing" --project mycel --points 5
crabban add "Fix the other thing" --project crabban --desc "It's broken" --points 2

# See your board
crabban show
crabban show --project mycel
crabban show --status sprint

# Move tasks through the pipeline
crabban move 1 ondeck
crabban move 1 sprint
crabban done 1

# Remove a task
crabban delete 2
```

## The Board

Tasks flow left to right through four columns:

```
BACKLOG → ON DECK → SPRINT → DONE
```

That's it. No sprints to configure, no ceremonies to schedule, no velocity charts to pretend to look at. Add tasks, move them across, get things done.

## Data

Your board lives at `~/.local/share/crabban/board.json`. It's plain JSON — readable, portable, and easy to back up. Crabban creates it automatically on first run.

## Built With

- [clap](https://docs.rs/clap) — CLI argument parsing
- [serde](https://serde.rs/) — JSON serialization
- [colored](https://docs.rs/colored) — terminal colors
- [chrono](https://docs.rs/chrono) — timestamps

## Why?

I wanted to learn Rust by building something I'd actually use.

## License

MIT
