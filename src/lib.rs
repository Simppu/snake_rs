pub mod r#struct;
pub mod math;
pub mod camera;
pub mod texture;
pub mod snake;

#[cfg(target_arch="wasm32")] 
pub mod wasm_lib;



use r#struct::State;
use winit::{
    dpi::LogicalSize, event::*, event_loop::{ControlFlow, EventLoop}, keyboard::{KeyCode, PhysicalKey}, window::WindowBuilder
};

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum SnakeInputs {
    Up,
    Down,
    Left,
    Right,
    Stay
}

 

 


pub async fn run() {
    env_logger::init();
    
    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new()
    .with_title("Snake")
    .with_inner_size(LogicalSize::new(800.0, 800.0))
    .with_resizable(false)
    .build(&event_loop).unwrap();
    window.set_resizable(false);
    let mut state = State::new(&window).await;

    event_loop.run(move |event, control_flow| {
        match event {
            
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == state.window().id() => if !state.input(event) {
                match event {
                WindowEvent::CloseRequested
                | WindowEvent::KeyboardInput {
                    event:
                        KeyEvent {
                            state: ElementState::Pressed,
                            physical_key: PhysicalKey::Code(KeyCode::Escape),
                            ..
                        },
                    ..
                } => control_flow.exit(),
                WindowEvent::RedrawRequested => {
                    state.window().request_redraw();
                    
        
                    state.update();
                    match state.render() {
                        Ok(_) => {}
                        // Reconfigure the surface if it's lost or outdated
                        Err(
                            wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated,
                        ) => state.resize(state.size),
                        // The system is out of memory, we should probably quit
                        Err(wgpu::SurfaceError::OutOfMemory) => {
                            log::error!("OutOfMemory");
                            control_flow.exit();
                        }
        
                        // This happens when the a frame takes too long to present
                        Err(wgpu::SurfaceError::Timeout) => {
                            log::warn!("Surface timeout")
                        }
                    }
                },
                
                _ => {}
            }},
            _ => {}
        }
    }).unwrap();

}












