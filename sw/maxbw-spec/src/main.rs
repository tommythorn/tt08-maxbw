//mod encoding;
use rand::Rng;
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
    let server_channels = Endpoint::new(reply_tx, cmd_rx, "    server".into());
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
        if n < 10 {
            if !paused {
                let magic_number = rand::thread_rng().gen_range(1..6);
                let is_read = /*magic_number % 2 == 0*/ true;
                let length = 1 << (magic_number / 2);
                let a = rand::thread_rng().gen_range(1..1000u64);
                if is_read {
                    ep.send(Command::Read(length, a));
                    pending_loads.insert(read_tag, a);
                    read_tag += 1;
                    println!("client: pending loads: {pending_loads:?}");
                } else {
                    ep.send(Command::Write(42 + magic_number, vec![1u8; length.into()]));
                }
                n += 1;
            }
        } else if pending_loads.is_empty() {
            ep.send(Command::Stop);
            return;
        } else {
            ep.send(Command::Idle);
        }

        // Handing any replies coming back
        while let Ok(reply) = ep.try_receive() {
            match reply {
                Reply::Idle => {}
                Reply::Synced => {
                    todo!("Isn't tested yet");
                    /*
                    pending = VecDeque::new(); // XXX flush method?
                    read_tag = 0;
                    data_tag = 0;
                    paused = false;
                    */
                }
                Reply::Pause => {
                    paused = true;
                }
                Reply::Resume => {
                    paused = false;
                }
                Reply::Data(delta, data) => {
                    //let this = data_tag + delta as i32;
                    let this = delta as i32;
                    println!(
                        "client: got #{this} {}  (delta {})",
                        data.len(),
                        this - data_tag
                    );
                    match pending_loads.remove(&this) {
                        None => panic!("client: got data for a non-pending load #{this}"),
                        Some(a) => println!("client: got {data:?} for load from address {a}"),
                    }
                    data_tag += 1;
                }
            }
        }
        thread::sleep(std::time::Duration::from_millis(100)); // block for 0.1 seconds
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
            println!(
                "    server: has {} pending loads: {:?}",
                pending_loads.len(),
                pending_loads
            );
            let i = rand::thread_rng().gen_range(0..pending_loads.len());
            println!("    server: randomly processing the {i}th pending load");
            let target_load_tag: isize = *(pending_loads.keys().nth(i).unwrap());
            let (tag2, (w, a)) = pending_loads.remove_entry(&target_load_tag).unwrap();
            assert_eq!(target_load_tag, tag2);
            println!("    server: now processing load #{target_load_tag} {w}B at {a}");
            let delta = /*target_load_tag - tag + 1*/ target_load_tag;
            assert_eq!(delta, isize::from(delta as i8));
            println!("    server: delta {}", target_load_tag - data_tag);
            ep.send(Reply::Data(delta as i8, vec![0u8; w as usize]));
            data_tag += 1;
        }
        match Ok(ep.receive()) {
            Ok(Command::Idle) => {}
            Ok(Command::Sync) => {
                tag = 0;
                // pending.flush
                ep.send(Reply::Synced);
            }
            Ok(Command::Write(_a, _d)) => { /*println!("server ->  write {d:x?} to {a:x}");*/ }
            Ok(Command::Read(w, a)) => {
                pending_loads.insert(tag, (w, a));
                println!("    server: pending loads {pending_loads:?}");
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
