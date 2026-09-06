# orwatch

OpenRouter spend watcher. Terminal report plus a waybar module.

Reads `GET https://openrouter.ai/api/v1/key` with your normal API key (not a management key).

```
orwatch           # today / week / month / key cap
orwatch waybar    # JSON for waybar
orwatch json      # raw snapshot
orwatch --fresh   # skip cache
```

## Key

`OPENROUTER_API_KEY` from the environment, or from `~/.secrets` (same file zsh sources). Waybar does not load `.zshrc`, so the binary reads `~/.secrets` itself.

## Install

```sh
git clone https://github.com/Streppel/orwatch.git ~/code/rust/orwatch
cargo install --path ~/code/rust/orwatch --root ~/.local
```

Binary lands at `~/.local/bin/orwatch`. Waybar wiring lives in [Streppel/dotfiles](https://github.com/Streppel/dotfiles) (`custom/orwatch`).

Optional config: `~/.config/orwatch/config.toml` (see `config.example.toml`).
