use std::{sync::mpsc::{Receiver, self}, io::stdin, path::Path};
use std::io::BufRead;

use eagle::Vision;

struct Button {
    handle: Receiver<()>,
}

impl Button {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let mut stdin = stdin().lock();
            for line in stdin.lines() {
                if sender.send(()).is_err() {
                    break;
                }
            }
        });
        Button {
            handle: receiver,
        }
    }

    pub fn is_pressed(&mut self) -> bool {
        self.handle.try_recv().is_ok()
    }
}

fn main() {
    let mut vision = Vision::new();
    let mut button = Button::new();
    let mut i = 0;
    loop {
        if button.is_pressed() {
            let images = vision.train_data().unwrap();
            println!("{}", images.len());
            for (image, is_white) in images {   
                loop {
                    let color_name = if is_white { "white" } else { "black" };
                    let s = format!("train_images/{i}_{color_name}.png");
                    i += 1;
                    if !Path::new(&s).exists() {
                        image.save(s).unwrap();
                        break;
                    } 
    
                }
            }

        }
        vision.pieces();
    }
}