use rendering::{process, SnakeInputs};
#[cfg(target_arch="x86_64")]
use rendering::run;

use tokio::join;
#[cfg(target_arch="wasm32")] 
use wasm_lib::run;

use tokio::sync::mpsc;





#[tokio::main]
async fn main() {
    let (input_sender, input_receiver) = mpsc::channel::<SnakeInputs>(32);
    let render = run(input_sender);
    let process = tokio::spawn(async move { pollster::block_on(process(input_receiver)) });
    

    join!(process, render).0.unwrap();
}
