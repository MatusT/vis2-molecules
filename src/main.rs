//! This applications implements paper [Implicit Representation of Molecular Surfaces](https://www.researchgate.net/publication/254026672_Implicit_Representation_of_Molecular_Surfaces).
//!

mod application;
mod camera;
mod grid;
mod pipelines;
mod ui;
mod utils;

///
/// Main function responsible for:
/// - initialization of window and surface
/// - initialization of application itself
/// - event loop
///
fn main() {
    use iced_wgpu::{Primitive, Renderer, Settings, Target, Viewport};
    use iced_winit::{mouse, Cache, Clipboard, Size, UserInterface};
    use winit::{
        event,
        event::ModifiersState,
        event::WindowEvent,
        event_loop::{ControlFlow, EventLoop},
    };

    // Create event loop
    let event_loop = EventLoop::new();

    // Create window and a rendering surface
    let (window, size, surface) = {
        let window = winit::window::Window::new(&event_loop).unwrap();
        window.set_title("Implicit Representation of Molecular Surfaces");
        window.set_inner_size(winit::dpi::PhysicalSize { width: 1280, height: 720 });
        let size = window.inner_size();
        let surface = wgpu::Surface::create(&window);
        (window, size, surface)
    };

    // Initialize the application itself
    let mut application = futures::executor::block_on(application::Application::new(size.width, size.height, &surface));

    // Create the swapchain
    let sc_format = wgpu::TextureFormat::Bgra8UnormSrgb;
    let mut sc_desc = wgpu::SwapChainDescriptor {
        usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
        format: sc_format,
        width: size.width,
        height: size.height,
        present_mode: wgpu::PresentMode::Mailbox,
    };
    let mut swap_chain = application.device().create_swap_chain(&surface, &sc_desc);

    // Initialize GUI
    let mut events = Vec::new();
    let modifiers = ModifiersState::default();
    let mut cache = Some(Cache::default());
    let mut renderer = Renderer::new(application.device(), Settings::default());
    let mut output = (Primitive::None, mouse::Interaction::default());
    let clipboard = Clipboard::new(&window);
    let mut ui = ui::UserInterface::new();

    let mut ui_on = true;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            event::Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => {
                sc_desc.width = size.width;
                sc_desc.height = size.height;
                swap_chain = application.device().create_swap_chain(&surface, &sc_desc);

                application.resize(sc_desc.width, sc_desc.height);
            }
            // Gather window + device events
            event::Event::WindowEvent { event, .. } => {
                match event {
                    WindowEvent::KeyboardInput {
                        input:
                            event::KeyboardInput {
                                virtual_keycode: Some(event::VirtualKeyCode::Escape),
                                state: event::ElementState::Pressed,
                                ..
                            },
                        ..
                    }
                    | WindowEvent::CloseRequested => {
                        *control_flow = ControlFlow::Exit;
                    }
                    WindowEvent::KeyboardInput {
                        input:
                            event::KeyboardInput {
                                virtual_keycode: Some(event::VirtualKeyCode::U),
                                state: event::ElementState::Pressed,
                                ..
                            },
                        ..
                    } => {
                        ui_on = !ui_on;
                    }
                    _ => {}
                };

                let event = event.to_static().unwrap();

                // Send window event to the graphics scene
                application.window_event(&event);

                // Map window event to iced event
                if let Some(event) = iced_winit::conversion::window_event(&event, window.scale_factor(), modifiers) {
                    events.push(event);
                }
            }
            event::Event::DeviceEvent { event, .. } => {
                application.device_event(&event);
            }
            //
            event::Event::MainEventsCleared => {
                // We need to:
                // 1. Process events of our user interface.
                // 2. Update state as a result of any interaction.
                // 3. Generate a new output for our renderer.

                // First, we build our user interface.
                let mut user_interface = UserInterface::build(
                    ui.view(&application),
                    Size::new(sc_desc.width as f32, sc_desc.height as f32),
                    cache.take().unwrap(),
                    &mut renderer,
                );

                // Then, we process the events, obtaining messages in return.
                let messages = user_interface.update(events.drain(..), clipboard.as_ref().map(|c| c as _), &renderer);

                let user_interface = if messages.is_empty() {
                    // If there are no messages, no interactions we care about have
                    // happened. We can simply leave our user interface as it is.
                    user_interface
                } else {
                    // If there are messages, we need to update our state
                    // accordingly and rebuild our user interface.
                    // We can only do this if we drop our user interface first
                    // by turning it into its cache.
                    cache = Some(user_interface.into_cache());

                    // In this example, `Controls` is the only part that cares
                    // about messages, so updating our state is pretty
                    // straightforward.
                    for message in messages {
                        ui.update(message, &mut application);
                    }

                    // Once the state has been changed, we rebuild our updated
                    // user interface.
                    UserInterface::build(
                        ui.view(&application),
                        Size::new(sc_desc.width as f32, sc_desc.height as f32),
                        cache.take().unwrap(),
                        &mut renderer,
                    )
                };

                // Finally, we just need to draw a new output for our renderer,
                output = user_interface.draw(&mut renderer);

                // update our cache,
                cache = Some(user_interface.into_cache());

                // and request a redraw
                window.request_redraw();
            }
            //
            event::Event::RedrawRequested(_) => {
                let frame = swap_chain
                    .get_next_texture()
                    .expect("Timeout when acquiring next swap chain texture");

                // We draw the scene first
                application.render(&frame.view);

                // And then iced on top
                if ui_on {
                    let mut encoder = application
                        .device()
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
                    let viewport = Viewport::new(sc_desc.width, sc_desc.height);
                    let mouse_interaction = renderer.draw(
                        application.device(),
                        &mut encoder,
                        Target {
                            texture: &frame.view,
                            viewport: &viewport,
                        },
                        &output,
                        window.scale_factor(),
                        &[""],
                    );

                    // Then we submit the work
                    application.queue_mut().submit(&[encoder.finish()]);

                    // And update the mouse cursor
                    window.set_cursor_icon(iced_winit::conversion::mouse_interaction(mouse_interaction));
                }
            }
            _ => {}
        }
    });
}
