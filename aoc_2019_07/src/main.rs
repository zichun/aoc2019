use std::io::{self};
use std::collections::VecDeque;
use std::collections::HashSet;
use std::iter::*;
use std::cell::RefCell;

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
    JumpIfTrue { cond: ParameterType, to: ParameterType },
    JumpIfFalse { cond: ParameterType, to: ParameterType },
    LessThan { left_op: ParameterType, right_op: ParameterType, into: ParameterType },
    Equals { left_op: ParameterType, right_op: ParameterType, into: ParameterType },
    Terminate,
}

struct IntCode<T: Iterator> {
    memory: Vec<i32>,
    address_ptr: usize,
    input_stream: T,
    output_buffer: VecDeque<i32>,
    is_terminated: bool
}

struct OutputStream<T: Iterator>(IntCode<T>);

impl<T> Iterator for OutputStream<T> where
    T: Iterator<Item = i32>
{
    type Item = i32;
    fn next(&mut self) -> Option<i32> {
        if self.0.output_buffer.len() > 0 {
            self.0.output_buffer.pop_front()
        } else {
            self.0.run_to_next_output()
        }
    }
}

impl<T> IntCode<T> where
    T: Iterator<Item = i32> {
    fn init(memory: &Vec<i32>, input_stream: T) -> IntCode<T> {
        IntCode {
            memory: memory.clone(),
            address_ptr: 0,
            input_stream: input_stream,
            output_buffer: VecDeque::new(),
            is_terminated: false
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

    fn output_stream(self) -> OutputStream<T> {
        OutputStream(self)
    }

    fn run_to_next_output(&mut self) -> Option<i32> {
        while self.output_buffer.len() == 0 && self.is_terminated == false {
            // bad code; output iterator should be a result
            self.run_tick().unwrap();
        }

        println!("{:?}", self.output_buffer);
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

    let input: Vec<i32> = input
        .split(",")
        .filter_map(|s|
                    s.trim().parse().ok()
        ).collect();

    Ok(())
}

fn run_amps(input: &Vec<i32>, phase_settings: &Vec<usize>) -> Result<i32> {
    let amp_0 = IntCode::init(&input,
                              once(phase_settings[0] as i32)
                              .chain(once(0)));
    let amp_1 = IntCode::init(&input,
                              once(phase_settings[1] as i32)
                              .chain(amp_0.output_stream()));
    let amp_2 = IntCode::init(&input,
                              once(phase_settings[2] as i32)
                              .chain(amp_1.output_stream()));
    let amp_3 = IntCode::init(&input,
                              once(phase_settings[3] as i32)
                              .chain(amp_2.output_stream()));
    let amp_4 = IntCode::init(&input,
                              once(phase_settings[4] as i32)
                              .chain(amp_3.output_stream()));

    amp_4.output_stream().next().ok_or("No output".into())
}

fn all_permutation(input: &Vec<i32>, collection: &mut HashSet<usize>, builder: &mut Vec<usize>, f: &dyn Fn(&Vec<i32>, &Vec<usize>) -> Result<i32>) -> i32 {
    let items: Vec<usize> = collection.iter().cloned().collect();

    if collection.len() == 0 {
        let tr = f(input, builder).unwrap_or(<i32>::min_value());
        return tr;
    }

    let mut max: i32 = <i32>::min_value();

    for ele in items {
        collection.remove(&ele);
        builder.push(ele);

        let curr = all_permutation(input, collection, builder, f);
        if curr > max {
            max = curr;
        }

        builder.pop();
        collection.insert(ele);
    }

    max
}

fn part1(input: &Vec<i32>) -> i32 {
    let mut collection: HashSet<usize> = (0..5).collect();
    all_permutation(input, &mut collection, &mut vec![], &run_amps)
}

fn run_amps_part2(input: &Vec<i32>, phase_settings: &Vec<usize>) -> Result<i32> {
    // adapted from https://github.com/Awfa/advent_of_code_2019/blob/master/src/day7.rs
    let pipe = RefCell::new(VecDeque::<i32>::new());

    let amp_0 = IntCode::init(&input,
                              once(phase_settings[0] as i32)
                              .chain(once(0))
                              .chain(from_fn(|| {
                                  Some(pipe.borrow_mut().pop_front().unwrap())
                              })));
    let amp_1 = IntCode::init(&input,
                              once(phase_settings[1] as i32)
                              .chain(amp_0.output_stream()));
    let amp_2 = IntCode::init(&input,
                              once(phase_settings[2] as i32)
                              .chain(amp_1.output_stream()));
    let amp_3 = IntCode::init(&input,
                              once(phase_settings[3] as i32)
                              .chain(amp_2.output_stream()));
    let amp_4 = IntCode::init(&input,
                              once(phase_settings[4] as i32)
                              .chain(amp_3.output_stream()));
    let amp_4_output = amp_4.output_stream().map(|value| {
        pipe.borrow_mut().push_back(value);
        value
    });
    amp_4_output.last().ok_or("No output".into())
}

fn part2(input: &Vec<i32>) -> i32 {
    let mut collection: HashSet<usize> = (5..10).collect();
    all_permutation(input, &mut collection, &mut vec![], &run_amps_part2)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_amp() {
        assert_eq!(run_amps(&vec![3,15,3,16,1002,16,10,16,1,16,15,15,4,15,99,0,0], &vec![4,3,2,1,0]).unwrap(), 43210);
        assert_eq!(run_amps(&vec![3,23,3,24,1002,24,10,24,1002,23,-1,23,101,5,23,23,1,24,23,23,4,23,99,0,0], &vec![0,1,2,3,4]).unwrap(), 54321);
        assert_eq!(run_amps(&vec![3,31,3,32,1002,32,10,32,1001,31,-2,31,1007,31,0,33,1002,33,7,33,1,33,31,31,1,32,31,31,4,31,99,0,0,0], &vec![1,0,4,3,2]).unwrap(), 65210);
    }

    #[test]
    fn test_part1() {
        assert_eq!(part1(&vec![3,15,3,16,1002,16,10,16,1,16,15,15,4,15,99,0,0]), 43210);
        assert_eq!(part1(&vec![3,23,3,24,1002,24,10,24,1002,23,-1,23,101,5,23,23,1,24,23,23,4,23,99,0,0]), 54321);
        assert_eq!(part1(&vec![3,31,3,32,1002,32,10,32,1001,31,-2,31,1007,31,0,33,1002,33,7,33,1,33,31,31,1,32,31,31,4,31,99,0,0,0]), 65210);
    }

    #[test]
    fn test_part2() {
        assert_eq!(part2(&vec![3,26,1001,26,-4,26,3,27,1002,27,2,27,1,27,26,27,4,27,1001,28,-1,28,1005,28,6,99,0,0,5]), 139629729);
        assert_eq!(part2(&vec![3,52,1001,52,-5,52,3,53,1,52,56,54,1007,54,5,55,1005,55,26,1001,54,-5,54,1105,1,12,1,53,54,53,1008,54,0,55,1001,55,1,55,2,53,55,53,4,53,1001,56,-1,56,1005,56,6,99,0,0,0,0,10]), 18216);
    }
}
