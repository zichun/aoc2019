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

    println!("{}", part1(&input)?);
    part2(&input)?;

    Ok(())
}

#[derive(Clone, Copy)]
enum Direction {
    Up, Down, Left, Right
}

impl Direction {
    fn value(&self) -> (i32, i32) {
        match *self {
            Direction::Up => (-1, 0),
            Direction::Down => (1, 0),
            Direction::Left => (0, -1),
            Direction::Right => (0, 1)
        }
    }
    fn curr_index(&self) -> usize {
        match *self {
            Direction::Up => 0,
            Direction::Right => 1,
            Direction::Down => 2,
            Direction::Left => 3
        }
    }
    fn mutate_direction(self, new_dir: i64, cur_y: i32, cur_x: i32) -> (Direction, i32, i32) {
        const directions: [Direction; 4] = [Direction::Up, Direction::Right, Direction::Down, Direction::Left];
        if new_dir == 0 {
            let new_direction = directions[(self.curr_index() + 3) % 4];
            let (dy, dx) = new_direction.value();
            (new_direction, cur_y + dy, cur_x + dx)
        } else if new_dir == 1 {
            let new_direction = directions[(self.curr_index() + 1) % 4];
            let (dy, dx) = new_direction.value();
            (new_direction, cur_y + dy, cur_x + dx)
        } else {
            panic!("Bad direction given");
        }
    }
}

fn part1(input: &Vec<i64>) -> Result<i64> {
    let mut black_cells = RefCell::new(HashSet::<(i32, i32)>::new());
    let mut ever_painted = HashSet::<(i32, i32)>::new();
    let mut cur_x: RefCell<i32> = RefCell::new(0);
    let mut cur_y: RefCell<i32> = RefCell::new(0);
    let mut dir = Direction::Up;

    let mut machine = IntCode::init(input,
                                    once(0)
                                    .chain(from_fn(|| {
                                        if black_cells.borrow().contains(&(*cur_y.borrow(), *cur_x.borrow())) {
                                            Some(1)
                                        } else {
                                            Some(0)
                                        }
                                    })));

    let mut output_stream = machine.output_stream();
    let mut part1_ans = 0;

    loop {
        if let Some(color) = output_stream.next() {
            if color == 1 {
                black_cells.borrow_mut().insert((*cur_y.borrow(), *cur_x.borrow()));
                if !ever_painted.contains(&(*cur_y.borrow(), *cur_x.borrow())) {
                    part1_ans = part1_ans + 1;
                    ever_painted.insert((*cur_y.borrow(), *cur_x.borrow()));
                }
            } else {
                black_cells.borrow_mut().remove(&(*cur_y.borrow(), *cur_x.borrow()));
            }

            let next_dir = output_stream.next().unwrap();

            let (new_dir, new_cur_y, new_cur_x) = dir.mutate_direction(next_dir, *cur_y.borrow(), *cur_x.borrow());
            *cur_y.borrow_mut() = new_cur_y;
            *cur_x.borrow_mut() = new_cur_x;
            dir = new_dir;
        } else {
            break;
        }
    }

    Ok(part1_ans)
}

fn part2(input: &Vec<i64>) -> Result<()> {
    let black_cells = RefCell::new(HashSet::<(i32, i32)>::new());
    let cur_x: RefCell<i32> = RefCell::new(0);
    let cur_y: RefCell<i32> = RefCell::new(0);
    let mut dir = Direction::Up;

    let machine = IntCode::init(input,
                                once(1)
                                .chain(from_fn(|| {
                                    if black_cells.borrow().contains(&(*cur_y.borrow(), *cur_x.borrow())) {
                                        Some(1)
                                    } else {
                                        Some(0)
                                    }
                                })));

    let mut output_stream = machine.output_stream();

    loop {
        if let Some(color) = output_stream.next() {
            if color == 1 {
                black_cells.borrow_mut().insert((*cur_y.borrow(), *cur_x.borrow()));
            } else {
                black_cells.borrow_mut().remove(&(*cur_y.borrow(), *cur_x.borrow()));
            }

            let next_dir = output_stream.next().unwrap();

            let (new_dir, new_cur_y, new_cur_x) = dir.mutate_direction(next_dir, *cur_y.borrow(), *cur_x.borrow());
            *cur_y.borrow_mut() = new_cur_y;
            *cur_x.borrow_mut() = new_cur_x;
            dir = new_dir;
        } else {
            break;
        }
    }

    let mut min_y = i32::max_value();
    let mut min_x = i32::max_value();
    let mut max_y = i32::min_value();
    let mut max_x = i32::min_value();
    for (y, x) in &(*black_cells.borrow()) {
        if y > &max_y {
            max_y = *y;
        }
        if y < &min_y {
            min_y = *y;
        }
        if x > &max_x {
            max_x = *x;
        }
        if x < &min_x {
            min_x = *x;
        }
    }

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            if black_cells.borrow().contains(&(y, x)) {
                print!("#")
            } else {
                print!(".")
            }
        }
        println!("");
    }

    Ok(())

}

#[cfg(test)]
mod test {
    use super::*;

}
