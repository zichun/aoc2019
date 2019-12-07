use std::io::{self, Read};

fn main() -> Result<(), Box<dyn ::std::error::Error>> {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;

    let input: Vec<i32> = input
        .split("\n")
        .filter_map(
            |s| s.trim().parse().ok())
        .collect();

    println!("Part1: {}", part1(&input));
    println!("Part2: {}", part2(&input));

    Ok(())
}

fn part1(modules: &Vec<i32>) -> i32 {
    modules.iter().map(
        |&s| calculate_fuel(s)
    ).sum()
}

fn part2(modules: &Vec<i32>) -> i32 {
    modules.iter().map(
        |&s| calculate_fuel_recur(s)
    ).sum()
}

fn calculate_fuel(weight: i32) -> i32 {
    let weight = weight / 3 - 2;
    if weight < 0 {
        0
    } else {
        weight
    }
}

fn calculate_fuel_recur(weight: i32) -> i32 {
    let need = calculate_fuel(weight);
    if need <= 0 {
        0
    } else {
        need + calculate_fuel_recur(need)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculate_fuel_test() {
        assert_eq!(calculate_fuel(12), 2);
        assert_eq!(calculate_fuel(1), 0);
        assert_eq!(calculate_fuel(10), 1);
    }

    #[test]
    fn calculate_fuel_recur_test() {
        assert_eq!(calculate_fuel_recur(100756), 50346);
    }
}
