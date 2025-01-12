pub mod r#struct;
pub mod math;
pub mod camera;
pub mod texture;
pub mod snake;




use r#struct::State;
use winit::{
    dpi::LogicalSize, event::*, event_loop::{ControlFlow, EventLoop}, window::WindowBuilder
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
    
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
    .with_title("Snake")
    .with_inner_size(LogicalSize::new(800.0, 800.0))
    .with_resizable(false)
    .build(&event_loop).unwrap();
    window.set_resizable(false);
    let mut state = State::new(window).await;

    event_loop.run(move |event, _, control_flow| match event {
        Event::RedrawRequested(window_id) if window_id == state.window().id() => {
            state.update();
            match state.render() {
                Ok(_) => {}
                Err(wgpu::SurfaceError::Lost) => {state.resize(state.size)}
                Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                Err(e) => eprintln!("{:?}", e)
            }
        }
        Event::WindowEvent {
            ref event,
            window_id,
        } if window_id == state.window.id() => if !state.input(event) {
            match event {
                WindowEvent::CloseRequested
                | WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            state: ElementState::Pressed,
                            virtual_keycode: Some(VirtualKeyCode::Escape),
                            ..
                        },
                        
                    ..
                } => {
                    
                    *control_flow = ControlFlow::Exit
                },

                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                    // new_inner_size is &&mut so we have to dereference it twice
                    state.resize(**new_inner_size);
                }
                _ => {}
            }
        }
        Event::MainEventsCleared => {
            state.window().request_redraw();
        }
    _ => {}
});
}












