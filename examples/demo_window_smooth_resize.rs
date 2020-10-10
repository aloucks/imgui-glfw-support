use futures::executor::block_on;
use std::time::Instant;

fn main() {
    let mut glfw = glfw::init(glfw::FAIL_ON_ERRORS).expect("GLFW failed to init");
    glfw.window_hint(glfw::WindowHint::ClientApi(glfw::ClientApiHint::NoApi));

    let (width, height) = (1600, 900);

    let (mut window, _event_receiver) = glfw
        .create_window(width, height, "Hello, ImGui", glfw::WindowMode::Windowed)
        .expect("failed to create window");

    window.set_all_polling(true);

    let instance = wgpu::Instance::new(wgpu::BackendBit::all());

    let surface = unsafe { instance.create_surface(&window) };

    let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: Some(&surface),
    }))
    .unwrap();

    let (device, queue) = block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            features: wgpu::Features::empty(),
            limits: wgpu::Limits::default(),
            shader_validation: false,
        },
        None,
    ))
    .unwrap();

    let mut swap_chain_desc = wgpu::SwapChainDescriptor {
        usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
        format: wgpu::TextureFormat::Bgra8Unorm,
        width,
        height,
        present_mode: wgpu::PresentMode::Immediate,
    };

    let mut swap_chain = device.create_swap_chain(&surface, &swap_chain_desc);

    let mut imgui = imgui::Context::create();
    imgui.set_ini_filename(None);

    let mut glfw_platform = imgui_glfw_support::GlfwPlatform::init(&mut imgui);

    glfw_platform.attach_window(
        imgui.io_mut(),
        &window,
        imgui_glfw_support::HiDpiMode::Default,
    );

    // Adding platform clipboard integration is unsafe because the caller must ensure that
    // the window outlives the imgui context and that all imgui functions that may access
    // the clipboard are called from the main thread.
    unsafe {
        glfw_platform.set_clipboard_backend(&mut imgui, &window);
    }

    let clear_color = wgpu::Color {
        r: 0.1,
        g: 0.2,
        b: 0.3,
        a: 1.0,
    };

    let mut imgui_renderer =
        imgui_wgpu::Renderer::new(&mut imgui, &device, &queue, swap_chain_desc.format);

    let mut last_cursor = None;
    let mut last_frame_time = Instant::now();

    while !window.should_close() {
        let mut render_fn = |imgui: &mut imgui::Context,
                             window: &mut glfw::Window,
                             swap_chain: &mut wgpu::SwapChain| {
            let frame = match swap_chain.get_current_frame() {
                Ok(frame) => frame,
                Err(err) => {
                    eprintln!("get_next_texture timed out: {:?}", err);
                    return;
                }
            };
            let now = Instant::now();
            imgui.io_mut().update_delta_time(now - last_frame_time);
            last_frame_time = now;

            glfw_platform
                .prepare_frame(imgui.io_mut(), window)
                .expect("prepare_frame failed");

            let ui = imgui.frame();
            ui.show_demo_window(&mut true);

            let cursor = ui.mouse_cursor();
            if last_cursor != cursor {
                last_cursor = cursor;
                glfw_platform.prepare_render(&ui, window);
            }

            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                    attachment: &frame.output.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(clear_color),
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });
            imgui_renderer
                .render(ui.render(), &queue, &device, &mut render_pass)
                .expect("render failed");
            drop(render_pass);
            queue.submit(std::iter::once(encoder.finish()));
        };

        glfw.wait_events_timeout_unbuffered(0.1, |_, (_, event)| {
            glfw_platform.handle_event(imgui.io_mut(), &window, &event);
            match event {
                glfw::WindowEvent::Size(width, height) => {
                    swap_chain_desc.width = width as _;
                    swap_chain_desc.height = height as _;
                    swap_chain = device.create_swap_chain(&surface, &swap_chain_desc);
                }
                glfw::WindowEvent::Refresh => render_fn(&mut imgui, &mut window, &mut swap_chain),
                _ => {}
            }
            None
        });

        render_fn(&mut imgui, &mut window, &mut swap_chain);
    }
}
