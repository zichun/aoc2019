use std::collections::HashSet;
use std::collections::HashMap;
use std::iter::FromIterator;

type Result<T> = ::std::result::Result<T, Box<dyn ::std::error::Error>>;

enum Direction {
    Up, Down, Left, Right
}

impl Direction {
    fn value(&self) -> (i8, i8) {
        match *self {
            Direction::Up => (-1, 0),
            Direction::Down => (1, 0),
            Direction::Left => (0, -1),
            Direction::Right => (0, 1)
        }
    }
}

struct Segment {
    direction: Direction,
    length: usize
}

fn main() -> Result<()> {
    let mut line0 = String::new();
    let mut line1 = String::new();

    std::io::stdin().read_line(&mut line0)?;
    std::io::stdin().read_line(&mut line1)?;

    let path0 = parse_input(&line0)?;
    let path1 = parse_input(&line1)?;

    println!("{}", part1(&path0, &path1)?);
    println!("{}", part2(&path0, &path1)?);
    Ok(())
}

fn path_to_coords(path: &Vec<Segment>) -> Vec<(i32, i32)> {
    let mut coords = Vec::<(i32, i32)>::new();
    let mut y: i32 = 0;
    let mut x: i32 = 0;

    for s in path {
        for cnt in 0..s.length {
            y += s.direction.value().0 as i32;
            x += s.direction.value().1 as i32;
            coords.push((y, x));
        }
    }

    coords
}
fn part1(path0: &Vec<Segment>, path1: &Vec<Segment>) -> Result<i32>
{
    // based off https://github.com/Ummon/AdventOfCode2019/blob/master/src/day03.rs
    let positions0: HashSet<(i32, i32)> = HashSet::from_iter(path_to_coords(path0));
    let positions1: HashSet<(i32, i32)> = HashSet::from_iter(path_to_coords(path1));
    let intersection: HashSet<_> = positions0.intersection(&positions1).collect();

    Ok(intersection.iter().map(|(y, x)| y.abs() + x.abs()).min().unwrap())
}

fn part2(path0: &Vec<Segment>, path1: &Vec<Segment>) -> Result<i32>
{
    let positions0 = path_to_coords(path0);
    let positions1 = path_to_coords(path1);
    let positions0_map: HashMap<&(i32, i32), usize> = HashMap::from_iter(positions0.iter().enumerate().map(|(i, pos)| (pos, i)));

    let best = positions1.iter().enumerate().filter_map(
        |(index, pos)|
        if let Some(s) = positions0_map.get(pos) {
            Some(s + index)
        } else {
            None
        }
    ).min().unwrap();

    Ok((best + 2) as i32)
}

fn parse_input(input: &str) -> Result<Vec<Segment>> {

    let path: Vec<Segment> = input
        .split(",")
        .map(|s|
             {
                 let dir = s.chars().nth(0).ok_or("Invalid Input").unwrap();
                 let len_str: String = s.chars().filter(|x| x.is_digit(10)).collect();
                 let len: usize = len_str.parse::<usize>().unwrap();
                 match dir {
                     'U' => {
                         Segment {
                             direction: Direction::Up,
                             length: len
                         }
                     }
                     'D' => {
                         Segment {
                             direction: Direction::Down,
                             length: len
                         }
                     }
                     'L' => {
                         Segment {
                             direction: Direction::Left,
                             length: len
                         }
                     }
                     'R' => {
                         Segment {
                             direction: Direction::Right,
                             length: len
                         }
                     }
                     _ => {
                         panic!("Invalid input!")
                     }
                 }
             }
        ).collect();

    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_part1(){
        let path0 = parse_input("R8,U5,L5,D3").unwrap();
        let path1 = parse_input("U7,R6,D4,L4").unwrap();
        assert_eq!(part1(&path0, &path1).unwrap(), 6);

        let path0 = parse_input("R75,D30,R83,U83,L12,D49,R71,U7,L72").unwrap();
        let path1 = parse_input("U62,R66,U55,R34,D71,R55,D58,R83").unwrap();
        assert_eq!(part1(&path0, &path1).unwrap(), 159);

        let path0 = parse_input("R98,U47,R26,D63,R33,U87,L62,D20,R33,U53,R51").unwrap();
        let path1 = parse_input("U98,R91,D20,R16,D67,R40,U7,R15,U6,R7").unwrap();
        assert_eq!(part1(&path0, &path1).unwrap(), 135);
    }

    #[test]
    fn test_part2() {
        let path0 = parse_input("R8,U5,L5,D3").unwrap();
        let path1 = parse_input("U7,R6,D4,L4").unwrap();
        assert_eq!(part2(&path0, &path1).unwrap(), 30);

        let path0 = parse_input("R75,D30,R83,U83,L12,D49,R71,U7,L72").unwrap();
        let path1 = parse_input("U62,R66,U55,R34,D71,R55,D58,R83").unwrap();
        assert_eq!(part2(&path0, &path1).unwrap(), 610);

        let path0 = parse_input("R98,U47,R26,D63,R33,U87,L62,D20,R33,U53,R51").unwrap();
        let path1 = parse_input("U98,R91,D20,R16,D67,R40,U7,R15,U6,R7").unwrap();
        assert_eq!(part2(&path0, &path1).unwrap(), 410);
    }
}
