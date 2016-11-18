extern crate clap;

use clap::Arg;
use clap::ArgMatches;
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::sync::mpsc::TryRecvError;


// This type isn't explicit in the crate, but implementors probably have one
//pub trait Runner;

pub trait Builder {
  type H: Handle;

  fn args<'a, 'b>() -> Vec<Arg<'a, 'b>> {
    Vec::new()
  }

  fn start<'a>(matches: &ArgMatches<'a>) -> Self::H;
}

pub trait Handle {
  fn block_until_finished(self);
  fn terminate(self);
}

pub struct BasicHandle {
  kill_sender: Sender<()>,
  thread_handle: JoinHandle<()>
}

// TODO: Provide a name so we know *what* failed
impl Handle for BasicHandle {
  fn block_until_finished(self) {
    self.thread_handle.join().expect("Runner thread panicked!");
  }

  fn terminate(self) {
    self.kill_sender.send(()).expect("Runner hung up unexpectedly!");
    self.block_until_finished();
  }
}

struct BasicRunner {
  kill_receiver: Receiver<()>
}

impl BasicRunner {
  pub fn new(kill_receiver: Receiver<()>) -> BasicRunner {
    BasicRunner {
      kill_receiver: kill_receiver
    }
  }

  /**
   * Run the web server until told to stop, or handle is dropped.
   */
  pub fn run<S: TickableService>(self, mut svc: S) {
    let mut running = true;

    while running {
      match self.kill_receiver.try_recv() {
        Ok(()) | Err(TryRecvError::Disconnected) => running = false,
        Err(TryRecvError::Empty) => ()
      }
      svc.tick();
    }

    svc.finalize();
  }
}

pub trait TickableService: Sized + Send + 'static {
  fn args<'a, 'b>() -> Vec<Arg<'a, 'b>> {
    Vec::new()
  }
  fn tick(&mut self) {
    println!("ticked");
    thread::sleep(Duration::from_millis(200));
  }

  fn finalize(self) {}
  fn build<'a>(args: &ArgMatches<'a>) -> Self;
}

impl<T> Builder for T where T: TickableService {
  type H = BasicHandle;

  fn start<'a>(args: &ArgMatches<'a>) -> BasicHandle {
    let (kill_sender, kill_receiver) = mpsc::channel();
    let service = Self::build(args);

    // Spin off
    let join_handle = thread::spawn(move || {
      let runner = BasicRunner::new(kill_receiver);
      runner.run(service);
    });

    BasicHandle {
      kill_sender: kill_sender,
      thread_handle: join_handle
    }
  }
}


#[cfg(test)]
mod tests {
  #[test]
  fn it_works() {
  }
}
