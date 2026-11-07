# orwatch

OpenRouter spend watcher. Terminal report plus a waybar module.

Reads `GET /api/v1/credits` and `/api/v1/key` with your OpenRouter API key
(not a management key). `credits` supplies the account balance; `key` supplies
the spend periods and any per-key cap.

```
orwatch           # balance + today / week / month / key cap
orwatch waybar    # JSON for waybar
orwatch json      # raw snapshot
orwatch --fresh   # skip cache
```

```
  env  or  ~/.secrets
           |
           v
        orwatch ------ GET /api/v1/credits + /api/v1/key
           |    \
           |     `--> ~/.cache/orwatch  (reuse ~50s)
           |
           +-- status --> human report  (terminal)
           +-- waybar --> { text, tooltip, class, percentage }  --> waybar
           +-- json   --> raw snapshot
```

Waybar only consumes that JSON. Module layout and CSS live in the [dotfiles](https://github.com/Streppel/dotfiles), not here.

The waybar text shows the most specific budget first: remaining key cap when
the key has one, otherwise the account balance left plus the percentage of
purchased credits still available, otherwise today's spend. The tooltip always
lists the full balance and all spend periods.

## Key

`OPENROUTER_API_KEY` from the environment, or from `~/.secrets` (same file zsh sources). Waybar does not load `.zshrc`, so the binary reads `~/.secrets` itself.

## Install

```sh
git clone https://github.com/Streppel/orwatch.git ~/code/rust/orwatch
cargo install --path ~/code/rust/orwatch --root ~/.local
```

Binary lands at `~/.local/bin/orwatch`. Waybar wiring lives in [Streppel/dotfiles](https://github.com/Streppel/dotfiles) (`custom/orwatch`).

Optional config: `~/.config/orwatch/config.toml` (see `config.example.toml`).
