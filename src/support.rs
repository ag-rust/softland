extern crate cgmath;
use cgmath::*;

use gfx;
use gfx::Device;
use gfx_window_glutin;
use glutin;
use glutin::{ElementState, MouseButton, MouseScrollDelta, VirtualKeyCode, TouchPhase, WindowEvent};
use imgui::{ImGui, Ui, ImGuiKey};
use imgui_gfx_renderer::Renderer;
use std::time::Instant;

use game_time::{GameClock, FrameCounter, FrameCount};
use game_time::framerate::RunningAverageSampler;
use game_time::step;

use genmesh::generators::*;
use genmesh::{Vertices, Triangulate};

use color;
use gpu;

use noise::{Perlin, NoiseModule, Seedable};
use rand;
use rand::*;

use shader;
use state::State;

pub type ColorFormat = gfx::format::Rgba8;
pub type DepthFormat = gfx::format::DepthStencil;

#[derive(Copy, Clone, PartialEq, Debug, Default)]
struct MouseState {
    pos: (i32, i32),
    pressed: (bool, bool, bool),
    wheel: f32,
}

macro_rules! process_event {
    ($event:ident, $imgui:ident, $window:ident, $renderer:ident, $mouse_state:ident, $game:ident, $main_color:ident, $main_depth:ident) => (
        match $event {
            WindowEvent::Resized(_, _) => {
                gfx_window_glutin::update_views(&$window, &mut $main_color, &mut $main_depth);
                $renderer.update_render_target($main_color.clone());
            }
            WindowEvent::Closed => $game.quit = true,
            WindowEvent::KeyboardInput(state, _, code, _) => {
                let pressed = state == ElementState::Pressed;
                let player = &mut $game.player;
                let camera = &mut player.camera;
                match code {
                    Some(VirtualKeyCode::Tab) => $imgui.set_key(0, pressed),
                    Some(VirtualKeyCode::Left) => {
                        $imgui.set_key(1, pressed);
                        camera.pan_x(-player.move_speed);
                    }
                    Some(VirtualKeyCode::Right) => {
                        $imgui.set_key(2, pressed);
                        camera.pan_x(player.move_speed);
                    }
                    Some(VirtualKeyCode::Up) => {
                        $imgui.set_key(3, pressed);
                        camera.pan_y(player.move_speed);
                    }
                    Some(VirtualKeyCode::Down) => {
                        $imgui.set_key(4, pressed);
                        camera.pan_y(-player.move_speed);
                    }
                    Some(VirtualKeyCode::PageUp) => $imgui.set_key(5, pressed),
                    Some(VirtualKeyCode::PageDown) => $imgui.set_key(6, pressed),
                    Some(VirtualKeyCode::Home) => $imgui.set_key(7, pressed),
                    Some(VirtualKeyCode::End) => $imgui.set_key(8, pressed),
                    Some(VirtualKeyCode::Delete) => $imgui.set_key(9, pressed),
                    Some(VirtualKeyCode::Back) => $imgui.set_key(10, pressed),
                    Some(VirtualKeyCode::Return) => {
// 1. Tell imgui the key was pressed.
                        $imgui.set_key(11, pressed);

// 2. Update our state w/regard to chat input.
                        $game.chat_window_state.user_editing = state == ElementState::Released;
                    },
                    Some(VirtualKeyCode::Escape) => $game.quit = true,
                    Some(VirtualKeyCode::A) => {
                        $imgui.set_key(13, pressed);

                        camera.move_left(player.move_speed);
                    }
                    Some(VirtualKeyCode::C) => $imgui.set_key(14, pressed),
                    Some(VirtualKeyCode::D) => {
                        camera.move_right(player.move_speed);
                    }
                    Some(VirtualKeyCode::S) => {
                        camera.move_backward(player.move_speed);
                    }
                    Some(VirtualKeyCode::V) => $imgui.set_key(15, pressed),
                    Some(VirtualKeyCode::W) => {
                        camera.move_forward(player.move_speed);
                    }
                    Some(VirtualKeyCode::X) => $imgui.set_key(16, pressed),
                    Some(VirtualKeyCode::Y) => $imgui.set_key(17, pressed),
                    Some(VirtualKeyCode::Z) => $imgui.set_key(18, pressed),
                    Some(VirtualKeyCode::LControl) |
                    Some(VirtualKeyCode::RControl) => $imgui.set_key_ctrl(pressed),
                    Some(VirtualKeyCode::LShift) |
                    Some(VirtualKeyCode::RShift) => $imgui.set_key_shift(pressed),
                    Some(VirtualKeyCode::LAlt) |
                    Some(VirtualKeyCode::RAlt) => $imgui.set_key_alt(pressed),
                    Some(VirtualKeyCode::LWin) |
                    Some(VirtualKeyCode::RWin) => $imgui.set_key_super(pressed),
                    _ => {}
                }
            }
            WindowEvent::MouseMoved(x, y) => $mouse_state.pos = (x, y),
            WindowEvent::MouseInput(state, MouseButton::Left) => {
                $mouse_state.pressed.0 = state == ElementState::Pressed
            }
            WindowEvent::MouseInput(state, MouseButton::Right) => {
                $mouse_state.pressed.1 = state == ElementState::Pressed
            }
            WindowEvent::MouseInput(state, MouseButton::Middle) => {
                $mouse_state.pressed.2 = state == ElementState::Pressed
            }
            WindowEvent::MouseWheel(MouseScrollDelta::LineDelta(_, y), TouchPhase::Moved) => {
                $mouse_state.wheel = y
            }
            WindowEvent::MouseWheel(MouseScrollDelta::PixelDelta(_, y), TouchPhase::Moved) => {
                $mouse_state.wheel = y
            }
            WindowEvent::ReceivedCharacter(c) => $imgui.add_input_character(c),
            _ => ()
        }
    )
}

fn calculate_color(height: f32) -> [f32; 4] {
    let c = {
        if height > 8.0 {
            [0.9, 0.9, 0.9] // white
        } else if height > 0.0 {
            [0.7, 0.7, 0.7] // greay
        } else if height > -5.0 {
            [0.2, 0.7, 0.2] // green
        } else {
            [0.2, 0.2, 0.7] // blue
        }
    };
    [c[0], c[1], c[2], 1.0]
}

fn make_geometry(n: usize) -> (Vec<shader::Vertex>, Vec<u32>) {
    let seed = rand::thread_rng().gen();
    let plane = Plane::subdivide(256, 256);
    let perlin = Perlin::new().set_seed(seed);
    let vertexes: Vec<shader::Vertex> = plane
        .shared_vertex_iter()
        .take(n)
        .map(|v| {
            let pos = v.pos;
            let value = perlin.get(pos);
            let pos = [pos[0], pos[1], value, 1.0];
            shader::Vertex {
                pos: pos,
                color: calculate_color(value),
            }
        })
        .collect();

    let indices : Vec<u32> = plane
        .indexed_polygon_iter()
        .take(n)
        .triangulate()
        .vertices()
        .map(|i| i as u32)
        .collect();
    (vertexes, indices)
}

pub fn run_game<F: FnMut(&Ui, &mut State)>(
    title: &str,
    clear_color: [f32; 4],
    game: &mut State,
    mut build_ui: F,
) {
    let mut imgui = ImGui::init();

    let (w, h) = game.window_dimensions;
    let events_loop = glutin::EventsLoop::new();
    let builder = glutin::WindowBuilder::new()
        .with_title(title)
        .with_dimensions(w, h)
        .with_vsync();
    let (window, mut device, mut factory, mut main_color, mut main_depth) =
        gfx_window_glutin::init::<ColorFormat, DepthFormat>(builder, &events_loop);
    let mut encoder: gfx::Encoder<_, _> = factory.create_command_buffer().into();
    let mut renderer = Renderer::init(&mut imgui, &mut factory, main_color.clone())
        .expect("Failed to initialize renderer");

    configure_keys(&mut imgui);

    println!("making ...");
    let (plane_vertices, plane_indices) = make_geometry(90000);
    println!("done!");

    let (triangle_pso, cube_pso, generated_pso) = {
        let mut pso_factory = gpu::PsoFactory::new(&mut factory);
        let triangle_pso = pso_factory.triangle_list();
        let cube_pso = pso_factory.triangle_strip();
        let generated_pso = pso_factory.triangle_list();
        (triangle_pso, cube_pso, generated_pso)
    };

    let mut last_frame = Instant::now();
    let mut mouse_state = MouseState::default();

    let mut clock = GameClock::new();
    let mut counter = FrameCounter::new(60.0, RunningAverageSampler::with_max_samples(120));
    let mut sim_time;

    loop {
        sim_time = clock.tick(&step::FixedStep::new(&counter));
        counter.tick(&sim_time);
        game.framerate = sim_time.instantaneous_frame_rate();

        events_loop.poll_events(|glutin::Event::WindowEvent { event, .. }| {
            process_event!(
                event,
                imgui,
                window,
                renderer,
                mouse_state,
                game,
                main_color,
                main_depth
            );
        });

        let now = Instant::now();
        let delta = now - last_frame;
        let delta_s = delta.as_secs() as f32 + delta.subsec_nanos() as f32 / 1_000_000_000.0;
        last_frame = now;

        update_mouse(&mut imgui, &mut mouse_state);
        let size_points = window.get_inner_size_points().unwrap();
        let size_pixels = window.get_inner_size_pixels().unwrap();
        let ui = imgui.frame(size_points, size_pixels, delta_s);

        // Draw our scene
        //
        // 1. Clear the background.
        encoder.clear(&mut main_color, clear_color);

        // 2. Submit geometry to GPU.
        {
            let mut gpu = gpu::Gpu::new(&mut factory, &mut encoder, &mut main_color);

            let dimensions = (0.25, 0.25, 0.25);
            let rect_colors: [[f32; 4]; 8] = [
                color::RED,
                color::YELLOW,
                color::RED,
                color::YELLOW,
                color::RED,
                color::YELLOW,
                color::RED,
                color::YELLOW,
            ];
            
            let view = game.player.camera.compute_view();
            let angle = cgmath::Deg(sim_time.frame_number() as f32);

            // non-ui 2d stuffz
            let projection = Matrix4::identity();
            let rot = Matrix4::from_angle_x(angle) * Matrix4::from_angle_y(angle);
            let mmatrix = Matrix4::identity() * rot;
            let uv_matrix = projection * view * mmatrix;

            gpu.draw_cube(&cube_pso, &dimensions, &rect_colors, uv_matrix);
            
            let rot = Matrix4::from_angle_x(angle) * Matrix4::from_angle_z(angle);
            let mmatrix = Matrix4::identity() * rot;
            let colors = [color::BLACK, color::GREEN, color::BLUE];
            let radius = 0.15;
            let uv_matrix = projection * view * mmatrix;
            gpu.draw_triangle(&triangle_pso, radius, &colors, uv_matrix);

            let projection = {
                let (width, height) = game.window_dimensions;
                let aspect_ratio = width / height;
                let (near, far) = (0.1, 200.0);
                let fovy = cgmath::Deg(60.0);
                cgmath::perspective(fovy, aspect_ratio as f32, near, far)
            };

            let mmatrix = Matrix4::identity();
            let uv_matrix = projection * view * mmatrix;
            gpu.draw_triangle_from_vertices(&generated_pso, &plane_vertices, &plane_indices, uv_matrix);            
        }

        // 3. Construct our UI.
        build_ui(&ui, game);

        // 4. Draw our scene (both UI and geometry submitted via encoder).
        renderer.render(ui, &mut factory, &mut encoder).expect(
            "Rendering failed",
        );

        // 3) Flush our device and swap the buffers.
        encoder.flush(&mut device);
        window.swap_buffers().unwrap();
        device.cleanup();

        if game.quit {
            break;
        }
    }
}

fn configure_keys(imgui: &mut ImGui) {
    imgui.set_imgui_key(ImGuiKey::Tab, 0);
    imgui.set_imgui_key(ImGuiKey::LeftArrow, 1);
    imgui.set_imgui_key(ImGuiKey::RightArrow, 2);
    imgui.set_imgui_key(ImGuiKey::UpArrow, 3);
    imgui.set_imgui_key(ImGuiKey::DownArrow, 4);
    imgui.set_imgui_key(ImGuiKey::PageUp, 5);
    imgui.set_imgui_key(ImGuiKey::PageDown, 6);
    imgui.set_imgui_key(ImGuiKey::Home, 7);
    imgui.set_imgui_key(ImGuiKey::End, 8);
    imgui.set_imgui_key(ImGuiKey::Delete, 9);
    imgui.set_imgui_key(ImGuiKey::Backspace, 10);
    imgui.set_imgui_key(ImGuiKey::Enter, 11);
    imgui.set_imgui_key(ImGuiKey::Escape, 12);
    imgui.set_imgui_key(ImGuiKey::A, 13);
    imgui.set_imgui_key(ImGuiKey::C, 14);
    imgui.set_imgui_key(ImGuiKey::V, 15);
    imgui.set_imgui_key(ImGuiKey::X, 16);
    imgui.set_imgui_key(ImGuiKey::Y, 17);
    imgui.set_imgui_key(ImGuiKey::Z, 18);
}

fn update_mouse(imgui: &mut ImGui, mouse_state: &mut MouseState) {
    let scale = imgui.display_framebuffer_scale();
    imgui.set_mouse_pos(
        mouse_state.pos.0 as f32 / scale.0,
        mouse_state.pos.1 as f32 / scale.1,
    );
    imgui.set_mouse_down(
        &[
            mouse_state.pressed.0,
            mouse_state.pressed.1,
            mouse_state.pressed.2,
            false,
            false,
        ],
    );
    imgui.set_mouse_wheel(mouse_state.wheel / scale.1);
    mouse_state.wheel = 0.0;
}
