mod common;

use anyhow::Error;
use common::{TestClient as Client, TestServer as Server};

#[tokio::test]
async fn test_chat_session() -> Result<(), Error> {
    let server = Server::new().await?;

    let mut joe = Client::new(&server.socket).await?;
    joe.send("JOIN cooking joe").await?;
    assert_eq!(joe.recv().await?, "joe has joined");
    joe.send("no one here yet").await?;
    assert_eq!(joe.recv().await?, "joe: no one here yet");
    joe.send("JOIN no longer does anything").await?;
    assert_eq!(joe.recv().await?, "joe: JOIN no longer does anything");
    assert!(joe.recv().await.is_err()); // should timeout

    let mut bob = Client::new(&server.socket).await?;
    bob.send("JOIN cooking bob").await?;
    assert_eq!(joe.recv().await?, "bob has joined");
    assert_eq!(bob.recv().await?, "bob has joined");
    assert!(bob.recv().await.is_err()); // should timeout
    assert!(joe.recv().await.is_err()); // should timeout

    bob.send("hi joe").await?;
    assert_eq!(joe.recv().await?, "bob: hi joe");
    assert_eq!(bob.recv().await?, "bob: hi joe");
    assert!(joe.recv().await.is_err()); // should timeout
    assert!(bob.recv().await.is_err()); // should timeout

    joe.send("good day, bob").await?;
    assert_eq!(joe.recv().await?, "joe: good day, bob");
    assert_eq!(bob.recv().await?, "joe: good day, bob");
    assert!(bob.recv().await.is_err()); // should timeout
    assert!(joe.recv().await.is_err()); // should timeout

    drop(joe);

    assert_eq!(bob.recv().await?, "joe has left");
    bob.send("all alone now").await?;
    assert_eq!(bob.recv().await?, "bob: all alone now");
    assert!(bob.recv().await.is_err()); // should timeout

    Ok(())
}
