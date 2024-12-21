use crate::message::{ client_message, server_message, AddRequest, AddResponse, ClientMessage, EchoMessage, ServerMessage, ErrorMessage};
use log::{error, info, warn};
use prost::Message;
use std::{
        io::{self, ErrorKind, Read, Write}, net::{TcpListener, TcpStream}, sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex
    }, thread, time::Duration
};
use threadpool::ThreadPool;

struct Client {
    stream: TcpStream,
}

impl Client {
    /// Creates a new client instance.
    ///
    /// # Arguments
    /// - `stream` TCP stream object that reads from and writes to the network.
    pub fn new(stream: TcpStream) -> Self {
        Client { stream }
    }

    /// Handle the incoming client request and send a reply according to the request.
    ///
    /// # Returns
    /// - Ok    upon successful message decoding and handling.
    /// - Err   when either the decoding or the handling fails.
    pub fn handle(&mut self) -> io::Result<()> {
        let mut buffer = [0; 512];
        // Read data from the client
        let bytes_read = self.stream.read(&mut buffer)?;
        if bytes_read == 0 {
            info!("Client disconnected.");
            return Ok(());
        }

        // Decode the message to decide on the type of the request.
        if let Ok(client_request) = ClientMessage::decode(&buffer[..bytes_read]) {
            match client_request.message {
                Some(client_message::Message::EchoMessage(echo_message)) => {
                    self.handle_echo_request(echo_message);
                } Some(client_message::Message::AddRequest(add_request)) => {
                    self.handle_add_request(add_request);
                } None => {
                    // In case the received request was not identified, this will execute.
                    error!("Bad Request!");
                    self.handle_bad_request();
                }
            }
        } else {
            // Executes when the decoding of the message fails.
            error!("Failed to decode message");
            self.handle_bad_request();
        }

        Ok(())
    }

    /// Handle echo requests by echoing back the same message.
    ///
    /// # Arguments
    /// - `echo_message` The message received from the client.
    fn handle_echo_request(&mut self, echo_message: EchoMessage) {
        // If the received request was simply an echo request, send the message back
        info!("Received Echo Request: {}", echo_message.content);

        // Create the response
        let response = ServerMessage {
            message: Some(server_message::Message::EchoMessage(echo_message))
        };

        self.send_response(response);
    }

    /// Handle the add requests by adding the two integers within the request then sending the result.
    ///
    /// # Arguments
    /// - `add_request` The client request containing the two integers to be added.
    fn handle_add_request(&mut self, add_request: AddRequest) {
        // If the received request is an add request, perform the operation.
        info!("Received Add Request: {} + {}", add_request.a, add_request.b);

        // Perform the request.
        let add_response = AddResponse {
            result: add_request.a + add_request.b
        };

        // Create the response.
        let response = ServerMessage {
            message: Some(server_message::Message::AddResponse(add_response))
        };

        self.send_response(response);
    }

    /// Handle a bad request sent by the client.
    fn handle_bad_request(&mut self) {
        let response = ServerMessage {
            message: Some(server_message::Message::ErrorMessage(ErrorMessage {
                content: "Bad Request!".to_string(),
            })),
        };
        self.send_response(response);
    }

    /// Send the a response message to the client.
    ///
    /// # Arguments
    /// - `response` The server message sent to hte client.
    fn send_response(&mut self, response: ServerMessage) {
        let payload = response.encode_to_vec();
        self.stream.write_all(&payload).expect("Failed to send response");
        self.stream.flush().expect("Failed to flush stream");
    }
}

pub struct Server {
    listener: TcpListener,
    is_running: Arc<AtomicBool>,
    // Use thread a thread pool instead of spawning a new thread
    // for each client for performance optimizations.
    thread_pool: ThreadPool,
    // Used to track if there are any active clients.
    active_clients: Arc<Mutex<Vec<TcpStream>>>,
}

impl Server {
    /// Creates a new server instance
    ///
    /// # Arguments
    /// - `addr` The ip address for the server.
    ///
    /// # Returns
    /// - Ok    upon successful message decoding and handling.
    /// - Err   when either the decoding or the handling fails.
    pub fn new(addr: &str) -> io::Result<Self> {
        let listener = TcpListener::bind(addr)?;
        let is_running = Arc::new(AtomicBool::new(false));
        let thread_pool = ThreadPool::new(15);
        let active_clients = Arc::new(Mutex::new(Vec::new()));
        Ok(Server {
            listener,
            is_running,
            thread_pool,
            active_clients,
        })
    }

    /// Runs the server, listening for incoming connections and handling them
    pub fn run(&self) -> io::Result<()> {
        // Set the server as running
        self.is_running.store(true, Ordering::SeqCst);
        info!("Server is running on {}", self.listener.local_addr()?);

        // Set the listener to non-blocking mode
        self.listener.set_nonblocking(true)?;

        while self.is_running.load(Ordering::SeqCst) {
            match self.listener.accept() {
                Ok((stream, addr)) => {
                    info!("New client connected: {}", addr);
                    // Add the client to the list of active clients.
                    {
                        self.active_clients.lock().unwrap().push(stream.try_clone().unwrap());
                    } // Lock is released here.

                    // Make a clone of the is_running attribute to be used within the threads.
                    let is_running = self.is_running.clone();

                    // Make a clone of the active_clients attribute to be used within the threads.
                    let active_clients = self.active_clients.clone();
                    // Create a thread for each client request.
                    self.thread_pool.execute( move || {
                        // Create a client instance.
                        let mut client = Client::new(stream);
                        // The thread will loop indefinetly until the serverr shuts down or an error occurs.
                        while is_running.load(Ordering::SeqCst) {
                            if let Err(e) = client.handle() {
                                error!("Error handling client: {}", e);
                                break;
                            }
                        }

                        // Remove the client from the list of active clients.
                        // This variable is shared across threads so a mutex must be used.
                        {
                            active_clients.lock().unwrap().retain(|s| s.peer_addr().unwrap() != addr);
                        } // Lock is released here.
                    });
                }

                Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                    // If there are no incoming connections, sleep for 100 ms.
                    thread::sleep(Duration::from_millis(100));
                }

                Err(e) => {
                    // Connection was not accepted succesfully.
                    error!("Error accepting connection: {}", e);
                }
            }
        }

        info!("Server stopped.");
        Ok(())
    }

    /// Send an error to all clients that are still active of the shut down.
    pub fn notify_clients_of_shutdown(&self) {
        // This variable is shared across threads so a mutex must be used.
        let clients = self.active_clients.lock().unwrap();

        // Iterate over the clients that are still running.
        for mut client in clients.iter() {
            // Create a server shut down message to the clients.
            let shutdown_message = ServerMessage {
                message: Some(server_message::Message::ErrorMessage(ErrorMessage {
                    content: "Server is shutting down.".to_string(),
                })),
            };

            // Send the message over the network.
            let payload = shutdown_message.encode_to_vec();
            if let Err(e) = client.write_all(&payload) {
                warn!("Failed to notify client: {}", e);
            }
        }
    }

    /// Stops the server by setting the `is_running` flag to `false`
    pub fn stop(&self) {
        if self.is_running.load(Ordering::SeqCst) {
            // Notify active clients of the shut down.
            info!("Server stopped, notifying clients...");
            self.notify_clients_of_shutdown();

            // Shutdown the server.
            self.is_running.store(false, Ordering::SeqCst);

            // Join all threads in the thread pool.
            self.thread_pool.join();

            info!("Shutdown signal sent.");
        } else {
            warn!("Server was already stopped or not running.");
        }
    }
}
