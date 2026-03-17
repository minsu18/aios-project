# Git Usage (AIOS)

## Quick aliases

| Command | Description |
|---------|-------------|
| `git ac` | Add all changes + status |
| `git cm "message"` | Commit (after add) |
| `git acp "message"` | Add + commit + push |
| `git acp` | Add + commit("update") + push |
| `git pushup` | First push: `git push -u origin main` |

## Typical workflow

```bash
cd path/to/aios-project

# One-shot: add, commit, push
git acp "feat: add new feature"

# Or step by step
git ac
git cm "fix: bug fix"
git push
```

## First push

1. Create `aios-project` repo as **Public** at https://github.com/new
2. Run:

```bash
git pushup
```

or

```bash
git push -u origin main
```

Credentials are stored in macOS Keychain after the first auth.
