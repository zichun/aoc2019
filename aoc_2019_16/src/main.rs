use std::io::{self};
use std::iter::from_fn;
use std::iter::Extend;

type Result<T> = ::std::result::Result<T, Box<dyn ::std::error::Error>>;

fn main() -> Result<()> {
    let mut input = String::new();

    io::stdin().read_line(&mut input)?;

    println!("part1: {}", part1(&input, 100)?);
    println!("part2: {}", part2(&input, 100)?);
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

fn parse_input_part2(input: &str) -> Vec<u8> {
    let base_input: Vec<u32> = input.chars().filter_map(|x| x.to_digit(10)).collect();
    let mut tr: Vec<u32> = Vec::new();
    for i in 0..10000 {
        tr.extend(base_input.iter());
    }
    tr.into_iter().map(|x| x as u8).collect()
}

fn part2(input: &str, phases: usize) -> Result<String> {
    let mut new_input = parse_input_part2(input);
    let skip_string: String = new_input.as_slice()[0..7].iter().map(|x| std::char::from_digit(*x as u32, 10).unwrap() ).collect();
    let skip = skip_string.parse::<usize>()?;

    for i in 1..=phases {
        let mut next_input = Vec::new();
        let mut prefix_sum: Vec<i64> = Vec::new();
        prefix_sum.push(new_input[0] as i64);
        for j in 1..new_input.len() {
            prefix_sum.push( prefix_sum[j - 1] + new_input[j] as i64 );
        }
        for j in 1..=new_input.len() {
            let mut start = j - 1;
            let mut sum: i64 = 0;
            let mut add = true;

            while start < new_input.len() {
                let segment = if start == 0 {
                    prefix_sum[ usize::min(prefix_sum.len() - 1, start + j - 1) ]
                } else {
                    prefix_sum[ usize::min(prefix_sum.len() - 1, start + j - 1) ] - prefix_sum[start - 1]
                };

                if add {
                    sum = sum + segment;
                } else {
                    sum = sum - segment;
                }

                start = start + j + j;
                add = !add;
            }
            next_input.push( (i64::abs(sum) % 10) as u8 );
        }
        new_input = next_input;
    }

    let output_string: String = new_input.as_slice()[skip..skip+8].into_iter().take(8).map(|x| std::char::from_digit(*x as u32, 10).unwrap() ).collect();
    Ok(output_string)
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

    #[test]
    fn test_part_2() {
        assert_eq!(part2("03036732577212944063491565474664", 100).unwrap(), "84462026");
        assert_eq!(part2("02935109699940807407585447034323", 100).unwrap(), "78725270");
        assert_eq!(part2("03081770884921959731165446850517", 100).unwrap(), "53553731");
    }
}
