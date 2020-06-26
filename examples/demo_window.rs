use std::time::Instant;

use futures::executor::block_on;

fn main() {
    let mut glfw = glfw::init(glfw::FAIL_ON_ERRORS).expect("GLFW failed to init");
    glfw.window_hint(glfw::WindowHint::ClientApi(glfw::ClientApiHint::NoApi));

    let (width, height) = (1600, 900);

    let (mut window, event_receiver) = glfw
        .create_window(width, height, "Hello, ImGui", glfw::WindowMode::Windowed)
        .expect("failed to create window");

    window.set_all_polling(true);

    let surface = wgpu::Surface::create(&window);

    let adapter = block_on(wgpu::Adapter::request(
        &wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
        },
        wgpu::BackendBit::PRIMARY,
    ))
    .unwrap();

    let (device, mut queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        extensions: wgpu::Extensions {
            anisotropic_filtering: false,
        },
        limits: wgpu::Limits::default(),
    }));

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

    let mut imgui_renderer = imgui_wgpu::Renderer::new(
        &mut imgui,
        &device,
        &mut queue,
        swap_chain_desc.format,
        Some(clear_color),
    );

    let mut last_cursor = None;
    let mut last_frame_time = Instant::now();

    while !window.should_close() {
        glfw.wait_events_timeout(0.1);

        let mut recreate_swap_chain = false;
        for (_timestamp, event) in event_receiver.try_iter() {
            glfw_platform.handle_event(imgui.io_mut(), &window, &event);
            match event {
                glfw::WindowEvent::Size(width, height) => {
                    swap_chain_desc.width = width as _;
                    swap_chain_desc.height = height as _;
                    recreate_swap_chain = true;
                }
                _ => {}
            }
        }
        if recreate_swap_chain {
            swap_chain = device.create_swap_chain(&surface, &swap_chain_desc);
        }

        let frame = swap_chain.get_next_texture().unwrap();
        last_frame_time = imgui.io_mut().update_delta_time(last_frame_time);

        glfw_platform
            .prepare_frame(imgui.io_mut(), &mut window)
            .expect("prepare_frame failed");

        let ui = imgui.frame();
        ui.show_demo_window(&mut true);

        let cursor = ui.mouse_cursor();
        if last_cursor != cursor {
            last_cursor = cursor;
            glfw_platform.prepare_render(&ui, &mut window);
        }

        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        imgui_renderer
            .render(ui.render(), &device, &mut encoder, &frame.view)
            .expect("render failed");
        queue.submit(&[encoder.finish()]);
    }
}
