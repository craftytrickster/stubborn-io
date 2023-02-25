stubborn-io
===========

This crate provides io traits/structs that automatically recover from potential disconnections/interruptions.

To use with your project, add the following to your Cargo.toml:

```toml
stubborn-io = "0.3"
```

API Documentation, examples and motivations can be found here -
https://docs.rs/stubborn-io .


### Usage Example

In this example, we will see a drop in replacement for tokio's TcpStream, with the
distinction that it will automatically attempt to reconnect in the face of connectivity failures.

```rust
use stubborn_io::StubbornTcpStream;
use tokio::io::AsyncWriteExt;

let addr = "localhost:8080";

// we are connecting to the TcpStream using the default built in options.
// these can also be customized (for example, the amount of reconnect attempts,
// wait duration, etc) using the connect_with_options method.
let mut tcp_stream = StubbornTcpStream::connect(addr).await?;
// once we acquire the wrapped IO, in this case, a TcpStream, we can
// call all of the regular methods on it, as seen below
tcp_stream.write_all(b"hello world!").await?;
```

