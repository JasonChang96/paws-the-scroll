# Contributing

## Code Quality

```bash
lychee -v .     # Lint markdown files for broken links and spelling errors
```

### TypeScript

```bash
pnpm lint       # Biome linting with auto-fix
pnpm typecheck  # TypeScript type checking
pnpm check      # Run all checks
```

### Rust

```bash
cargo clippy --all-targets --all-features  # Linting
cargo fmt                                   # Formatting
```

## Code Style & Philosophy

### Typing & Pattern Matching

- Prefer **explicit types** over raw dicts -- make invalid states unrepresentable where practical
- Prefer **typed variants over string literals** when the set of valid values is known
- Use **exhaustive pattern matching** (`match` in Python and Rust, `ts-pattern` in TypeScript) so the type checker can verify all cases are handled
- Structure types to enable exhaustive matching when handling variants
- Prefer **shared internal functions over factory patterns** when extracting common logic from hooks or functions -- keep each export explicitly defined for better IDE navigation and readability

### Self-Documenting Code

- **Verbose naming**: Variable and function naming should read like documentation
- **Strategic comments**: Only for non-obvious logic or architectural decisions; avoid restating what code shows
