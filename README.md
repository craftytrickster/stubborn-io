stubborn-io
===========

*Note: This lib will not be published to crates.io until tokio creates
an official async/await release in Rust 1.38. In the meantime, this is
a nightly only crate.*

This crate provides io traits/structs that automatically recover from potential disconnections/interruptions.

To use with your project, add the following to your Cargo.toml:

```toml
stubborn-io = { git = "https://github.com/craftytrickster/stubborn-io" }
```

API Documentation, examples and motivations can be found here -
https://docs.rs/stubborn-io/0.1.1/stubborn-io/ .
