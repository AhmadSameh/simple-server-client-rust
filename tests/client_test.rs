use embedded_recruitment_task::{
    message::{client_message, server_message, AddRequest, EchoMessage, ServerMessage},
    server::Server,
};
use prost::Message;
use std::{
    sync::Arc,
    thread::{self, JoinHandle},
    time::Duration
};
use std::io::{Write, Read};

mod client;

fn setup_server_thread(server: Arc<Server>) -> JoinHandle<()> {
    thread::spawn(move || {
        server.run().expect("Server encountered an error");
    })
}

fn create_server() -> Arc<Server> {
    Arc::new(Server::new("localhost:8080").expect("Failed to start server"))
}

#[test]
fn test_client_connection() {
    // Set up the server in a separate thread
    let server = create_server();
    let handle = setup_server_thread(server.clone());

    // Create and connect the client
    let mut client = client::Client::new("localhost", 8080, 1000);
    assert!(client.connect().is_ok(), "Failed to connect to the server");

    // Disconnect the client
    assert!(
        client.disconnect().is_ok(),
        "Failed to disconnect from the server"
    );

    // Stop the server and wait for thread to finish
    server.stop();
    assert!(
        handle.join().is_ok(),
        "Server thread panicked or failed to join"
    );
}

#[test]
fn test_client_echo_message() {
    // Set up the server in a separate thread
    let server = create_server();
    let handle = setup_server_thread(server.clone());

    // Create and connect the client
    let mut client = client::Client::new("localhost", 8080, 1000);
    assert!(client.connect().is_ok(), "Failed to connect to the server");

    // Prepare the message
    let mut echo_message = EchoMessage::default();
    echo_message.content = "Hello, World!".to_string();
    let message = client_message::Message::EchoMessage(echo_message.clone());

    // Send the message to the server
    assert!(client.send(message).is_ok(), "Failed to send message");

    // Receive the echoed message
    let response = client.receive();
    assert!(
        response.is_ok(),
        "Failed to receive response for EchoMessage"
    );

    match response.unwrap().message {
        Some(server_message::Message::EchoMessage(echo)) => {
            assert_eq!(
                echo.content, echo_message.content,
                "Echoed message content does not match"
            );
        }
        _ => panic!("Expected EchoMessage, but received a different message"),
    }

    // Disconnect the client
    assert!(
        client.disconnect().is_ok(),
        "Failed to disconnect from the server"
    );

    // Stop the server and wait for thread to finish
    server.stop();
    assert!(
        handle.join().is_ok(),
        "Server thread panicked or failed to join"
    );
}

#[test]
fn test_multiple_echo_messages() {
    // Set up the server in a separate thread
    let server = create_server();
    let handle = setup_server_thread(server.clone());

    // Create and connect the client
    let mut client = client::Client::new("localhost", 8080, 1000);
    assert!(client.connect().is_ok(), "Failed to connect to the server");

    // Prepare multiple messages
    let messages = vec![
        "Hello, World!".to_string(),
        "How are you?".to_string(),
        "Goodbye!".to_string(),
    ];

    // Send and receive multiple messages
    for message_content in messages {
        let mut echo_message = EchoMessage::default();
        echo_message.content = message_content.clone();
        let message = client_message::Message::EchoMessage(echo_message);

        // Send the message to the server
        assert!(client.send(message).is_ok(), "Failed to send message");

        // Receive the echoed message
        let response = client.receive();
        assert!(
            response.is_ok(),
            "Failed to receive response for EchoMessage"
        );

        match response.unwrap().message {
            Some(server_message::Message::EchoMessage(echo)) => {
                assert_eq!(
                    echo.content, message_content,
                    "Echoed message content does not match"
                );
            }
            _ => panic!("Expected EchoMessage, but received a different message"),
        }
    }

    // Disconnect the client
    assert!(
        client.disconnect().is_ok(),
        "Failed to disconnect from the server"
    );

    // Stop the server and wait for thread to finish
    server.stop();
    assert!(
        handle.join().is_ok(),
        "Server thread panicked or failed to join"
    );
}

#[test]
fn test_multiple_clients() {
    // Set up the server in a separate thread
    let server = create_server();
    let handle = setup_server_thread(server.clone());

    // Create and connect multiple clients
    let mut clients = vec![
        client::Client::new("localhost", 8080, 1000),
        client::Client::new("localhost", 8080, 1000),
        client::Client::new("localhost", 8080, 1000),
    ];

    for client in clients.iter_mut() {
        assert!(client.connect().is_ok(), "Failed to connect to the server");
    }

    // Prepare multiple messages
    let messages = vec![
        "Hello, World!".to_string(),
        "How are you?".to_string(),
        "Goodbye!".to_string(),
    ];

    // Send and receive multiple messages for each client
    for message_content in messages {
        let mut echo_message = EchoMessage::default();
        echo_message.content = message_content.clone();
        let message = client_message::Message::EchoMessage(echo_message.clone());

        for client in clients.iter_mut() {
            // Send the message to the server
            assert!(
                client.send(message.clone()).is_ok(),
                "Failed to send message"
            );

            // Receive the echoed message
            let response = client.receive();
            assert!(
                response.is_ok(),
                "Failed to receive response for EchoMessage"
            );

            match response.unwrap().message {
                Some(server_message::Message::EchoMessage(echo)) => {
                    assert_eq!(
                        echo.content, message_content,
                        "Echoed message content does not match"
                    );
                }
                _ => panic!("Expected EchoMessage, but received a different message"),
            }
        }
    }

    // Disconnect the clients
    for client in clients.iter_mut() {
        assert!(
            client.disconnect().is_ok(),
            "Failed to disconnect from the server"
        );
    }

    // Stop the server and wait for thread to finish
    server.stop();
    assert!(
        handle.join().is_ok(),
        "Server thread panicked or failed to join"
    );
}

#[test]
fn test_client_add_request() {
    // Set up the server in a separate thread
    let server = create_server();
    let handle = setup_server_thread(server.clone());

    // Create and connect the client
    let mut client = client::Client::new("localhost", 8080, 1000);
    assert!(client.connect().is_ok(), "Failed to connect to the server");

    // Prepare the message
    let mut add_request = AddRequest::default();
    add_request.a = 10;
    add_request.b = 20;
    let message = client_message::Message::AddRequest(add_request.clone());

    // Send the message to the server
    assert!(client.send(message).is_ok(), "Failed to send message");

    // Receive the response
    let response = client.receive();
    assert!(
        response.is_ok(),
        "Failed to receive response for AddRequest"
    );

    match response.unwrap().message {
        Some(server_message::Message::AddResponse(add_response)) => {
            assert_eq!(
                add_response.result,
                add_request.a + add_request.b,
                "AddResponse result does not match"
            );
        }
        _ => panic!("Expected AddResponse, but received a different message"),
    }

    // Disconnect the client
    assert!(
        client.disconnect().is_ok(),
        "Failed to disconnect from the server"
    );

    // Stop the server and wait for thread to finish
    server.stop();
    assert!(
        handle.join().is_ok(),
        "Server thread panicked or failed to join"
    );
}

// The following test is aimed at performing echo requests
// and add requests in parallel.
#[test]
fn test_parallel_client_requests() {
    // Set up the server in a separate thread
    let server = create_server();
    let handle = setup_server_thread(server.clone());

    // Spawn ten client threads.
    let clients: Vec<_> = (0..10).map(|i| {
        thread::spawn(move || {
            // Create and connect the client
            let mut client = client::Client::new("localhost", 8080, 1000);
            assert!(client.connect().is_ok(), "Failed to connect to the server");

            if i%2 == 0 {
                // Send an echo message request.
                // Prepare the message
                let mut echo_message = EchoMessage::default();
                echo_message.content = format!("Hello, World From Client {}!", i);
                let message = client_message::Message::EchoMessage(echo_message.clone());

                // Send the message to the server
                assert!(client.send(message).is_ok(), "Failed to send message");

                // Receive the echoed message
                let response = client.receive();
                assert!(
                    response.is_ok(),
                    "Failed to receive response for EchoMessage"
                );

                match response.unwrap().message {
                    Some(server_message::Message::EchoMessage(echo)) => {
                        assert_eq!(
                            echo.content, echo_message.content,
                            "Echoed message content does not match"
                        );
                    }
                    _ => panic!("Expected EchoMessage, but received a different message"),
                }
            } else {
                // Send an add request.
                // Prepare the message
                let mut add_request = AddRequest::default();
                add_request.a = i;
                add_request.b = i;
                let message = client_message::Message::AddRequest(add_request.clone());

                // Send the message to the server
                assert!(client.send(message).is_ok(), "Failed to send message");

                // Receive the response
                let response = client.receive();
                assert!(
                    response.is_ok(),
                    "Failed to receive response for AddRequest"
                );

                match response.unwrap().message {
                    Some(server_message::Message::AddResponse(add_response)) => {
                        assert_eq!(
                            add_response.result,
                            add_request.a + add_request.b,
                            "AddResponse result does not match"
                        );
                    }
                    _ => panic!("Expected AddResponse, but received a different message"),
                }
            }
        })
    }).collect();

    // Wait until all client thread receieve their requests.
    for client in clients {
        client.join().unwrap();
    }

    // Stop the server and wait for thread to finish
    server.stop();
    assert!(
        handle.join().is_ok(),
        "Server thread panicked or failed to join"
    );
}

// The following test is aimed at testing how a server
// would handle a bad request.
#[test]
fn test_client_bad_request() {
    // Set up the server in a separate thread
    let server = create_server();
    let handle = setup_server_thread(server.clone());

    // Create a direct TcpStream to the server, since the client struct
    // will not recoginze the corrupt data.
    let mut stream = std::net::TcpStream::connect("localhost:8080").expect("Failed to connect directly to the server");

    // Send the corrupt data 0xdeadbeef over the stream
    let malformed_data = vec![0xde, 0xad, 0xbe, 0xef];
    stream.write_all(&malformed_data).expect("Failed to send malformed data");
    stream.flush().expect("Failed to flush stream");

    // Read data which the server sent.
    let mut buffer = [0; 512];
    let bytes_read = stream.read(&mut buffer).expect("Failed to read response from the server");

    // Decode the received server response.
    let server_response = ServerMessage::decode(&buffer[..bytes_read]).expect("Failed to decode server response");

    // Check the incoming value.
    match server_response.message {
        Some(server_message::Message::ErrorMessage(error_message)) => {
            assert_eq!(
                error_message.content, "Bad Request!",
                "Unexpected error message content"
            );
        }
        _ => panic!("Expected ErrorMessage, but received a different message type"),
    }

    // Disconnect the stream.
    stream.shutdown(std::net::Shutdown::Both).expect("Failed to shut down the stream");

    // Stop the server and wait for the thread to finish
    server.stop();
    assert!(
        handle.join().is_ok(),
        "Server thread panicked or failed to join"
    );
}

// The following test is aimed at testing how the client
// would behave when the server shuts own mid execution.
#[test]
fn test_server_failure() {
    // Set up the server in a separate thread
    let server = create_server();
    let server_handle = setup_server_thread(server.clone());

    // Create and connect the client
    let mut client = client::Client::new("localhost", 8080, 1000);
    assert!(client.connect().is_ok(), "Failed to connect to the server");

    // Spawn a thread to stop the server after 2 seconds.
    let stop_thread = thread::spawn(move || {
        thread::sleep(Duration::from_secs(2));
        server.stop();
    });

    // Iterate indefinetly until the server stops.
    for i in 0.. {
        // Prepare the message
        let mut echo_message = EchoMessage::default();
        echo_message.content = format!("Message #{}", i);
        let message = client_message::Message::EchoMessage(echo_message.clone());

        // Send the message to the server
        assert!(client.send(message).is_ok(), "Failed to send message");

        // Receive the server response.
        let response = client.receive();
        assert!(
            response.is_ok(),
            "Failed to receive response for EchoMessage"
        );

        match response.unwrap().message {
            Some(server_message::Message::EchoMessage(message)) => {
                assert_eq!(
                    message.content, echo_message.content,
                    "Returned error message content does not match"
                );
            }
            Some(server_message::Message::ErrorMessage(error)) => {
                assert_eq!(
                    error.content, "Server is shutting down.",
                    "Returned error message content does not match"
                );
                break;
            }
            _ => panic!("Expected ErrorMessage or EchoMessage, but received a different message"),
        }

        // Sleep for a short duration to simulate message intervals
        thread::sleep(Duration::from_millis(100));
    }

    assert!(
        stop_thread.join().is_ok(),
        "Client thread panicked or failed to join"
    );

    assert!(
        server_handle.join().is_ok(),
        "Server thread panicked or failed to join"
    );

    // Ensure the client detects the disconnection
    assert!(client.disconnect().is_ok(), "Client failed to disconnect properly");
}
