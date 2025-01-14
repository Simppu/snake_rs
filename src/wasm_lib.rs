
use wasm_bindgen::prelude::*;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
    keyboard::{KeyCode, PhysicalKey}
};
use winit::platform::web::WindowExtWebSys;
use winit::dpi::PhysicalSize;

use crate::r#struct::State;
use winit::dpi::LogicalSize;

#[cfg_attr(target_arch="wasm32", wasm_bindgen(start))]
pub async fn run() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    console_log::init_with_level(log::Level::Warn).expect("Couldn't initialize logger");
    
    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new()
    .with_title("Snake")
    .with_inner_size(LogicalSize::new(800.0, 800.0))
    .with_resizable(false)
    .build(&event_loop).unwrap();
    window.set_resizable(false);
    let mut state = State::new(&window).await;
    
    web_sys::window()
        .and_then(|win| win.document())
        .and_then(|doc| {
            let dst = doc.get_element_by_id("body")?;
            let canvas = web_sys::Element::from(window.canvas().unwrap());
            dst.append_child(&canvas).ok()?;
            Some(())
        })
        .expect("Couldn't append canvas to document body.");

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

