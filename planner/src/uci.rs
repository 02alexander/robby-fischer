use std::cmp::Reverse;
use std::fmt::{Display, Write as _};
use std::io::{BufRead, BufReader, Result, Write};
use std::ops::Range;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

struct Uci {
    _child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl Uci {
    pub fn new(program: &str, args: &[&str]) -> Result<Self> {
        let mut child = Command::new(program)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());

        let mut uci = Uci {
            _child: child,
            stdin,
            stdout,
        };

        uci.write_line("uci")?;

        loop {
            let line = uci.read_line()?;
            let parts: Vec<_> = line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }
            if parts[0] == "uciok" {
                break;
            }
        }

        Ok(uci)
    }

    fn write_line(&mut self, line: impl Display) -> Result<()> {
        //println!("> {line}");
        writeln!(self.stdin, "{line}")
    }

    fn read_line(&mut self) -> Result<String> {
        let mut line = String::new();
        self.stdout.read_line(&mut line)?;
        line.pop();
        //println!("< {line}");
        Ok(line)
    }

    pub fn show_all_lines(&mut self, show: bool) -> Result<()> {
        self.write_line(match show {
            true => "setoption name MultiPV value 500",
            false => "setoption name MultiPV value 1",
        })
    }

    pub fn start_search<I, S>(&mut self, moves: I) -> Result<()>
    where
        I: IntoIterator<Item = S>,
        S: Display,
    {
        let mut command = "position startpos moves".to_owned();
        for mov in moves {
            write!(command, " {mov}").unwrap();
        }
        self.write_line(command)?;
        self.write_line("go infinite")
    }

    pub fn stop_search(&mut self) -> Result<Vec<(String, i32)>> {
        self.write_line("stop")?;

        let mut lines = Vec::new();

        loop {
            let line = self.read_line()?;
            let mut parts = line.split_whitespace();

            match parts.next() {
                Some("bestmove") => break,
                Some("info") => {
                    let mut multipv: Option<u32> = None;
                    let mut score: Option<i32> = None;
                    let mut mov: Option<&str> = None;
                    while let Some(part) = parts.next() {
                        match part {
                            "multipv" => multipv = parts.next().and_then(|part| part.parse().ok()),
                            "cp" => score = parts.next().and_then(|part| part.parse().ok()),
                            "mate" => {
                                score = parts
                                    .next()
                                    .and_then(|part| part.parse().ok())
                                    .map(|moves: i32| moves.signum() * 100_000)
                            }
                            "pv" => mov = parts.next(),
                            _ => {}
                        }
                    }
                    if multipv.unwrap() == 1 {
                        lines.clear();
                    }
                    lines.push((mov.unwrap().to_owned(), score.unwrap()));
                }
                _ => {}
            }
        }

        Ok(lines)
    }
}

pub struct Engine {
    uci: Uci,
    trying_to_win: bool,
    gambit_range: Range<i32>,
}

impl Engine {
    pub fn new(program: &str, args: &[&str]) -> Result<Self> {
        let uci = Uci::new(program, args)?;
        Ok(Engine {
            uci,
            trying_to_win: false,
            gambit_range: -1000..-500,
        })
    }
    pub fn start_search<I, S>(&mut self, moves: I) -> Result<()>
    where
        I: IntoIterator<Item = S>,
        S: Display,
    {
        self.uci.show_all_lines(!self.trying_to_win)?;
        self.uci.start_search(moves)
    }
    pub fn stop_search(&mut self) -> Result<String> {
        let mut moves = self.uci.stop_search()?;
        moves.sort_by_key(|&(_, score)| Reverse(score));

        if moves[0].1 < self.gambit_range.end {
            self.trying_to_win = true;
        }

        if self.trying_to_win {
            return Ok(moves[0].0.clone());
        }

        if let Some((mov, _)) = moves
            .iter()
            .find(|&&(_, score)| self.gambit_range.contains(&score))
        {
            return Ok(mov.clone());
        }

        if let Some((mov, _)) = moves
            .iter()
            .rfind(|&&(_, score)| score > self.gambit_range.start)
        {
            return Ok(mov.clone());
        }

        Ok(moves[0].0.clone())
    }
}
