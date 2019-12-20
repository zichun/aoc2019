use std::io::{self};
use std::iter::from_fn;

type Result<T> = ::std::result::Result<T, Box<dyn ::std::error::Error>>;

fn main() -> Result<()> {
    let mut input = String::new();

    io::stdin().read_line(&mut input)?;

    println!("part1: {}", part1(&input, 100)?);
    Ok(())
}

fn parse_input(input: &str) -> Vec<u32> {
    input.chars()
        .filter_map(|x| x.to_digit(10)).collect()
}

struct FTT {
    seq: Vec<u8>
}

impl Iterator for FTT {
    type Item = Vec<u8>;

    fn next(&mut self) -> Option<Vec<u8>>{
        let base_pattern: Vec<i8> = vec![0, 1, 0, -1];

        let mut new_vec = Vec::<u8>::new();
        for i in 1..=self.seq.len() {
            let mut base_pattern_index = 0;
            let mut pattern_count = 0;
            let mut pattern_iter = from_fn(|| {
                let to_print = base_pattern[base_pattern_index];
                pattern_count = pattern_count + 1;
                if pattern_count == i {
                    pattern_count = 0;
                    base_pattern_index = (base_pattern_index + 1) % base_pattern.len();
                }
                Some(to_print)
            });

            let _mul = pattern_iter.next().unwrap(); // drop first value

            let mut val: i32 = 0;
            for j in &self.seq {
                let mul = pattern_iter.next().unwrap();
                val = val + (*j as i32) * (mul as i32);
            }
            new_vec.push((i32::abs(val) % 10) as u8);
        }

        self.seq = new_vec.clone();
        Some(new_vec)
    }
}

fn part1(input: &str, phases: usize) -> Result<String> {
    let input: Vec<u8> = parse_input(input).into_iter().map(|x| x as u8).collect();
    let base_pattern = vec![0, 1, 0, -1];

    let ftt_stream = FTT {
        seq: input.clone()
    };

    let output = ftt_stream.take(phases).last().unwrap();
    let output_string: String = output.into_iter().take(8).map(|x| std::char::from_digit(x as u32, 10).unwrap() ).collect();

    Ok(output_string.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_part_1() {
        assert_eq!(part1("12345678", 4).unwrap(), "01029498");
        assert_eq!(part1("80871224585914546619083218645595", 100).unwrap(), "24176176");
        assert_eq!(part1("19617804207202209144916044189917", 100).unwrap(), "73745418");
        assert_eq!(part1("69317163492948606335995924319873", 100).unwrap(), "52432133");
    }
}
