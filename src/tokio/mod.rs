//! Provides functionality related to asynchronous IO, including concrete
//! ready to use structs such as [StubbornTcpStream](StubbornTcpStream) as well as
//! the [trait](tokio::io::UnderlyingIo) and [struct](tokio::io::StubbornIo)
//! needed to create custom stubborn io types yourself.

mod io;
mod tcp;

pub use self::io::{StubbornIo, UnderlyingIo};

pub use self::tcp::StubbornTcpStream;

#[test]
fn test_compilation_for_doc_example() {
    use crate::tokio::{StubbornIo, UnderlyingIo};
    use std::error::Error;
    use std::future::Future;
    use std::path::PathBuf;
    use std::pin::Pin;
    use tokio::fs::File;

    impl UnderlyingIo<PathBuf> for File {
        // Implementing the creation function that will be used to establish an io connection.
        fn create(path: PathBuf) -> Pin<Box<dyn Future<Output = Result<Self, Box<dyn Error>>>>> {
            Box::pin(async move { Ok(File::create(path).await?) })
        }
    }

    type HomemadeStubbornFile = StubbornIo<File, PathBuf>;
    let _ = HomemadeStubbornFile::connect(PathBuf::from("foo"));
}
