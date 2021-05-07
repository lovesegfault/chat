mod common;

use anyhow::Error;
use common::{TestClient as Client, TestServer as Server};

#[tokio::test]
async fn test_join_valid() -> Result<(), Error> {
    let server = Server::new().await?;

    let mut client_a = Client::new(&server.socket).await?;
    client_a.send("JOIN some_chan foo").await?;
    assert_eq!(client_a.recv().await?, "foo has joined");
    assert!(client_a.recv().await.is_err()); // should timeout

    let mut client_b = Client::new(&server.socket).await?;
    client_b.send("JOIN some_chan bar").await?;
    assert_eq!(client_a.recv().await?, "bar has joined");
    assert_eq!(client_b.recv().await?, "bar has joined");

    assert!(client_a.recv().await.is_err()); // should timeout
    assert!(client_b.recv().await.is_err()); // should timeout

    Ok(())
}

#[tokio::test]
async fn test_join_wrong_command() -> Result<(), Error> {
    let server = Server::new().await?;

    let mut client = Client::new(&server.socket).await?;
    client.send("WRONG some_chan foo").await?;
    let err_msg = client.recv().await?;
    assert_eq!(err_msg, "ERROR");

    Ok(())
}

#[tokio::test]
async fn test_join_wrong_channel_length() -> Result<(), Error> {
    let server = Server::new().await?;

    let mut client = Client::new(&server.socket).await?;
    client
        .send("JOIN this_channel_name_is_way_too_long_the_limit_is_20 foo")
        .await?;
    let err_msg = client.recv().await?;
    assert_eq!(err_msg, "ERROR");

    let mut client = Client::new(&server.socket).await?;
    client.send("JOIN 012345678901234567890 foo").await?;
    let err_msg = client.recv().await?;
    assert_eq!(err_msg, "ERROR");

    Ok(())
}

#[tokio::test]
async fn test_join_wrong_user_length() -> Result<(), Error> {
    let server = Server::new().await?;

    let mut client = Client::new(&server.socket).await?;
    client
        .send("JOIN some_chan this_user_name_is_way_too_long_the_limit_is_20")
        .await?;
    let err_msg = client.recv().await?;
    assert_eq!(err_msg, "ERROR");

    let mut client = Client::new(&server.socket).await?;
    client.send("JOIN some_chan 012345678901234567890").await?;
    let err_msg = client.recv().await?;
    assert_eq!(err_msg, "ERROR");

    Ok(())
}

#[tokio::test]
async fn test_join_all_wrong_lengths() -> Result<(), Error> {
    let server = Server::new().await?;

    let mut client = Client::new(&server.socket).await?;
    client
        .send("JOIN this_channel_name_is_way_too_long_the_limit_is_20 this_user_name_is_way_too_long_the_limit_is_20")
        .await?;
    let err_msg = client.recv().await?;
    assert_eq!(err_msg, "ERROR");

    Ok(())
}

#[tokio::test]
async fn test_join_incomplete_args() -> Result<(), Error> {
    let server = Server::new().await?;

    let mut client = Client::new(&server.socket).await?;
    client.send("JOIN").await?;
    let err_msg = client.recv().await?;
    assert_eq!(err_msg, "ERROR");

    let mut client = Client::new(&server.socket).await?;
    client.send("JOIN some_chan").await?;
    let err_msg = client.recv().await?;
    assert_eq!(err_msg, "ERROR");

    Ok(())
}

#[tokio::test]
async fn test_join_too_many_args() -> Result<(), Error> {
    let server = Server::new().await?;

    let mut client = Client::new(&server.socket).await?;
    client.send("JOIN some_chan some_user invalid").await?;
    let err_msg = client.recv().await?;
    assert_eq!(err_msg, "ERROR");

    let mut client = Client::new(&server.socket).await?;
    client
        .send("JOIN some_chan some_user invalid invalid invalid invalid invalid invalid")
        .await?;
    let err_msg = client.recv().await?;
    assert_eq!(err_msg, "ERROR");

    Ok(())
}

#[tokio::test]
async fn test_join_username_conflict() -> Result<(), Error> {
    let server = Server::new().await?;

    let mut client_a = Client::new(&server.socket).await?;
    client_a.send("JOIN some_chan foo").await?;
    assert_eq!(client_a.recv().await?, "foo has joined");
    assert!(client_a.recv().await.is_err()); // should timeout

    let mut client_b = Client::new(&server.socket).await?;
    client_b.send("JOIN some_chan foo").await?;
    assert!(client_a.recv().await.is_err()); // should timeout

    assert_eq!(client_b.recv().await?, "ERROR");

    Ok(())
}
