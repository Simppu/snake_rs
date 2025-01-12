#[cfg(target_arch="x86_64")]
use rendering::run;


#[cfg(target_arch="wasm32")] 
use wasm_lib::run;







fn main() {
    pollster::block_on(run());
}
