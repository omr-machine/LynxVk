use std::time::Duration;

use sdl2::{event::Event, keyboard::Keycode};

// https://docs.rs/sdl3/0.14.36/sdl3/
// https://docs.rs/sdl2/latest/sdl2/

use crate::mess::ash_test;

pub fn ash_test_main() {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem
        .window("rust-sdl2 demo", 800, 600)
        .position_centered()
        .build()
        .unwrap();

    let x = ash_test::Core::new(&window).unwrap();

    let mut event_pump = sdl_context.event_pump().unwrap();

    let mut i = 0;
    let mut k = 0;
    'running: loop {
        i = (i + 1) % 255;

        if (i % 255) == 254 {
            k += 1;
        }

        if (k % 2) == 0 {
        } else {
        }

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                _ => {}
            }
        }

        // x.
        ::std::thread::sleep(Duration::new(0, (1_000_000u32 as f64 / 0.1) as u32));
    }

    // sdl_test();
}

mod sdl2_test {
    use sdl2::event::Event;
    use sdl2::keyboard::Keycode;
    use sdl2::pixels::Color;
    use std::time::Duration;
    fn sdl_test() {
        let sdl_context = sdl2::init().unwrap();
        let video_subsystem = sdl_context.video().unwrap();

        let window = video_subsystem
            .window("rust-sdl2 demo", 800, 600)
            .position_centered()
            .build()
            .unwrap();

        let mut canvas = window.into_canvas().build().unwrap();

        canvas.set_draw_color(Color::RGB(0, 255, 255));
        canvas.clear();
        canvas.present();
        let mut event_pump = sdl_context.event_pump().unwrap();
        let mut i = 0;
        let mut k = 0;
        'running: loop {
            i = (i + 1) % 255;

            if (i % 255) == 254 {
                k += 1;
            }

            if (k % 2) == 0 {
                canvas.set_draw_color(Color::RGB(i, 64, 255 - i));
            } else {
                canvas.set_draw_color(Color::RGB(255 - i, 64, i));
            }
            canvas.clear();
            for event in event_pump.poll_iter() {
                match event {
                    Event::Quit { .. }
                    | Event::KeyDown {
                        keycode: Some(Keycode::Escape),
                        ..
                    } => break 'running,
                    _ => {}
                }
            }

            canvas.present();
            ::std::thread::sleep(Duration::new(0, (1_000_000u32 as f64 / 0.1) as u32));
        }
    }
}
