//mod encoding;
use rand::Rng;

use std::collections::VecDeque;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;

struct Endpoint<S: std::fmt::Debug, R: std::fmt::Debug> {
    tx: Sender<S>,
    rx: Receiver<R>,
    id: String,
}

impl<S: std::fmt::Debug, R: std::fmt::Debug> Endpoint<S, R> {
    fn new(tx: Sender<S>, rx: Receiver<R>, id: String) -> Self {
        Self { tx, rx, id }
    }

    fn send(&self, msg: S) {
        println!("{}:{msg:?} ->", self.id);
        self.tx.send(msg).unwrap();
    }

    fn log_received(&self, v: &R) {
        println!("{}: <- {v:?}", self.id);
    }
    fn try_receive(&self) -> Result<R, TryRecvError> {
        let r = self.rx.try_recv();
        if let Ok(v) = &r {
            self.log_received(v);
        }
        r
    }

    fn receive(&self) -> R {
        let v = self.rx.recv().unwrap();
        self.log_received(&v);
        v
    }
}

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

fn main() {
    let (cmd_tx, cmd_rx) = mpsc::channel();
    let (reply_tx, reply_rx) = mpsc::channel();
    let client_channels = Endpoint::new(cmd_tx, reply_rx, "client".into());
    let server_channels = Endpoint::new(reply_tx, cmd_rx, "server".into());
    thread::spawn(move || memory_server(server_channels));
    client(client_channels);
}

fn client(ep: Endpoint<Command, Reply>) {
    let mut pending = VecDeque::new();
    let mut read_tag = 0;
    let mut data_tag = 0;
    let mut paused = false;

    // Sending idle isn't required but reflects what would happen on hardware
    ep.send(Command::Idle);

    // We need to start out in a known state
    ep.send(Command::Sync);
    while !matches!(ep.receive(), Reply::Synced) {}

    let mut n = 1;
    loop {
        if n < 10 {
            if !paused {
                let magic_number = rand::thread_rng().gen_range(1..6);
                let is_read = magic_number % 2 == 0;
                let length = 1 << (magic_number / 2);
                if is_read {
                    ep.send(Command::Read(length, 42 + magic_number));
                    pending.push_front(read_tag);
                    n += 1;
                    read_tag += 1;
                    println!("  push, now: {pending:?}");
                } else {
                    ep.send(Command::Write(42 + magic_number, vec![1u8; length.into()]));
                }
            }
        } else {
            ep.send(Command::Stop);
            return;
        }

        // Handing any replies coming back
        while let Ok(reply) = ep.try_receive() {
            match reply {
                Reply::Idle => {}
                Reply::Synced => {
                    println!("  syncing");
                    pending = VecDeque::new(); // XXX flush method?
                    read_tag = 0;
                    data_tag = 0;
                    paused = false;
                }
                Reply::Pause => {
                    paused = true;
                }
                Reply::Resume => {
                    paused = false;
                }
                Reply::Data(delta, data) => {
                    let this = data_tag + delta as i32;
                    println!("\tClient: got #{this} {}", data.len());
                    //                println!("  popping from: {pending:?}");
                    let expected_tag = pending.pop_back().unwrap();
                    assert_eq!(this, expected_tag);
                    data_tag += 1;
                }
            }
        }
        // thread::sleep(std::time::Duration::from_millis(100)); // block for 0.1 seconds
    }
}

fn memory_server(ep: Endpoint<Reply, Command>) {
    let mut tag = 0;
    //let mut pending_loads = VecDeque::new();

    // Sending idle isn't required but reflects what would happen on hardware
    ep.send(Reply::Idle);

    let mut pause_requested = false;
    loop {
        let magic_number = rand::thread_rng().gen_range(1..14000);
        if magic_number == 7 {
            if pause_requested {
                ep.send(Reply::Resume);
            } else {
                ep.send(Reply::Pause);
            }
            pause_requested = !pause_requested;
        }

        match Ok(ep.receive()) {
            Ok(Command::Idle) => {}
            Ok(Command::Sync) => {
                tag = 0;
                // pending.flush
                ep.send(Reply::Synced);
            }
            Ok(Command::Write(_a, _d)) => { /*println!("server ->  write {d:x?} to {a:x}");*/ }
            Ok(Command::Read(_w, _a)) => {
                ep.send(Reply::Data(0, vec![0u8]));
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
