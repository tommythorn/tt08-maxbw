mod encoding;
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
}

#[derive(Debug)]
enum Reply {
    Idle,
    Synced,
    Pause,
    Resume,
    Data(i8, Vec<u8>),
}

fn send<T: std::fmt::Debug>(tx: &Sender<T>, msg: T) {
    println!("Sending {msg:?}");
    tx.send(msg).unwrap();
}

fn try_receive<T>(rx: &Receiver<T>) -> Result<T, TryRecvError> {
    rx.try_recv()
}

fn main() {
    let (cmd_tx, cmd_rx) = mpsc::channel();
    let (reply_tx, reply_rx) = mpsc::channel();

    // Let's spawn the client (~ CPU)
    thread::spawn(move || run_client(cmd_tx, reply_rx));
    run_memory_server(cmd_rx, reply_tx);
}

fn run_client(cmd_tx: Sender<Command>, reply_rx: Receiver<Reply>) {
    let mut pending = VecDeque::new();
    let mut tag = 0;
    let mut paused = false;

    // Sending idle isn't required but reflects what would happen on hardware
    send(&cmd_tx, Command::Idle);

    // We need to start out in a known state
    send(&cmd_tx, Command::Sync);
    while let Ok(reply) = try_receive(&reply_rx) {
        if matches!(reply, Reply::Synced) {
            break;
        }
    }

    let mut n = 1;
    loop {
        if n < 10 && !paused {
            let magic_number = rand::thread_rng().gen_range(1..14);
            let is_read = magic_number % 2 == 0;
            let length = 1 << (magic_number / 2);
            let cmd = if is_read {
                Command::Read(length, 42 + magic_number)
            } else {
                Command::Write(42 + magic_number, vec![42u8; length.into()])
            };
            send(&cmd_tx, cmd);
            pending.push_front(n);
            println!("  push, now: {pending:?}");
            n += 1;
        } else {
            panic!("stop");
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
                    println!("Client: got #{this} {data:?}");
                    println!("  popping from: {pending:?}");
                    pending.pop_back().unwrap();
                    tag += 1;
                }
            }
        }
        thread::sleep(std::time::Duration::from_millis(100)); // block for 0.1 seconds
    }
}

fn run_memory_server(cmd_rx: Receiver<Command>, reply_tx: Sender<Reply>) {
    let mut tag = 0;
    //let mut pending_loads = VecDeque::new();

    // Sending idle isn't required but reflects what would happen on hardware
    send(&reply_tx, Reply::Idle);

    loop {
        match try_receive(&cmd_rx) {
            Ok(Command::Idle) => {}
            Ok(Command::Sync) => {
                tag = 0;
                // pending.flush
                send(&reply_tx, Reply::Synced);
            }
            Ok(Command::Write(a, d)) => {
                println!("Server: write {d:x?} to {a:x}");
            }
            Ok(Command::Read(_w, _a)) => {
                send(&reply_tx, Reply::Data(0, vec![0u8]));
                tag += 1;
            }
            Err(TryRecvError::Empty) => {}
            Err(e) => {
                panic!("{e:?}");
            }
        }
    }
}
