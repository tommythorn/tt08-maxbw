//mod encoding;
use rand::Rng;

use std::collections::VecDeque;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;

#[derive(Debug)]
enum Command {
    Idle,
    Sync,
    Write(u64, Vec<u8>), // XXX Width can only be 1, 2, 4, ..., 64; we could enforce with types
    Read(u8, u64),       // XXX Width can only be 1, 2, 4, ..., 64; we could enforce with types
    Stop,                // Sim-only
}

#[derive(Debug)]
enum Reply {
    Idle,
    Synced,
    Pause,
    Resume,
    Data(i8, Vec<u8>),
}

fn send<T: std::fmt::Debug>(prefix: &str, tx: &Sender<T>, msg: T) {
    println!("{prefix}Sending {msg:?}");
    tx.send(msg).unwrap();
}

fn try_receive<T>(rx: &Receiver<T>) -> Result<T, TryRecvError> {
    rx.try_recv()
}

fn receive<T>(rx: &Receiver<T>) -> T {
    rx.recv().unwrap()
}

fn main() {
    let (cmd_tx, cmd_rx) = mpsc::channel();
    let (reply_tx, reply_rx) = mpsc::channel();
    thread::spawn(move || memory_server(cmd_rx, reply_tx));
    client(cmd_tx, reply_rx);
}

fn client(cmd_tx: Sender<Command>, reply_rx: Receiver<Reply>) {
    let mut pending = VecDeque::new();
    let mut tag = 0;
    let mut paused = false;

    // Sending idle isn't required but reflects what would happen on hardware
    send("\t<- client  ", &cmd_tx, Command::Idle);

    // We need to start out in a known state
    send("\t<- client  ", &cmd_tx, Command::Sync);
    while !matches!(receive(&reply_rx), Reply::Synced) {}

    let mut n = 1;
    loop {
        if n < 10 {
            if !paused {
                let magic_number = rand::thread_rng().gen_range(1..14);
                let is_read = magic_number % 2 == 0;
                let length = 1 << (magic_number / 2);
                if is_read {
                    send(
                        "\t<- client  ",
                        &cmd_tx,
                        Command::Read(length, 42 + magic_number),
                    );
                    pending.push_front(n);
                    n += 1;
                    println!("  push, now: {pending:?}");
                } else {
                    send(
                        "\t<- client  ",
                        &cmd_tx,
                        Command::Write(42 + magic_number, vec![42u8; length.into()]),
                    );
                }
            }
        } else {
            send("\t<- client  ", &cmd_tx, Command::Stop);
            return;
        }

        // Handing any replies coming back
        for reply in reply_rx.try_iter() {
            match reply {
                Reply::Idle => {}
                Reply::Synced => {
                    println!("  syncing");
                    pending = VecDeque::new(); // XXX flush method?
                    tag = 0;
                    paused = false;
                }
                Reply::Pause => {
                    paused = true;
                }
                Reply::Resume => {
                    paused = false;
                }
                Reply::Data(delta, data) => {
                    let this = tag + delta as i32;
                    println!("\tClient: got #{this} {}", data.len());
                    //                println!("  popping from: {pending:?}");
                    pending.pop_back().unwrap();
                    tag += 1;
                }
            }
        }
        // thread::sleep(std::time::Duration::from_millis(100)); // block for 0.1 seconds
    }
}

fn memory_server(cmd_rx: Receiver<Command>, reply_tx: Sender<Reply>) {
    let mut tag = 0;
    //let mut pending_loads = VecDeque::new();

    // Sending idle isn't required but reflects what would happen on hardware
    send("server ->  ", &reply_tx, Reply::Idle);

    let mut pause_requested = false;
    loop {
        let magic_number = rand::thread_rng().gen_range(1..14000);
        if magic_number == 7 {
            if pause_requested {
                send("server ->  ", &reply_tx, Reply::Resume);
            } else {
                send("server ->  ", &reply_tx, Reply::Pause);
            }
            pause_requested = !pause_requested;
        }

        match Ok(receive(&cmd_rx)) {
            Ok(Command::Idle) => {}
            Ok(Command::Sync) => {
                tag = 0;
                // pending.flush
                send("server ->  ", &reply_tx, Reply::Synced);
            }
            Ok(Command::Write(a, d)) => {
                println!("server ->  write {d:x?} to {a:x}");
            }
            Ok(Command::Read(_w, _a)) => {
                send("server -> c ", &reply_tx, Reply::Data(0, vec![0u8]));
                tag += 1;
            }
            Ok(Command::Stop) => {
                return;
            }
            Err(TryRecvError::Empty) => {}
            Err(e) => {
                panic!("{e:?}");
            }
        }
    }
}
