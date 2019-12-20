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
//    println!("{}", part2(&input)?);

    Ok(())
}

fn part1(input: &Vec<i64>) -> Result<i64> {
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

