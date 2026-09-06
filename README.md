# orwatch

OpenRouter spend watcher. Terminal report plus a waybar module.

Reads `GET https://openrouter.ai/api/v1/key` with your normal API key (not a management key).

```
orwatch           # today / week / month / key cap
orwatch waybar    # JSON for waybar
orwatch json      # raw snapshot
orwatch --fresh   # skip cache
```

```
  env  or  ~/.secrets
           |
           v
        orwatch ------ GET /api/v1/key
           |    \
           |     `--> ~/.cache/orwatch  (reuse ~50s)
           |
           +-- status --> human report  (terminal)
           +-- waybar --> { text, tooltip, class }  --> waybar
           +-- json   --> raw snapshot
```

Waybar only consumes that JSON. Module layout and CSS live in the [dotfiles](https://github.com/Streppel/dotfiles), not here.

## Key

`OPENROUTER_API_KEY` from the environment, or from `~/.secrets` (same file zsh sources). Waybar does not load `.zshrc`, so the binary reads `~/.secrets` itself.

## Install

```sh
git clone https://github.com/Streppel/orwatch.git ~/code/rust/orwatch
cargo install --path ~/code/rust/orwatch --root ~/.local
```

Binary lands at `~/.local/bin/orwatch`. Waybar wiring lives in [Streppel/dotfiles](https://github.com/Streppel/dotfiles) (`custom/orwatch`).

Optional config: `~/.config/orwatch/config.toml` (see `config.example.toml`).
