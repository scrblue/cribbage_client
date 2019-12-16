extern crate cribbage;
extern crate serde;
use serde::{Deserialize, Serialize};
use std::io;
use std::io::{Read, Write};
use std::net;
use std::str;
use std::sync::mpsc;
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

    // TODO Get some kind of error handling on this all
    let mut connection_attemps: u8 = 0;

    while connection_attemps < 10 {
        if let mut stream = net::TcpStream::connect(&ip) {
            connection_attemps = 11;
            println!("Connected");
            read_message(&mut stream.unwrap(), name.to_string());
        } /*else {
              connection_attemps += 1;
              if connection_attemps == 1 {
                  print!("Failed to connect; retrying");
              } else if connection_attemps == 10 {
                  println!(".");
              } else {
                  print!(".");
              }
              thread::sleep(Duration::from_secs(1));
          }*/
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

    // A simple denial for when the player is given a yes/no choice
    Denial,

    // The name the client wishes to be known by for the duration of the game
    // TODO A way to determine this with user authentication for eventual lobby and account system
    Name(String),

    // That one or two cards are discarded by the player
    DiscardOne { index: u8 },
    DiscardTwo { index_one: u8, index_two: u8 },

    // That a given index has been played; as a hand is four cards, an index of 0 to 3 will
    // represent that card being played and a None will represent a go
    PlayTurn(Option<u8>),

    // That the included ScoreEvents have been given by the player for the most recent play
    PlayScore(Vec<cribbage::score::ScoreEvent>),
}

// Messages sent from the game model to the to the client over TCP
#[derive(PartialEq, Serialize, Deserialize, Clone)]
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

    // That the game is waiting for confirmation to deal the hand
    WaitDeal,

    // That cards are actively being dealt
    Dealing,

    // That the player's hand is the included vector
    DealtHand(Vec<cribbage::deck::Card>),

    // That the game is waiting for a discard selection of one or two cards
    WaitDiscardOne,
    WaitDiscardTwo,

    // That someone has discarded one or two cards
    DiscardPlacedOne(String),
    DiscardPlacedTwo(String),

    // That all discards have been placed
    AllDiscards,

    // That the game is waiting for confirmation to cut the starter card
    WaitCutStarter,

    // That the starter card has been cut and the name of the player who cut it and its value
    CutStarter(String, cribbage::deck::Card),

    // That the game is waiting to know whether the dealer calls nibs or not
    WaitNibs,

    // That the dealer has cut a jack and received two points
    Nibs,

    // That a player has played a card and the following ScoreEvents have been claimed
    CardPlayed {
        name: String,
        card: cribbage::deck::Card,
        scores: Vec<cribbage::score::ScoreEvent>,
    },

    // That the game is waiting for a player to place a card and that the valid indices are as
    // listed
    WaitPlay(Vec<u8>),

    // That the game is waiting for ScoreEvents for the previous play
    WaitPlayScore,

    // That the game rejected the scoring because there was an invalid ScoreEvent
    InvalidPlayScoring,

    // That the game has rejected the scoring because the scores are incomplete
    IncompletePlayScoring,

    // That the scores are as follows; contains a vector of pairs of names and scores
    ScoreUpdate(Vec<(String, u8)>),

    // That an error has occured
    Error(String),

    // That the client should not expect further messages from the game model
    Disconnect,
}

fn read_message(stream: &mut net::TcpStream, username: String) {
    let mut parsed_from_server: Option<GameToClient> = None;
    let mut last_message: Option<GameToClient> = None;

    while parsed_from_server != Some(GameToClient::Disconnect) {
        // Wait for a message from the server, parse it, and respond appropriately
        let mut message = [0 as u8; 256];
        stream.read(&mut message).unwrap();

        parsed_from_server = Some(bincode::deserialize(&message.to_vec()).unwrap());

        if parsed_from_server != last_message {
            match &parsed_from_server {
                Some(GameToClient::DeniedTableFull) => {
                    println!("The table is full");
                }

                Some(GameToClient::WaitName) => {
                    stream
                        .write(&bincode::serialize(&ClientToGame::Name(username.clone())).unwrap())
                        .unwrap();
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

                Some(GameToClient::WaitDeal) => {
                    println!("Press return to deal the hands");

                    let mut stdin = io::stdin();
                    let _ = stdin.read(&mut [0u8]).unwrap();
                    stream
                        .write(&bincode::serialize(&ClientToGame::Confirmation).unwrap())
                        .unwrap();
                }

                Some(GameToClient::Dealing) => {
                    println!("The hands are being dealt");
                }

                Some(GameToClient::DealtHand(hand)) => {
                    println!("Your hand is: {:?}", hand);
                }

                // TODO Fix DiscardPlaced messages reception
                Some(GameToClient::WaitDiscardOne) => {
                    // Spawn IO thread
                    let (transmitter, receiver) = mpsc::channel();
                    thread::spawn(|| {
                        listen_discards(1, transmitter);
                    });
                    // Loop until AllDiscards received
                    stream.set_nonblocking(true).unwrap();
                    let mut message_from_server: Option<GameToClient> = None;
                    while message_from_server != Some(GameToClient::AllDiscards) {
                        // Read message from server and check for DiscardPlacedOne and AllDiscards
                        let mut message = [0 as u8; 256];
                        match stream.read(&mut message) {
                            Ok(_) => {
                                message_from_server =
                                    Some(bincode::deserialize(&message.to_vec()).unwrap());
                            }
                            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                                message_from_server = None;
                            }
                            _ => println!("TCP read error"),
                        }

                        match &message_from_server {
                            Some(GameToClient::DiscardPlacedOne(name)) => {
                                println!("Received DiscardPlacedOne event");
                                if *name != username {
                                    println!("{} placed their card in the discards", name.clone());
                                } else {
                                    println!("You placed your card in the discards");
                                }
                            }

                            Some(GameToClient::AllDiscards) => {
                                println!("All discards have been placed")
                            }

                            _ => {}
                        }

                        // Check for transmission from IO thread
                        match receiver.try_recv() {
                            Ok(index) => {
                                stream.set_nonblocking(false).unwrap();
                                stream
                                    .write(
                                        &bincode::serialize(&ClientToGame::DiscardOne { index })
                                            .unwrap(),
                                    )
                                    .unwrap();
                            }
                            _ => {}
                        }
                    }
                }
                Some(GameToClient::WaitDiscardTwo) => {
                    // Spawn IO thread
                    let (transmitter, receiver) = mpsc::channel();
                    thread::spawn(|| {
                        listen_discards(2, transmitter);
                    });

                    let mut indices: Vec<u8> = Vec::new();

                    // Loop until AllDiscards received
                    stream.set_nonblocking(true).unwrap();
                    let mut message_from_server: Option<GameToClient> = None;
                    while message_from_server != Some(GameToClient::AllDiscards) {
                        // Read message from server and check for DiscardPlacedOne and AllDiscards
                        let mut message = [0 as u8; 256];
                        match stream.read(&mut message) {
                            Ok(_) => {
                                message_from_server =
                                    Some(bincode::deserialize(&message.to_vec()).unwrap());
                            }
                            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                                message_from_server = None;
                            }
                            _ => println!("TCP read error"),
                        }

                        match &message_from_server {
                            Some(GameToClient::DiscardPlacedTwo(name)) => {
                                if *name != username {
                                    println!("{} placed their card in the discards", name.clone());
                                } else {
                                    println!("You placed your card in the discards");
                                }
                            }

                            Some(GameToClient::AllDiscards) => {
                                println!("All discards have been placed")
                            }

                            _ => {}
                        }

                        // Check for transmission from IO thread
                        match receiver.try_recv() {
                            Ok(index) => {
                                indices.push(index);
                            }
                            _ => {}
                        }

                        if indices.len() == 2 {
                            stream.set_nonblocking(false).unwrap();
                            stream
                                .write(
                                    &bincode::serialize(&ClientToGame::DiscardTwo {
                                        index_one: indices[0],
                                        index_two: indices[1],
                                    })
                                    .unwrap(),
                                )
                                .unwrap();
                        }
                    }
                }

                Some(GameToClient::WaitCutStarter) => {
                    println!("Press return to cut the starter card");
                    let mut stdin = io::stdin();
                    let _ = stdin.read(&mut [0u8]).unwrap();
                    stream
                        .write(&bincode::serialize(&ClientToGame::Confirmation).unwrap())
                        .unwrap();
                }

                Some(GameToClient::CutStarter(name, card)) => {
                    if *name == username {
                        println!("You cut the starter card, {:?}", card);
                    } else {
                        println!("{} cut the starter card, {:?}", name, card);
                    }
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

        last_message = parsed_from_server.clone();
    }

    println!("The game has ended");
}

// Function to poll for discard selection and pass selections to the main thread such that the main thread can still receive DiscardPlacedOne and DiscardPlacedTwo selections
fn listen_discards(num_selections: u8, transmitter: mpsc::Sender<u8>) {
    if num_selections == 2 {
        let mut valid_input_one = false;
        let mut index_one = 6;
        let mut valid_input_two = false;
        let mut index_two = 6;

        while !valid_input_one {
            println!("Enter the index (0 - 5) of the first card to discard");
            let mut input = String::new();
            let stdin = io::stdin();
            stdin.read_line(&mut input).unwrap();
            let input: u8 = input.trim().parse().unwrap();
            if input < 6 {
                valid_input_one = true;
                index_one = input;
            }
        }

        transmitter.send(index_one).unwrap();

        while !valid_input_two {
            println!("Enter the index (0 - 5) of the second card to discard");
            let mut input = String::new();
            let stdin = io::stdin();
            stdin.read_line(&mut input).unwrap();
            let input: u8 = input.trim().parse().unwrap();
            if input < 6 && input != index_one {
                valid_input_two = true;
                index_two = input;
            }
        }

        transmitter.send(index_two).unwrap();
    } else {
        let mut valid_input = false;
        let mut index = 6;

        while !valid_input {
            println!("Enter the index (0 - 4) of the card to discard");
            let mut input = String::new();
            let stdin = io::stdin();
            stdin.read_line(&mut input).unwrap();
            let input: u8 = input.trim().parse().unwrap();
            if input < 6 {
                valid_input = true;
                index = input;
            }
        }

        transmitter.send(index).unwrap();
    }
}
