stubborn-io
===========

This crate provides io traits/structs that automatically recover from potential disconnections/interruptions.

To use with your project, add the following to your Cargo.toml:

```toml
stubborn-io = "0.1"
```

API Documentation, examples and motivations can be found here -
https://docs.rs/stubborn-io .

*Note: This crate requires at least version 1.39 of the Rust compiler.*


### Usage Example

In this example, we will see a drop in replacement for tokio's TcpStream, with the
distinction that it will automatically attempt to reconnect in the face of connectivity failures.

```rust
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use stubborn_io::StubbornTcpStream;

let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);

async {
    // we are connecting to the TcpStream using the default built in options.
    // these can also be customized (for example, the amount of reconnect attempts,
    // wait duration, etc) using the connect_with_options method.
    let tcp_stream = StubbornTcpStream::connect(&addr).await.unwrap();

    // once we acquire the wrapped IO, in this case, a TcpStream, we can
    // call all of the regular methods on it, as seen below
    let regular_tokio_tcp_function_result = tcp_stream.peer_addr();
};
```

