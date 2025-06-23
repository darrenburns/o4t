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
```

CLI options have the highest precedence, followed by environment variables, followed by `config.toml`.

## Themes

o4t supports various themes, like `dracula` and `tokyo-night`:

<img width="765" alt="image" src="https://github.com/user-attachments/assets/efa3ea39-c4d1-41bd-bcab-02fe945d8275" />

<img width="756" alt="image" src="https://github.com/user-attachments/assets/839e573c-9c27-4d5d-a9d8-ef3825668bc5" />

Supported themes: `terminal-yellow`, `terminal-cyan`, `nord`, `catppuccin-mocha`, `dracula`, `gruvbox`, `solarized-dark`, `tokyo-night`, `monokai`, and `galaxy`.

Themes prefixed with `terminal-` use your terminal emulator's ANSI colours.

## History

This hasn't been implemented, but I'm considering saving data for each session in JSONL format or a local SQLite database.

## More Info

- _WPM_ is "Words Per Minute", and is computed as `((character_matches + num_correct_words) / 5.) * (60. / seconds_elapsed)`. There is currently no penalty for incorrectly typed words, meaning if you miss one character in a word, the other correctly typed characters will still be counted.
- _Accuracy_ is the percentage of all characters typed during the game that matched the expected character. This means that if you've made corrections during a game, you will not have 100% accuracy.
- On the score screen, "Perfect!" will only appear if you made no mistakes at any time during the game.
