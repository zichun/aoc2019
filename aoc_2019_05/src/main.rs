use std::io::{self};
use std::collections::VecDeque;

type Result<T> = ::std::result::Result<T, Box<dyn ::std::error::Error>>;

#[derive(Debug,PartialEq)]
enum ParameterType {
    Ref(usize),
    Value(i32)
}

enum Instruction {
    Add { left_op: ParameterType, right_op: ParameterType, into: ParameterType },
    Mul { left_op: ParameterType, right_op: ParameterType, into: ParameterType },
    Input { into: ParameterType },
    Output { param: ParameterType },
    Terminate,
}

struct IntCode {
    memory: Vec<i32>,
    address_ptr: usize,
}

impl IntCode {
    fn init(memory: &Vec<i32>) -> IntCode {
        IntCode {
            memory: memory.clone(),
            address_ptr: 0
        }
    }

    fn parse_op_code(input: &i32) -> Result<(u32, VecDeque<ParameterType>)> {
        let op_code = input % 100;
        let mut parameter_mode = VecDeque::<ParameterType>::new();
        let mut parameter_stream = input / 100;

        while parameter_stream > 0 {
            parameter_mode.push_back(
                match parameter_stream % 10 {
                    0 => ParameterType::Ref(0),
                    1 => ParameterType::Value(0),
                    _ => { return Err(format!("Invalid OpCode: {}", input).into()) }
                }
            );
            parameter_stream /= 10;
        }

        Ok((op_code as u32, parameter_mode))
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
            }
        }
    }

    fn read_instruction(&mut self) -> Result<(Instruction)> {
        let op_code = self.memory.get(self.address_ptr).ok_or("Invalid Address, address pointer out of bounds when reading instruction")?;
        self.address_ptr = self.address_ptr + 1;

        let (op_code, mut parameter_mode) = IntCode::parse_op_code(op_code)?;

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
            99 => {
                Instruction::Terminate
            }
            _ => {
                return Err("Invalid Opcode".into());
            }
        };

        Ok(instruction)
    }

    fn resolve_parameter_value(&self, parameter: ParameterType) -> Result<i32> {
        match parameter {
            ParameterType::Ref(address) => {
                Ok(*self.memory.get(address).ok_or(format!("Invalid address reference: {}", address))?)
            },
            ParameterType::Value(value) => {
                Ok(value)
            }
        }
    }

    fn write_memory(&mut self, into: ParameterType, value: i32) -> Result<()> {
        match into {
            ParameterType::Ref(address) => {
                let into_ref = self.memory.get_mut(address).ok_or(format!("Invalid address reference: {}", address))?;
                *into_ref = value;
            },
            _ => {
                panic!("")
            }
        }
        Ok(())
    }

    fn run(&mut self, input_stream: &VecDeque<i32>) -> Result<(&Vec<i32>, Vec<i32>)> {
        let mut output_stream = Vec::<i32>::new();
        let mut input_stream = input_stream.clone();

        loop {
            let instruction = self.read_instruction()?;

            match instruction {
                Instruction::Add { left_op, right_op, into } => {
                    let sum = self.resolve_parameter_value(left_op)? + self.resolve_parameter_value(right_op)?;
                    self.write_memory(into, sum)?;
                },
                Instruction::Mul { left_op, right_op, into } => {
                    let product = self.resolve_parameter_value(left_op)? * self.resolve_parameter_value(right_op)?;
                    self.write_memory(into, product)?;
                },
                Instruction::Input { into } => {
                    let input_value = input_stream.pop_front().ok_or("Ran out of input")?;
                    self.write_memory(into, input_value)?;
                },
                Instruction::Output { param } => {
                    output_stream.push(self.resolve_parameter_value(param)?);
                },
                Instruction::Terminate => {
                    return Ok((&self.memory, output_stream));
                }
            };
        }
    }
}

fn main() -> Result<()> {
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let input: Vec<i32> = input
        .split(",")
        .filter_map(|s|
                    s.trim().parse().ok()
        ).collect();

    println!("Part1: {:?}", part1(&input));

    Ok(())
}

fn part1(input: &Vec<i32>) -> Result<Vec<i32>> {
    let mut mem = IntCode::init(input);
    let output = mem.run(&VecDeque::from(vec![1]))?;
    Ok(output.1)
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_basic() {
        let mut mem = IntCode::init(&vec![1,9,10,3,2,3,11,0,99,30,40,50]);
        assert_eq!(*mem.run(&VecDeque::new()).unwrap().0, vec![3500,9,10,70,2,3,11,0,99,30,40,50]);

        let mut mem = IntCode::init(&vec![1,0,0,0,99]);
        assert_eq!(*mem.run(&VecDeque::new()).unwrap().0, vec![2,0,0,0,99]);

        let mut mem = IntCode::init(&vec![2,3,0,3,99]);
        assert_eq!(*mem.run(&VecDeque::new()).unwrap().0, vec![2,3,0,6,99]);

        let mut mem = IntCode::init(&vec![2,4,4,5,99,0]);
        assert_eq!(*mem.run(&VecDeque::new()).unwrap().0, vec![2,4,4,5,99,9801]);

        let mut mem = IntCode::init(&vec![1,1,1,4,99,5,6,0,99]);
        assert_eq!(*mem.run(&VecDeque::new()).unwrap().0, vec![30,1,1,4,2,5,6,0,99]);
    }

    #[test]
    fn test_inout() {
        let mut mem = IntCode::init(&vec![3,0,4,0,3,1,4,1,99]);
        let run = mem.run(&VecDeque::from(vec![42, 58])).unwrap();
        assert_eq!(run.1, vec![42, 58]);
    }

}
