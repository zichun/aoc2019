use std::io::{self, Read};
use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;

type Result<T> = ::std::result::Result<T, Box<dyn ::std::error::Error>>;

type AdjList = HashMap<String, Vec<String>>;

fn add_adj(graph: &mut AdjList, from: &str, to: &str) {
    let key: String = from.to_string();
    if !graph.contains_key(&key) {
        graph.insert(from.to_string(), Vec::new());
    }
    graph.get_mut(&key).unwrap().push(to.to_string());
}

fn parse_input(input: &String) -> Result<AdjList> {
    let mut graph = AdjList::new();

    input.lines()
        .for_each(|x| {
            let v: Vec<&str> = x.split(')').collect();
            assert_eq!(v.len(), 2);
            add_adj(&mut graph, v[0], v[1]);
            add_adj(&mut graph, v[1], v[0]);
        });

    Ok(graph)
}

fn dfs(graph: &AdjList, curr: &String, prev: &String, curr_cnt: u32) -> u32 {
    let mut tr = curr_cnt;

    if let Some(adj) = graph.get(curr) {
        for u in adj {
            if u != prev {
                tr = tr + dfs(graph, u, curr, curr_cnt + 1);
            }
        }
        tr
    } else {
        curr_cnt
    }
}

fn part1(graph: &AdjList) -> u32 {
    dfs(graph, &"COM".to_string(), &"".to_string(), 0)
}

struct QueueElement {
    node: String,
    dist: u32
}

fn part2(graph: &AdjList) -> Result<u32> {
    let mut queue = VecDeque::<QueueElement>::new();
    let mut visited = HashSet::<String>::new();

    queue.push_back(QueueElement {
        node: "YOU".to_string(),
        dist: 0
    });
    visited.insert("YOU".to_string());

    while !queue.is_empty() {
        let top = queue.pop_front().unwrap();

        if top.node == "SAN" {
            return Ok(top.dist - 2);
        }

        for u in graph.get(&top.node).unwrap() {
            if !visited.contains(u) {
                visited.insert(u.to_string());
                queue.push_back(QueueElement {
                    node: u.to_string(),
                    dist: top.dist + 1
                });
            }
        }
    }

    Err("Couldn't find a path from YOU to SAN".into())
}

fn main() -> Result<()>{
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;

    let graph = parse_input(&input)?;

    println!("part1: {}", part1(&graph));
    println!("part2: {}", part2(&graph)?);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_part1() {
        let graph = parse_input(&"COM)B
B)C
C)D
D)E
E)F
B)G
G)H
D)I
E)J
J)K
K)L".to_string()).unwrap();
        assert_eq!(part1(&graph), 42);
    }

    #[test]
    fn test_part2() {
        let graph = parse_input(&"COM)B
B)C
C)D
D)E
E)F
B)G
G)H
D)I
E)J
J)K
K)L
K)YOU
I)SAN".to_string()).unwrap();
        assert_eq!(part2(&graph).unwrap(), 4);
    }
}
