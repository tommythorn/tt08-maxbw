//! Model of the MaxBW protocol
//!
//! Todo:
//! - Array based pending lists
//! - Flow control (to avoid overrunning buffer space and delta range)
//! - Message serialization

//mod encoding;
use rand::Rng;
use std::sync::mpsc::{self, Receiver, Sender};
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
        println!("{}{msg:?} ->", self.id);
        self.tx.send(msg).unwrap();
    }

    fn receive(&self) -> R {
        let v = self.rx.recv().unwrap();
        println!("{} <- {v:?}", self.id);
        v
    }
}

#[derive(Debug)]
enum Command {
    Idle,
    Sync,
    Write(u64, Vec<u8>), // XXX Width can only be 1, 2, 4, ..., 64; we could enforce with types
    Read(u8, u64),       // XXX Width can only be 1, 2, 4, ..., 64; we could enforce with types
    EndSim,              // Sim-only
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
    let client_channels = Endpoint::new(cmd_tx, reply_rx, "client: ".into());
    let server_channels = Endpoint::new(reply_tx, cmd_rx, "\t\tserver: ".into());
    thread::spawn(move || memory_server(server_channels));
    client(client_channels);
}

fn client(ep: Endpoint<Command, Reply>) {
    let mut pending_loads = std::collections::BTreeMap::new();
    let mut read_tag = 0;
    let mut data_tag = 0;
    let mut paused = false;

    // Sending idle isn't required but reflects what would happen on hardware
    ep.send(Command::Idle);

    // We need to start out in a known state
    ep.send(Command::Sync);
    while !matches!(ep.receive(), Reply::Synced) {}

    println!();

    let mut n = 1;
    loop {
        if !paused && n < 100 {
            let magic_number = rand::thread_rng().gen_range(1..6);
            let is_read = /*magic_number % 2 == 0*/ true;
            let length = 1 << (magic_number / 2);
            let a = rand::thread_rng().gen_range(1..1000u64);
            if is_read {
                pending_loads.insert(read_tag, a);
                read_tag += 1;
                ep.send(Command::Read(length, a));
                //println!("client: pending loads: {pending_loads:?}");
                n += 1;
            } else {
                ep.send(Command::Write(a, vec![1u8; length.into()]));
            }
        } else if pending_loads.is_empty() {
            ep.send(Command::EndSim);
            return;
        } else {
            ep.send(Command::Idle);
        }

        // Handle replies
        match ep.receive() {
            Reply::Idle => {}
            Reply::Synced => {
                pending_loads.clear();
                read_tag = 0;
                data_tag = 0;
                paused = false;
            }
            Reply::Pause => paused = true,
            Reply::Resume => paused = false,
            Reply::Data(delta, data) => {
                let this = data_tag + delta as i32;
                match pending_loads.remove(&this) {
                    None => panic!(
                        "client: got data for unknown load #{this} \
			 (delta {delta}, data_tag {data_tag}"
                    ),
                    Some(a) => println!(
                        "client: load #{this} {data:?} address {a} (delta {})",
                        this - data_tag
                    ),
                }
                data_tag += 1;
            }
        }
    }
}

fn memory_server(ep: Endpoint<Reply, Command>) {
    let mut tag = 0isize;
    let mut data_tag = 0isize;
    let mut pending_loads = std::collections::BTreeMap::new();

    // Sending idle isn't required but reflects what would happen on hardware
    ep.send(Reply::Idle);

    let mut pause_requested = false;
    loop {
        match ep.receive() {
            Command::Idle => {}
            Command::Sync => {
                tag = 0;
                data_tag = 0;
                pending_loads.clear();
                ep.send(Reply::Synced);
            }
            Command::Write(_a, _d) => {}
            Command::Read(w, a) => {
                pending_loads.insert(tag, (w, a));
                tag += 1;
            }
            Command::EndSim => return,
        }

        let magic_number = rand::thread_rng().gen_range(1..14000);
        if magic_number == 7 {
            if pause_requested {
                ep.send(Reply::Resume);
            } else {
                ep.send(Reply::Pause);
            }
            pause_requested = !pause_requested;
        }

        if !pending_loads.is_empty() && rand::thread_rng().gen_range(0..100) < 20 {
            /*println!(
                "\t\tserver: {} pending loads: {:?}",
                pending_loads.len(),
                pending_loads
            );*/
            let i = rand::thread_rng().gen_range(0..pending_loads.len());
            let target_load_tag: isize = *(pending_loads.keys().nth(i).unwrap());
            let (tag2, (w, a)) = pending_loads.remove_entry(&target_load_tag).unwrap();
            assert_eq!(target_load_tag, tag2);
            println!(
                "\t\tserver: processing load #{target_load_tag} {w}B at {a} \
		 (delta {} data_tag {data_tag})",
                target_load_tag - data_tag
            );
            let delta = target_load_tag - data_tag;
            assert_eq!(delta, isize::from(delta as i8)); // XXX Narrow that range
            ep.send(Reply::Data(delta as i8, vec![0u8; w as usize]));
            data_tag += 1;
        } else {
            ep.send(Reply::Idle);
        }
    }
}
