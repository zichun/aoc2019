use std::io::{self};
use std::collections::VecDeque;
use std::collections::HashSet;
use std::iter::*;
use std::cell::RefCell;

type Result<T> = ::std::result::Result<T, Box<dyn ::std::error::Error>>;

#[derive(Debug,PartialEq)]
enum ParameterType {
    Ref(usize),
    Value(i64),
    Relative(i64)
}

enum Instruction {
    Add { left_op: ParameterType, right_op: ParameterType, into: ParameterType },
    Mul { left_op: ParameterType, right_op: ParameterType, into: ParameterType },
    Input { into: ParameterType },
    Output { param: ParameterType },
    JumpIfTrue { cond: ParameterType, to: ParameterType },
    JumpIfFalse { cond: ParameterType, to: ParameterType },
    LessThan { left_op: ParameterType, right_op: ParameterType, into: ParameterType },
    Equals { left_op: ParameterType, right_op: ParameterType, into: ParameterType },
    RelativeBase { adjust: ParameterType },
    Terminate,
}

struct IntCode<T: Iterator> {
    memory: Vec<i64>,
    address_ptr: usize,
    input_stream: T,
    output_buffer: VecDeque<i64>,
    is_terminated: bool,
    relative_ptr: i64
}

struct OutputStream<T: Iterator>(IntCode<T>);

impl<T> Iterator for OutputStream<T> where
    T: Iterator<Item = i64>
{
    type Item = i64;
    fn next(&mut self) -> Option<i64> {
        if self.0.output_buffer.len() > 0 {
            self.0.output_buffer.pop_front()
        } else {
            self.0.run_to_next_output()
        }
    }
}

impl<T> IntCode<T> where
    T: Iterator<Item = i64> {
    fn init(memory: &Vec<i64>, input_stream: T) -> IntCode<T> {
        IntCode {
            memory: memory.clone(),
            address_ptr: 0,
            input_stream: input_stream,
            output_buffer: VecDeque::new(),
            is_terminated: false,
            relative_ptr: 0
        }
    }

    fn parse_op_code(input: &i64) -> Result<(u32, VecDeque<ParameterType>)> {
        let op_code = input % 100;
        let mut parameter_mode = VecDeque::<ParameterType>::new();
        let mut parameter_stream = input / 100;

        while parameter_stream > 0 {
            parameter_mode.push_back(
                match parameter_stream % 10 {
                    0 => ParameterType::Ref(0),
                    1 => ParameterType::Value(0),
                    2 => ParameterType::Relative(0),
                    _ => { return Err(format!("Invalid OpCode: {}", input).into()) }
                }
            );
            parameter_stream /= 10;
        }

        Ok((op_code as u32, parameter_mode))
    }

    fn output_stream(self) -> OutputStream<T> {
        OutputStream(self)
    }

    fn run_to_next_output(&mut self) -> Option<i64> {
        while self.output_buffer.len() == 0 && self.is_terminated == false {
            // bad code; output iterator should be a result
            self.run_tick().unwrap();
        }

        self.output_buffer.pop_front()
    }

    fn read_parameter(
        &mut self,
        parameter_mode: &mut VecDeque<ParameterType>,
        is_writing: bool // If parameter is for a write operation, parameter type must be a reference
    ) -> Result<ParameterType> {
        let parameter_value = self.memory.get(self.address_ptr).ok_or("Invalid Address, address pointer out of bounds when reading parameter")?;
        let parameter_type = parameter_mode.pop_front().unwrap_or(ParameterType::Ref(0));

        self.address_ptr = self.address_ptr + 1;

        match parameter_type {
            ParameterType::Ref(_) => {
                Ok(ParameterType::Ref(*parameter_value as usize))
            },
            ParameterType::Value(_) => {
                if is_writing {
                    Err("Invalid parameter type: parameter is for a write operation".into())
                } else {
                    Ok(ParameterType::Value(*parameter_value))
                }
            },
            ParameterType::Relative(_) => {
                Ok(ParameterType::Relative(*parameter_value))
            }
        }
    }

    fn read_instruction(&mut self) -> Result<(Instruction)> {
        let op_code = self.memory.get(self.address_ptr).ok_or("Invalid Address, address pointer out of bounds when reading instruction")?;
        self.address_ptr = self.address_ptr + 1;

        let (op_code, mut parameter_mode) = IntCode::<T>::parse_op_code(op_code)?;

        let instruction = match op_code {
            1 => {
                Instruction::Add {
                    left_op: self.read_parameter(&mut parameter_mode, false)?,
                    right_op: self.read_parameter(&mut parameter_mode, false)?,
                    into: self.read_parameter(&mut parameter_mode, true)?
                }
            }
            2 => {
                Instruction::Mul {
                    left_op: self.read_parameter(&mut parameter_mode, false)?,
                    right_op: self.read_parameter(&mut parameter_mode, false)?,
                    into: self.read_parameter(&mut parameter_mode, true)?
                }
            }
            3 => {
                Instruction::Input {
                    into: self.read_parameter(&mut parameter_mode, true)?
                }
            },
            4 => {
                Instruction::Output {
                    param: self.read_parameter(&mut parameter_mode, false)?
                }
            }
            5 => {
                Instruction::JumpIfTrue {
                    cond: self.read_parameter(&mut parameter_mode, false)?,
                    to: self.read_parameter(&mut parameter_mode, false)?
                }
            }
            6 => {
                Instruction::JumpIfFalse {
                    cond: self.read_parameter(&mut parameter_mode, false)?,
                    to: self.read_parameter(&mut parameter_mode, false)?
                }
            }
            7 => {
                Instruction::LessThan {
                    left_op: self.read_parameter(&mut parameter_mode, false)?,
                    right_op: self.read_parameter(&mut parameter_mode, false)?,
                    into: self.read_parameter(&mut parameter_mode, true)?
                }
            },
            8 => {
                Instruction::Equals {
                    left_op: self.read_parameter(&mut parameter_mode, false)?,
                    right_op: self.read_parameter(&mut parameter_mode, false)?,
                    into: self.read_parameter(&mut parameter_mode, true)?
                }
            }
            9 => {
                Instruction::RelativeBase {
                    adjust: self.read_parameter(&mut parameter_mode, false)?
                }
            }
            99 => {
                Instruction::Terminate
            }
            _ => {
                return Err("Invalid Opcode".into());
            }
        };

        Ok(instruction)
    }

    fn resolve_parameter_value(&self, parameter: ParameterType) -> Result<i64> {
        match parameter {
            ParameterType::Ref(address) => {
                Ok(*self.memory.get(address).unwrap_or(&0))
            },
            ParameterType::Value(value) => {
                Ok(value)
            },
            ParameterType::Relative(offset) => {
                Ok(*self.memory.get((self.relative_ptr + offset) as usize).unwrap_or(&0))
            }
        }
    }

    fn write_memory(&mut self, into: ParameterType, value: i64) -> Result<()> {
        let address = match into {
            ParameterType::Ref(address) => {
                address
            },
            ParameterType::Relative(offset) => {
                (self.relative_ptr + offset) as usize
            },
            _ => {
                panic!("")
            }
        };

        if address >= self.memory.len() {
            self.memory.resize(address + 1, 0);
        }

        let into_ref = self.memory.get_mut(address).ok_or(format!("Invalid address reference: {}", address))?;
        *into_ref = value;

        Ok(())
    }

    fn run_tick(&mut self) -> Result<()> {
        let instruction = self.read_instruction()?;

        match instruction {
            Instruction::Add { left_op, right_op, into } => {
                let sum = self.resolve_parameter_value(left_op)? + self.resolve_parameter_value(right_op)?;
                self.write_memory(into, sum)?;
            }
            Instruction::Mul { left_op, right_op, into } => {
                let product = self.resolve_parameter_value(left_op)? * self.resolve_parameter_value(right_op)?;
                self.write_memory(into, product)?;
            }
            Instruction::Input { into } => {
                let input_value = self.input_stream.next().ok_or("Ran out of input")?;
                self.write_memory(into, input_value)?;
            }
            Instruction::Output { param } => {
                self.output_buffer.push_back(self.resolve_parameter_value(param)?);
            }
            Instruction::JumpIfTrue { cond, to } => {
                let val = self.resolve_parameter_value(cond)?;
                if val != 0 {
                    self.address_ptr = self.resolve_parameter_value(to)? as usize;
                }
            }
            Instruction::JumpIfFalse { cond, to } => {
                let val = self.resolve_parameter_value(cond)?;
                if val == 0 {
                    self.address_ptr = self.resolve_parameter_value(to)? as usize;
                }
            }
            Instruction::LessThan { left_op, right_op, into } => {
                let less_than = if self.resolve_parameter_value(left_op)? < self.resolve_parameter_value(right_op)? {
                    1
                } else { 0 };
                self.write_memory(into, less_than)?;
            }
            Instruction::Equals { left_op, right_op, into } => {
                let equals = if self.resolve_parameter_value(left_op)? == self.resolve_parameter_value(right_op)? {
                    1
                } else { 0 };
                self.write_memory(into, equals)?;
            }
            Instruction::RelativeBase { adjust } => {
                self.relative_ptr = self.relative_ptr + self.resolve_parameter_value(adjust)?;
            }
            Instruction::Terminate => {
                self.is_terminated = true;
            }
        };

        Ok(())
    }

    fn run_to_termination(&mut self) -> Result<()> {
        while self.is_terminated == false {
            self.run_tick()?;
        }
        Ok(())
    }
}

fn main() -> Result<()> {
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let input: Vec<i64> = input
        .split(",")
        .filter_map(|s|
                    s.trim().parse().ok()
        ).collect();

    println!("Part1: {}", part1(&input)?);
    println!("Part2: {}", part2(&input)?);

    Ok(())
}

type MapType = Vec<Vec<char>>;

fn parse_map(input: &Vec<i64>) -> MapType {
    let machine = IntCode::init(input, once(1));
    let output: Vec<i64> = machine.output_stream().collect();
    let map_string: String = output.iter().map(|x| (*x as u8) as char).collect();

    let mut map: Vec<Vec<char>> = Vec::new();
    println!("{}", map_string);
    map_string.lines().for_each(|x| {
        let mut map_line = Vec::new();
        if x.trim().len() > 0 {
            x.chars().for_each(|x| {
                map_line.push(x);
            });
            map.push(map_line);
        }
    });

    map
}

fn path_to_string(path: &PathType) -> String {
    let mut output = String::new();
    for p in path {
        if output.len() > 0 {
            output = output + ",";
        }
        output = output + &p.0.to_string() + ",";
        output = output + &p.1.to_string();
    }
    output
}

struct Coord(i16, i16);

#[derive(Copy, Clone, Debug)]
enum Direction {
    Up, Down, Left, Right
}

#[derive(Clone, Debug)]
enum Turn {
    L(Direction),
    R(Direction)
}

impl PartialEq for Turn {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Turn::L(a), Turn::L(b)) => true,
            (Turn::R(a), Turn::R(b)) => true,
            _ => false
        }
    }
}

impl Turn {
    fn to_string(&self) -> String {
        match self {
            Turn::L(x) => "L".to_string(),
            Turn::R(x) => "R".to_string()
        }
    }
    fn dir(&self) -> Direction {
        match self {
            Turn::L(x) => *x,
            Turn::R(x) => *x
        }
    }
}

impl Direction {
    fn val(&self) -> Coord {
        match self {
            Direction::Up => Coord(-1, 0),
            Direction::Down => Coord(1, 0),
            Direction::Left => Coord(0, -1),
            Direction::Right => Coord(0, 1),
        }
    }
    fn turn(&self) -> (Turn, Turn) {
        match self {
            Direction::Up => (Turn::L(Direction::Left), Turn::R(Direction::Right)),
            Direction::Down => (Turn::L(Direction::Right), Turn::R(Direction::Left)),
            Direction::Left => (Turn::L(Direction::Down), Turn::R(Direction::Up)),
            Direction::Right => (Turn::L(Direction::Up), Turn::R(Direction::Down))
        }
    }
}

fn has_route(map: &MapType, coord: &Coord) -> bool {
    let total_row = map.len();
    let total_col = map[0].len();

    if coord.0 < 0 || coord.0 >= total_row as i16 ||
        coord.1 < 0 || coord.1 >= total_col as i16
    {
        return false;
    }

    map[coord.0 as usize][coord.1 as usize] != '.'
}

fn move_in_dir(coord: &Coord, dir: &Direction) -> Coord {
    let displacement = dir.val();
    Coord(
        coord.0 + displacement.0,
        coord.1 + displacement.1
    )
}

fn can_turn(map: &MapType, coord: &Coord, dir: &Direction) -> bool {
    let new_coord = move_in_dir(coord, dir);
    has_route(map, &new_coord)
}

type PathType = Vec<(Turn, usize)>;
type PathSlice = [(Turn, usize)];

fn feasible(path_slice: &PathSlice) -> bool {
    let mut req_size = 0;
    for p in path_slice {
        req_size = req_size + if p.1 >= 10 {
            2
        } else {
            1
        };
        req_size = req_size + 2;
    }
    req_size -= 1;

    req_size <= 20
}

fn try_split_path(path: &PathType, part_a: &PathSlice, part_b: &PathSlice, part_c: &PathSlice) -> Option<Vec<char>> {
    let mut start = 0;
    let mut arrangement = Vec::new();

    while start < path.len() {
        if can_consume(path, part_a, start) {
            start += part_a.len();
            arrangement.push('A');
        } else if can_consume(path, part_b, start) {
            start += part_b.len();
            arrangement.push('B');
        } else if can_consume(path, part_c, start) {
            start += part_c.len();
            arrangement.push('C');
        } else {
            return None;
        }
    }

    if arrangement.len() * 2 - 1 > 20 {
        None
    } else {
        Some(arrangement)
    }

}

fn can_consume(path: &PathType, part: &PathSlice, start_index: usize) -> bool {
    if start_index + part.len() > path.len() {
        return false;
    }

    for i in 0..part.len() {
        if part[i] != path[i + start_index] {
            return false;
        }
    }
    return true;
}

fn break_path(path: &PathType) -> Option<(PathType, PathType, PathType, Vec<char>)> {
    let mut split_0 = 0;
    let mut split_1 = 0;

    for i in 1..path.len() {
        let part_a = path.get(0..i).unwrap();
        if !feasible(part_a) {
            break;
        }

        for j in (i + 1)..path.len() {
            let part_b = path.get(i..j).unwrap();

            if !feasible(part_b) {
                break;
            }

            let mut k = j;
            loop {
                if can_consume(path, part_a, k) {
                    k += part_a.len();
                } else if can_consume(path, part_b, k) {
                    k += part_b.len();
                } else {
                    break;
                }
            }

            for l in k + 1..path.len() {
                let part_c = path.get(k..l).unwrap();
                if !feasible(part_c) {
                    break;
                }

                let attempt = try_split_path(path, part_a, part_b, part_c);
                match attempt {
                    Some(arrangement) => {
                        return Some(
                            (part_a.to_vec(),
                             part_b.to_vec(),
                             part_c.to_vec(),
                             arrangement)
                        );
                    }
                    None => {
                        continue;
                    }
                }
            }
        }
    }

    None
}

fn part2(input: &Vec<i64>) -> Result<i64> {
    let map = parse_map(input);
    let total_row = map.len();
    let total_col = map[0].len();

    let mut cur_row = total_row + 1;
    let mut cur_col = 0;

    for r in 0..total_row {
        for c in 0..total_col {
            if map[r][c] == '^' {
                cur_row = r;
                cur_col = c;
                break;
            }
        }
        if cur_row <= total_row {
            break;
        }
    }
    if cur_row == total_row + 1 {
        return Err("Cannot find starting position!".into());
    }

    //
    // Path exploration is greedy. This is exploiting nature of the
    // graph in the input that will necessarily result in an euler
    // walk.
    //

    let mut cur_dir = Direction::Up;
    let mut cur_coord = Coord(cur_row as i16, cur_col as i16);
    let mut path = Vec::new();

    loop {
        //
        // Find next direction
        //
        let turns = cur_dir.turn();
        let mut current_turn = Turn::L(Direction::Up);
        if can_turn(&map, &cur_coord, &(turns.0).dir()) {
            current_turn = turns.0;
        } else if can_turn(&map, &cur_coord, &(turns.1).dir()) {
            current_turn = turns.1;
        } else {
            // We are done!
            break;
        }

        cur_dir = current_turn.dir();

        //
        // Move in direction
        //
        let mut move_count = 0;
        loop {
            let next_coord = move_in_dir(&cur_coord, &cur_dir);
            if !has_route(&map, &next_coord) {
                break;
            } else {
                move_count = move_count + 1;
                cur_coord = next_coord;
            }
        }

        path.push((current_turn, move_count));
    }

    let (path_a, path_b, path_c, arrangement) = break_path(&path).ok_or("cannot find path")?;
    println!("{}", path_to_string(&path));
    let mut output = String::new();
    for a in arrangement {
        if output.len() > 0 {
            output = output + ",";
        }
        output = output + &a.to_string();
    }
    output = output + "\n";
    output = output + &path_to_string(&path_a) + "\n";
    output = output + &path_to_string(&path_b) + "\n";
    output = output + &path_to_string(&path_c) + "\n";
    output = output + "n\n";
    println!("{}", output);

    let mut hack = input.clone();
    hack[0] = 2;
    let input_stream = output.chars().map(|x| x as i64);

    let machine = IntCode::init(&hack, input_stream);
    let output = machine.output_stream();
    Ok(output.last().ok_or("No output")?)
}

fn part1(input: &Vec<i64>) -> Result<i64> {
    let map = parse_map(input);
    let total_row = map.len();
    let total_col = map[0].len();

    let mut sum = 0;
    for r in 1..total_row-1 {
        for c in 1..total_col-1 {
            if map[r][c] == '#' && map[r-1][c] == '#' && map[r+1][c] == '#'
                && map[r][c-1] == '#' && map[r][c+1] == '#' {
                    sum = sum + ((r as i64) * (c as i64));
                }
        }
    }

    Ok(sum)
}

