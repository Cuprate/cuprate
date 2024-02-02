# Development environment advice

This documentation contain advice for setting up the development environment.

Cuprate is a rust project, and therefore inherit the use of the default of LSP plugin Rust-analyzer. Rust-analyzer is well conceived but can be 
slow on big project such as Cuprate. 

Here are following suggested configurations from Polkadot-SDK's documentation:

### Rust-analyzer's VSCode plugin:

```json
{
  // Use a separate target dir for Rust Analyzer. Helpful if you want to use Rust
  // Analyzer and cargo on the command line at the same time.
  "rust-analyzer.rust.analyzerTargetDir": "target/vscode-rust-analyzer",
  // Improve stability
  "rust-analyzer.server.extraEnv": {
    "CHALK_OVERFLOW_DEPTH": "100000000",
    "CHALK_SOLVER_MAX_SIZE": "10000000"
  },
  // Check feature-gated code
  "rust-analyzer.cargo.features": "all",
  "rust-analyzer.cargo.extraEnv": {
    // Skip building WASM, there is never need for it here
    "SKIP_WASM_BUILD": "1"
  },
  // Don't expand some problematic proc_macros
  "rust-analyzer.procMacro.ignored": {
    "async-trait": ["async_trait"],
    "napi-derive": ["napi"],
    "async-recursion": ["async_recursion"],
    "async-std": ["async_std"]
  },
  // Use nightly formatting.
  // See the polkadot-sdk CI job that checks formatting for the current version used in
  // polkadot-sdk.
  "rust-analyzer.rustfmt.extraArgs": ["+nightly-2024-01-22"],
}
```

### Rust-analyzer's Neovim LUA plugin:

```lua
["rust-analyzer"] = {
  rust = {
    # Use a separate target dir for Rust Analyzer. Helpful if you want to use Rust
    # Analyzer and cargo on the command line at the same time.
    analyzerTargetDir = "target/nvim-rust-analyzer",
  },
  server = {
    # Improve stability
    extraEnv = {
      ["CHALK_OVERFLOW_DEPTH"] = "100000000",
      ["CHALK_SOLVER_MAX_SIZE"] = "100000000",
    },
  },
  cargo = {
    # Check feature-gated code
    features = "all",
    extraEnv = {
      # Skip building WASM, there is never need for it here
      ["SKIP_WASM_BUILD"] = "1",
    },
  },
  procMacro = {
    # Don't expand some problematic proc_macros
    ignored = {
      ["async-trait"] = { "async_trait" },
      ["napi-derive"] = { "napi" },
      ["async-recursion"] = { "async_recursion" },
      ["async-std"] = { "async_std" },
    },
  },
  rustfmt = {
    # Use nightly formatting.
    # See the polkadot-sdk CI job that checks formatting for the current version used in
    # polkadot-sdk.
    extraArgs = { "+nightly-2024-01-22" },
  },
},
```

### Usage of cargo -p

Prefer to use cargo -p <CRATE> while possible, as any cargo in workspace mode will check and build dependencies of every crates in the repository.

On Rust-analyzer's VSCode plugin, you can add the following configuration if you're focused on one specific crate:

```json
"rust-analyzer.check.extraArgs": [
	"-p <CRATE_NAME>"
],
```

### Alternative IDE

If you still deal with lags on VSCode or Neovim, you could try the following IDE:
- RustRover: It have been reported to have excellent performance at managing huge workspace. It use its own fine-tuned plugins by jetbrains.
- Zed: Rust-written IDE focused on performance. Still in beta and macOS only.