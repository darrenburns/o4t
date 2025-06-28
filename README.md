# `o4t`

o4t (pronounced _"oat"_) is a typing game that runs in your terminal.

<img width="812" alt="image" src="https://github.com/user-attachments/assets/c3e88646-ba5b-4cb3-8d65-fe0ab33d0739" />

It's heavily inspired by [monkeytype](https://monkeytype.com).

## Installation

> [!IMPORTANT]  
> o4t isn't yet released, and I'm actively working on the `main` branch.

Right now, you'll have to check out the repo and run it with `cargo run`, but it might not work. Sorry!

## Usage

Run `o4t` via the command line. Start typing the words on screen to immediately start a typing test.

### Configuration 

Pass config to o4t via the CLI, environment variables, or `config.toml`.

- `-t`/`--time`: the duration of games in seconds
- `-c`/`--cursor`: either `underline`, `block`, or `none` - the type of cursor to use
- `--theme`: the theme to use
- `--current-word`: either `bold`, `highlight`, or `none` - how the word under the cursor should be highlighted
- `--target-wpm`: if non-zero, displays a "ghost" cursor which moves at the specified wpm

To use environment variables, simply take the name of the CLI option, prefix it with `O4T_`, upper-case it, and convert `-` to `_`. 

For example, you could invoke o4t via the command line like so `O4T_CURRENT_WORD=bold o4t`. This is equivalent to `o4t --current-word bold`.

All config options can also be set via the `config.toml` file, and use snake case.
The location of this file is shown in the output of `o4t --help`.
Here's an example `config.toml`:

```toml
current_word = "highlight"
cursor = "underline"
theme = "gruvbox"
time = 45
target_wpm = 100
```

CLI options have the highest precedence, followed by environment variables, followed by `config.toml`.

## Themes

o4t supports various themes, including `nord`, `catppuccin-mocha`, `dracula`, `gruvbox`, `solarized-dark`, `tokyo-night`, `monokai`, `galaxy`, `terminal-yellow`, `terminal-cyan`.

Themes prefixed with `terminal-` use your terminal emulator's ANSI colours.

## Word lists

This is a WIP - there's currently only 1 word list - "English Top 1k", and it's defined in code. The plan is to just be able to load arbitrary word lists from disk, but I haven't implemented that yet.

## Target WPM

o4t can display a "pace cursor" which you can race against. The speed of this cursor is defined by the `target_wpm` config.

![o4t-ghost-cursor-short](https://github.com/user-attachments/assets/bf69167a-4c83-4d70-83a5-8663a1d83ae7)

## History

This hasn't been implemented, but I'm considering saving data for each session in JSONL format or a local SQLite database.

## More Info

- _WPM_ is "Words Per Minute", and is computed as `((character_matches + num_correct_words) / 5.) * (60. / seconds_elapsed)`. There is currently no penalty for incorrectly typed words, meaning if you miss one character in a word, the other correctly typed characters will still be counted.
- _Accuracy_ is the percentage of all characters typed during the game that matched the expected character. This means that if you've made corrections during a game, you will not have 100% accuracy.
- On the score screen, "Perfect!" will only appear if you made no mistakes at any time during the game.
