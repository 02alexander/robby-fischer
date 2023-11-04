use eagle::Vision;

fn main() {
    let mut vision = Vision::new();
    loop {
        println!("{:?}", vision.pieces());
    }
}
