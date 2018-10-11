#![feature(duration_as_u128)]

use glutin::GlContext;
use std::{
    collections::VecDeque,
    env, ffi, fs, mem, ptr,
    time::{Duration, Instant},
};

const VERTEX_SHADER_SOURCE: &str = r#"
#version 330 core
layout (location = 0) in vec3 aPos;

out vec2 TexPos;

void main()
{
    TexPos = aPos.xy + 0.5;
    gl_Position = vec4(aPos.xy * 2, aPos.z, 1.0);
}
"#;

fn main() {
    let frag_path = env::args().skip(1).next().expect("Missing fragment shader");

    let mut events_loop = glutin::EventsLoop::new();
    let (window_width, window_height) = (800, 600);
    let window = glutin::WindowBuilder::new()
        .with_title("Hello world!")
        .with_dimensions((window_width, window_height).into());
    let context = glutin::ContextBuilder::new().with_vsync(false);
    let gl_window = glutin::GlWindow::new(window, context, &events_loop).unwrap();

    unsafe {
        gl_window.make_current().unwrap();
    }

    gl::load_with(|symbol| gl_window.get_proc_address(symbol) as *const _);

    let create_program = |frag_src: &str| {
        let create_shader = |src: &str, kind: u32| unsafe {
            let vertex_shader = gl::CreateShader(kind);
            let source = ffi::CString::new(src).unwrap();
            gl::ShaderSource(
                vertex_shader,
                1,
                [source.as_ptr()].as_ptr() as _,
                ptr::null() as *const _,
            );
            gl::CompileShader(vertex_shader);

            let mut success = mem::uninitialized();
            gl::GetShaderiv(vertex_shader, gl::COMPILE_STATUS, &mut success);
            if success == 0 {
                let mut error_log_size = 512;
                let mut buffer: Vec<u8> = Vec::with_capacity(error_log_size as usize);
                gl::GetShaderInfoLog(
                    vertex_shader,
                    error_log_size,
                    &mut error_log_size,
                    buffer.as_mut_ptr() as *mut _,
                );
                buffer.set_len(error_log_size as usize);
                let error_msg =
                    String::from_utf8(buffer).expect("error message could not be turned into utf8");
                println!("Error while compiling shader");
                for line in error_msg.lines() {
                    println!("{}", line);
                }
                return Err(());
            }

            Ok(vertex_shader)
        };

        let vertex_shader = create_shader(VERTEX_SHADER_SOURCE, gl::VERTEX_SHADER)?;
        let fragment_shader = create_shader(frag_src, gl::FRAGMENT_SHADER)?;

        let shader_program = unsafe {
            let shader_program = gl::CreateProgram();
            gl::AttachShader(shader_program, vertex_shader);
            gl::AttachShader(shader_program, fragment_shader);
            gl::LinkProgram(shader_program);

            let success = {
                gl::LinkProgram(shader_program);

                let mut success = mem::uninitialized();
                gl::GetProgramiv(shader_program, gl::LINK_STATUS, &mut success);
                success
            };

            if success == 0 {
                let mut error_log_size = 512;
                let mut buffer: Vec<u8> = Vec::with_capacity(error_log_size as usize);
                gl::GetProgramInfoLog(
                    shader_program,
                    error_log_size,
                    &mut error_log_size,
                    buffer.as_mut_ptr() as *mut _,
                );
                buffer.set_len(error_log_size as usize);
                let error_msg = String::from_utf8(buffer);
                for line in error_msg.unwrap().lines() {
                    println!("{}", line);
                }
                return Err(());
            }

            shader_program
        };

        unsafe {
            gl::DeleteShader(vertex_shader);
            gl::DeleteShader(fragment_shader);
        }

        Ok(shader_program)
    };

    let mut fragment_src = fs::read_to_string(&frag_path).expect("failed to find fragment src");
    let mut shader_program = create_program(&fragment_src).expect("failed to create program");

    let verticies = [
        [0.5, 0.5, 0.0],
        [0.5, -0.5, 0.0],
        [-0.5, -0.5, 0.0],
        [-0.5, 0.5, 0.0_f32],
    ];
    let indicies = [[0, 1, 3], [1, 2, 3]];

    let vao = unsafe {
        let mut vbo = 0;
        let mut vao = 0;
        let mut ebo = 0;

        gl::GenVertexArrays(1, &mut vao);
        gl::GenBuffers(1, &mut vbo);
        gl::GenBuffers(1, &mut ebo);

        gl::BindVertexArray(vao);

        gl::BindBuffer(gl::ARRAY_BUFFER, vao);
        gl::BufferData(
            gl::ARRAY_BUFFER,
            mem::size_of_val(&verticies) as _,
            verticies.as_ptr() as _,
            gl::STATIC_DRAW,
        );

        gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ebo);
        gl::BufferData(
            gl::ELEMENT_ARRAY_BUFFER,
            mem::size_of_val(&indicies) as _,
            indicies.as_ptr() as _,
            gl::STATIC_DRAW,
        );

        gl::VertexAttribPointer(
            0,
            3,
            gl::FLOAT,
            gl::FALSE,
            3 * mem::size_of::<f32>() as i32,
            ptr::null(),
        );
        gl::EnableVertexAttribArray(0);

        vao
    };

    let start = Instant::now();
    let mut last = start;
    let mut last_check = start;
    let mut quit = false;
    let mut micro_queue = VecDeque::new();

    while !quit {
        let now = Instant::now();
        let delta = now.duration_since(last).as_micros();
        last = now;
        let total_delta = now.duration_since(start).as_millis();

        micro_queue.push_back(delta);
        if micro_queue.len() >= 60 {
            micro_queue.pop_front();
        }

        let avg_micro: f64 =
            micro_queue.iter().fold(0.0, |sum, a| sum + *a as f64) / micro_queue.len() as f64;

        if now.duration_since(last_check) < Duration::from_millis(1000) {
            let avg_ms = avg_micro / 1000.0;
            gl_window.set_title(&format!("{:.5}fps / {:.5}ms", 1000.0 / avg_ms, avg_ms));
            last_check = now;
            let new_fragment_src =
                fs::read_to_string(&frag_path).expect("failed to find fragment src");
            if new_fragment_src != fragment_src {
                fragment_src = new_fragment_src;
                match create_program(&fragment_src) {
                    Ok(sp) => {
                        shader_program = sp;
                    }
                    Err(()) => {}
                }
            }
        }

        events_loop.poll_events(|e| match e {
            glutin::Event::DeviceEvent { event, .. } => match event {
                _ => {}
            },
            glutin::Event::WindowEvent { event: e, .. } => match e {
                glutin::WindowEvent::CloseRequested => {
                    quit = true;
                }
                glutin::WindowEvent::KeyboardInput { input, .. } => {
                    if let Some(keycode) = input.virtual_keycode {
                        use glutin::VirtualKeyCode as Kc;

                        match keycode {
                            Kc::Escape => {
                                quit = true;
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            },
            _ => {}
        });

        unsafe {
            gl::ClearColor(0.2, 0.3, 0.3, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);

            gl::UseProgram(shader_program);

            let get_location = |name| {
                gl::GetUniformLocation(
                    shader_program,
                    ffi::CString::new(name)
                        .expect("unable to create a CString from passes str")
                        .as_ptr(),
                )
            };

            let time_location = get_location("time");
            gl::Uniform1f(time_location, (total_delta as f64 / 1000.0) as f32);

            gl::BindVertexArray(vao);
            gl::DrawElements(gl::TRIANGLES, 6, gl::UNSIGNED_INT, ptr::null());

            gl_window.swap_buffers().expect("unable to swap buffers");
        }
    }
}
