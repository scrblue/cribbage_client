extern crate cribbage;
extern crate serde;
use serde::{Deserialize, Serialize};
use std::io;
use std::io::{Read, Write};
use std::net;
use std::str;
use std::thread;
use std::time::Duration;

fn main() {
    let mut name = String::new();
    // TODO Get user account system and lobby and use username from there
    // There should be no technical issues if two players have the same name, but output would get
    // confusing
    println!("Enter your desired username");
    io::stdin()
        .read_line(&mut name)
        .expect("Failed to read input");
    let name = name.trim();

    // TODO Get ip and port from the lobby
    // Allows the user to pick the server to connect to
    let mut ip = String::new();
    println!("Enter the IP address of the server");
    io::stdin()
        .read_line(&mut ip)
        .expect("Failed to read input");
    let ip = ip.trim();

    println!("Trying to connect to ip {}", ip);

    let mut connection_attemps: u8 = 0;

    while connection_attemps < 10 {
        if let mut stream = net::TcpStream::connect(&ip) {
            connection_attemps = 11;
            println!("Connected");
            read_message(&mut stream.unwrap(), name.to_string());
        } else {
            connection_attemps += 1;
            if connection_attemps == 1 {
                print!("Failed to connect; retrying");
            } else if connection_attemps == 10 {
                println!(".");
            } else {
                print!(".");
            }
            thread::sleep(Duration::from_secs(1));
        }
    }

    println!("Disconnected or failed to connect to server");
}

// The messages sent from the client to the client handler thread over TCP
#[derive(PartialEq, Serialize, Deserialize)]
pub enum ClientToGame {
    // A message to initiate communication between the client thread and the game thread and to
    // indicate that the client thread is ready to receive requests
    Greeting,

    // A simple confirmation from the client to continue the game model progression
    Confirmation,

    // The name the client wishes to be known by for the duration of the game
    // TODO A way to determine this with user authentication for eventual lobby and account system
    Name(String),
}

// Messages sent from the game model to the to the client over TCP
#[derive(PartialEq, Serialize, Deserialize)]
pub enum GameToClient {
    // Message indicating that the maximum number of cliets that can play have already joined
    DeniedTableFull,

    // That the game model is requesting the name that the client will go by
    WaitName,

    // That a player has successfully joined the game
    PlayerJoinNotification {
        name: String,
        number: u8,
        of: u8,
    },

    // That the model is waiting for a confirmation event to process that player's initial cut
    WaitInitialCut,

    // That the named player has cut the specified card in their initial cut
    InitialCutResult {
        name: String,
        card: cribbage::deck::Card,
    },

    // That the named player has been decided to be the first dealer as per the cut
    InitialCutSuccess(String),

    // That the cut has resulted in a tie and that it must be redone
    InitialCutFailure,

    // That an error has occured
    Error(String),

    // That the client should not expect further messages from the game model
    Disconnect,
}

fn read_message(stream: &mut net::TcpStream, username: String) {
    let mut parsed_from_server: Option<GameToClient> = None;
    while parsed_from_server != Some(GameToClient::Disconnect) {
        // Wait for a message from the server, parse it, and respond appropriately
        let mut message = [0 as u8; 50];
        stream.read(&mut message).unwrap();

        parsed_from_server = Some(bincode::deserialize(&message.to_vec()).unwrap());

        match &parsed_from_server {
            Some(GameToClient::DeniedTableFull) => {
                println!("The table is full");
            }

            Some(GameToClient::WaitName) => {
                stream.write(&bincode::serialize(&ClientToGame::Name(username.clone())).unwrap());
            }

            Some(GameToClient::PlayerJoinNotification { name, number, of }) => {
                println!("{} has joined the game; {} of {}", name, number, of);
            }

            Some(GameToClient::WaitInitialCut) => {
                println!("Press return to cut the deck");
                let mut stdin = io::stdin();
                let _ = stdin.read(&mut [0u8]).unwrap();
                stream
                    .write(&bincode::serialize(&ClientToGame::Confirmation).unwrap())
                    .unwrap();
            }

            // TODO Functionality to prevent two people with the same name
            Some(GameToClient::InitialCutResult { name, card }) => {
                if *name == username {
                    println!("You cut a {:?}", card);
                } else {
                    println!("{} cut a {:?}", name, card);
                }
            }

            Some(GameToClient::InitialCutSuccess(name)) => {
                if *name == username {
                    println!("You won the cut.");
                } else {
                    println!("{} won the cut.", name);
                }
            }

            Some(GameToClient::InitialCutFailure) => {
                println!("There was a tie; redoing the cut");
            }

            Some(GameToClient::Disconnect) => {
                println!("Game has ended");
            }

            Some(GameToClient::Error(string)) => {
                println!("Error from server: {}", string);
            }

            _ => panic!("Invalid packet from server"),
        }
    }

    println!("The game has ended");
}
