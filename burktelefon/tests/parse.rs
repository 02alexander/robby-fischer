use burktelefon::Burk;

#[derive(Burk)]
pub enum Command {
    #[burk(name = "Q")]
    Queue(f32, f32),
    Other,
}

fn main() {
    let input = "Q 11.56 -100";
    let cmd: Command = input.parse().unwrap();
    println!("{}", cmd);

    //println!("{}", Command::Other);
}
