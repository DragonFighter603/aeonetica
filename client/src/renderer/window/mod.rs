pub mod events;

use std::sync::mpsc::Receiver;

use aeonetica_engine::log;
use crate::{renderer::context::Context};

extern crate glfw;
extern crate gl;
use glfw::{*, Window as GlfwWindow, Context as GlfwContext};

use super::Renderer;

pub(crate) struct Window {
    glfw_handle: Glfw,
    glfw_window: GlfwWindow,
    event_receiver: Receiver<(f64, WindowEvent)>,
    renderer: Renderer,
    context: Context
}

impl Window {
    const DEFAULT_WINDOW_WIDTH: u32 = 1280;
    const DEFAULT_WINDOW_HEIGHT: u32 = 720;
    const DEFAULT_WINDOW_TITLE: &'static str = "Aeonetica Game Engine";

    pub(crate) fn new(full_screen: bool, context: Context) -> Self {
        match glfw::init(glfw::FAIL_ON_ERRORS) {
            Ok(mut glfw) => {
                let (mut window, events) = glfw.with_primary_monitor(|glfw, monitor| {
                    glfw.create_window(
                    Self::DEFAULT_WINDOW_WIDTH,
                    Self::DEFAULT_WINDOW_HEIGHT,
                    Self::DEFAULT_WINDOW_TITLE,
                    if full_screen {
                        monitor.map_or(WindowMode::Windowed, |m| WindowMode::FullScreen(m))
                    } else {
                        WindowMode::Windowed
                    }
                )}).expect("Error creating GLFW window!");

                window.make_current();
                window.set_key_polling(true);

                gl::load_with(|s| glfw.get_proc_address_raw(s));
                gl::Viewport::load_with(|s| glfw.get_proc_address_raw(s));

                log!(r#"
==== OpenGL info ====
  -> Vendor: {}
  -> Renderer: {}
  -> Version: {}"#, 
                    unsafe { std::ffi::CStr::from_ptr(gl::GetString(gl::VENDOR) as *const i8).to_str().unwrap() },
                    unsafe { std::ffi::CStr::from_ptr(gl::GetString(gl::RENDERER) as *const i8).to_str().unwrap() },
                    unsafe { std::ffi::CStr::from_ptr(gl::GetString(gl::VERSION) as *const i8).to_str().unwrap() }
                );

                let renderer = Renderer::new();

                Self {
                    glfw_handle: glfw,
                    glfw_window: window,
                    event_receiver: events,
                    renderer,
                    context
                }
            },
            Err(err) => panic!("Error creating window: {err}!") 
        }
    }

    pub(crate) fn poll_events(&mut self) {
        self.glfw_handle.poll_events();
        for (_, event) in flush_messages(&self.event_receiver) {
            let event = events::Event::from_glfw(event);

            if let events::EventType::WindowClose() = event.typ() {
                self.glfw_window.set_should_close(true);
            }

            self.context.on_event(event);
        }
    }

    pub(crate) fn render(&mut self) {
        unsafe {
            gl::Viewport(0, 0, self.glfw_window.get_size().0, self.glfw_window.get_size().1);
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
            gl::ClearColor(0.1, 0.1, 0.2, 0.0);
        }
        
        // render here
        unsafe {
            self.renderer.render();
        }

        self.glfw_window.swap_buffers();
    }

    pub(crate) fn should_close(&self) -> bool {
        self.glfw_window.should_close()
    }
}