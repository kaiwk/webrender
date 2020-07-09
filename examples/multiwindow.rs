/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use gleam::gl;
use std::fs::File;
use std::io::Read;
use std::rc::Rc;
use webrender::api::*;
use webrender::api::units::*;
use webrender::DebugFlags;
use winit::dpi::LogicalSize;

struct Notifier {
    events_proxy: winit::EventsLoopProxy,
}

impl Notifier {
    fn new(events_proxy: winit::EventsLoopProxy) -> Notifier {
        Notifier { events_proxy }
    }
}

impl RenderNotifier for Notifier {
    fn clone(&self) -> Box<dyn RenderNotifier> {
        Box::new(Notifier {
            events_proxy: self.events_proxy.clone(),
        })
    }

    fn wake_up(&self) {
        #[cfg(not(target_os = "android"))]
        let _ = self.events_proxy.wakeup();
    }

    fn new_frame_ready(&self,
                       _: DocumentId,
                       _scrolled: bool,
                       _composite_needed: bool,
                       _render_time: Option<u64>) {
        self.wake_up();
    }
}

struct Window {
    events_loop: winit::EventsLoop, //TODO: share events loop?
    window: winit::Window,
    context: surfman::Context,
    device: surfman::Device,
    gl: Rc<dyn gl::Gl>,
    renderer: Option<webrender::Renderer>,
    name: &'static str,
    pipeline_id: PipelineId,
    document_id: DocumentId,
    epoch: Epoch,
    api: RenderApi,
    font_instance_key: FontInstanceKey,
}

impl Drop for Window {
    fn drop(&mut self) {
        self.device.destroy_context(&mut self.context).unwrap();
        self.renderer.take().unwrap().deinit();
    }
}

impl Window {
    fn new(name: &'static str, clear_color: ColorF) -> Self {
        let events_loop = winit::EventsLoop::new();
        let window_builder = winit::WindowBuilder::new()
            .with_title(name)
            .with_multitouch()
            .with_dimensions(LogicalSize::new(800., 600.));
        let window = window_builder.build(&events_loop).unwrap();

        let connection = surfman::Connection::from_winit_window(&window).unwrap();
        let widget = connection.create_native_widget_from_winit_window(&window).unwrap();
        let adapter = connection.create_adapter().unwrap();
        let mut device = connection.create_device(&adapter).unwrap();
        let (major, minor) = match device.gl_api() {
            surfman::GLApi::GL => (3, 2),
            surfman::GLApi::GLES => (3, 0),
        };
        let context_descriptor = device.create_context_descriptor(&surfman::ContextAttributes {
            version: surfman::GLVersion {
                major,
                minor,
            },
            flags: surfman::ContextAttributeFlags::ALPHA |
            surfman::ContextAttributeFlags::DEPTH |
            surfman::ContextAttributeFlags::STENCIL,
        }).unwrap();
        let mut context = device.create_context(&context_descriptor, None).unwrap();
        device.make_context_current(&context).unwrap();

        let gl = match device.gl_api() {
            surfman::GLApi::GL => unsafe {
                gl::GlFns::load_with(
                    |symbol| device.get_proc_address(&context, symbol) as *const _
                )
            },
            surfman::GLApi::GLES => unsafe {
                gl::GlesFns::load_with(
                    |symbol| device.get_proc_address(&context, symbol) as *const _
                )
            },
        };
        let gl = gl::ErrorCheckingGl::wrap(gl);

        let surface = device.create_surface(
            &context,
            surfman::SurfaceAccess::GPUOnly,
            surfman::SurfaceType::Widget { native_widget: widget },
        ).unwrap();
        device.bind_surface_to_context(&mut context, surface).unwrap();

        let device_pixel_ratio = window.get_hidpi_factor() as f32;

        let opts = webrender::RendererOptions {
            device_pixel_ratio,
            clear_color: Some(clear_color),
            ..webrender::RendererOptions::default()
        };

        let device_size = {
            let size = window
                .get_inner_size()
                .unwrap()
                .to_physical(device_pixel_ratio as f64);
            DeviceIntSize::new(size.width as i32, size.height as i32)
        };
        let notifier = Box::new(Notifier::new(events_loop.create_proxy()));
        let (renderer, sender) = webrender::Renderer::new(gl.clone(), notifier, opts, None, device_size).unwrap();
        let mut api = sender.create_api();
        let document_id = api.add_document(device_size, 0);

        let epoch = Epoch(0);
        let pipeline_id = PipelineId(0, 0);
        let mut txn = Transaction::new();

        let font_key = api.generate_font_key();
        let font_bytes = load_file("../wrench/reftests/text/FreeSans.ttf");
        txn.add_raw_font(font_key, font_bytes, 0);

        let font_instance_key = api.generate_font_instance_key();
        txn.add_font_instance(font_instance_key, font_key, 32.0, None, None, Vec::new());

        api.send_transaction(document_id, txn);

        Window {
            events_loop,
            window,
            device,
            context,
            renderer: Some(renderer),
            name,
            epoch,
            pipeline_id,
            document_id,
            api,
            font_instance_key,
            gl,
        }
    }

    fn tick(&mut self) -> bool {
        let mut do_exit = false;
        let my_name = &self.name;
        let renderer = self.renderer.as_mut().unwrap();
        let api = &mut self.api;

        self.events_loop.poll_events(|global_event| match global_event {
            winit::Event::WindowEvent { event, .. } => match event {
                winit::WindowEvent::CloseRequested |
                winit::WindowEvent::KeyboardInput {
                    input: winit::KeyboardInput {
                        virtual_keycode: Some(winit::VirtualKeyCode::Escape),
                        ..
                    },
                    ..
                } => {
                    do_exit = true
                }
                winit::WindowEvent::KeyboardInput {
                    input: winit::KeyboardInput {
                        state: winit::ElementState::Pressed,
                        virtual_keycode: Some(winit::VirtualKeyCode::P),
                        ..
                    },
                    ..
                } => {
                    println!("set flags {}", my_name);
                    api.send_debug_cmd(DebugCommand::SetFlags(DebugFlags::PROFILER_DBG))
                }
                _ => {}
            }
            _ => {}
        });
        if do_exit {
            return true
        }

        self.device.make_context_current(&self.context).unwrap();

        let device_pixel_ratio = self.window.get_hidpi_factor() as f32;
        let device_size = {
            let size = self
                .window
                .get_inner_size()
                .unwrap()
                .to_physical(device_pixel_ratio as f64);
            DeviceIntSize::new(size.width as i32, size.height as i32)
        };
        let layout_size = device_size.to_f32() / euclid::Scale::new(device_pixel_ratio);
        let mut txn = Transaction::new();
        let mut builder = DisplayListBuilder::new(self.pipeline_id, layout_size);
        let space_and_clip = SpaceAndClipInfo::root_scroll(self.pipeline_id);

        let bounds = LayoutRect::new(LayoutPoint::zero(), builder.content_size());
        builder.push_simple_stacking_context(
            bounds.origin,
            space_and_clip.spatial_id,
            PrimitiveFlags::IS_BACKFACE_VISIBLE,
        );

        builder.push_rect(
            &CommonItemProperties::new(
                LayoutRect::new(
                    LayoutPoint::new(100.0, 200.0),
                    LayoutSize::new(100.0, 200.0),
                ),
                space_and_clip,
            ),
            LayoutRect::new(
                LayoutPoint::new(100.0, 200.0),
                LayoutSize::new(100.0, 200.0),
            ),
            ColorF::new(0.0, 1.0, 0.0, 1.0));

        let text_bounds = LayoutRect::new(
            LayoutPoint::new(100.0, 50.0),
            LayoutSize::new(700.0, 200.0)
        );
        let glyphs = vec![
            GlyphInstance {
                index: 48,
                point: LayoutPoint::new(100.0, 100.0),
            },
            GlyphInstance {
                index: 68,
                point: LayoutPoint::new(150.0, 100.0),
            },
            GlyphInstance {
                index: 80,
                point: LayoutPoint::new(200.0, 100.0),
            },
            GlyphInstance {
                index: 82,
                point: LayoutPoint::new(250.0, 100.0),
            },
            GlyphInstance {
                index: 81,
                point: LayoutPoint::new(300.0, 100.0),
            },
            GlyphInstance {
                index: 3,
                point: LayoutPoint::new(350.0, 100.0),
            },
            GlyphInstance {
                index: 86,
                point: LayoutPoint::new(400.0, 100.0),
            },
            GlyphInstance {
                index: 79,
                point: LayoutPoint::new(450.0, 100.0),
            },
            GlyphInstance {
                index: 72,
                point: LayoutPoint::new(500.0, 100.0),
            },
            GlyphInstance {
                index: 83,
                point: LayoutPoint::new(550.0, 100.0),
            },
            GlyphInstance {
                index: 87,
                point: LayoutPoint::new(600.0, 100.0),
            },
            GlyphInstance {
                index: 17,
                point: LayoutPoint::new(650.0, 100.0),
            },
        ];

        builder.push_text(
            &CommonItemProperties::new(
                text_bounds,
                space_and_clip,
            ),
            text_bounds,
            &glyphs,
            self.font_instance_key,
            ColorF::new(1.0, 1.0, 0.0, 1.0),
            None,
        );

        builder.pop_stacking_context();

        txn.set_display_list(
            self.epoch,
            None,
            layout_size,
            builder.finalize(),
            true,
        );
        txn.set_root_pipeline(self.pipeline_id);
        txn.generate_frame();
        api.send_transaction(self.document_id, txn);

        let framebuffer_object = self
            .device
            .context_surface_info(&self.context)
            .unwrap()
            .unwrap()
            .framebuffer_object;
        self.gl.bind_framebuffer(gl::FRAMEBUFFER, framebuffer_object);
        assert_eq!(self.gl.check_frame_buffer_status(gleam::gl::FRAMEBUFFER), gl::FRAMEBUFFER_COMPLETE);

        renderer.update();
        renderer.render(device_size).unwrap();

        let mut surface = self.device.unbind_surface_from_context(&mut self.context).unwrap().unwrap();
        self.device.present_surface(&self.context, &mut surface).unwrap();
        self.device.bind_surface_to_context(&mut self.context, surface).unwrap();

        false
    }
}

fn main() {
    let mut win1 = Window::new("window1", ColorF::new(0.3, 0.0, 0.0, 1.0));
    let mut win2 = Window::new("window2", ColorF::new(0.0, 0.3, 0.0, 1.0));

    loop {
        if win1.tick() {
            break;
        }
        if win2.tick() {
            break;
        }
    }
}

fn load_file(name: &str) -> Vec<u8> {
    let mut file = File::open(name).unwrap();
    let mut buffer = vec![];
    file.read_to_end(&mut buffer).unwrap();
    buffer
}
