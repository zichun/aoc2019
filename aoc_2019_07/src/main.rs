use std::io::{self};
use std::collections::VecDeque;
use std::collections::HashSet;

type Result<T> = ::std::result::Result<T, Box<dyn ::std::error::Error>>;

#[derive(Debug,PartialEq)]
enum ParameterType {
    Ref(usize),
    Value(i32)
}

enum Instruction {
    Add { left_op: ParameterType, right_op: ParameterType, into: ParameterType },
    Mul { left_op: ParameterType, right_op: ParameterType, into: ParameterType },
    Inputf { into: ParameterType },
    Output { param: ParameterType },
    JumpIfTrue { cond: ParameterType, to: ParameterType },
    JumpIfFalse { cond: ParameterType, to: ParameterType },
    LessThan { left_op: ParameterType, right_op: ParameterType, into: ParameterType },
    Equals { left_op: ParameterType, right_op: ParameterType, into: ParameterType },
    Terminate,
}

struct IntCode {
    memory: Vec<i32>,
    address_ptr: usize,
    input_stream: InputStream,
    output_buffer: VecDeque<i32>,
    is_terminated: bool
}

struct OutputStream<'a>(&'a mut IntCode<'a>);

impl<'a> Iterator for OutputStream<'a> {
    type Item = i32;
    fn next(&mut self) -> Option<i32> {
        if self.0.output_buffer.len() > 0 {
            self.0.output_buffer.pop_front()
        } else {
            self.0.run_to_next_output()
        }
    }
}

struct InputStream(VecDeque<i32>, Option<&mut OutputStream>);

impl Iterator for InputStream {
    type Item = i32;
    fn next(&mut self) -> Option<i32> {
        if self.0.len() > 0 {
            self.0.pop_front()
        } else if let Some(s) = &mut self.1 {
            s.next()
        } else {
            None
        }
    }
}

impl IntCode {
    fn init(memory: &Vec<i32>, input_stream: InputStream) -> IntCode {
        IntCode {
            memory: memory.clone(),
            address_ptr: 0,
            input_stream: None,
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

    fn output_stream(self) -> OutputStream {
        OutputStream(self)
    }

    fn run_to_next_output(&mut self) -> Option<i32> {
        None
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

    fn run_to_termination(&mut self) -> Result<()> {
        loop {
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
                    if let Some(s) = &mut self.input_stream {
                        let input_value = s.next().ok_or("Ran out of input")?;
                        println!("INPUT VALUE: {}", input_value);
                        self.write_memory(into, input_value)?;
                    } else {
                        return Err("No input streams defined".into());
                    }
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
                    return Ok(());
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

fn run_amps(input: &Vec<i32>, phase_settings: &Vec<usize>) -> Result<i32> {
    let mut amps: Vec<IntCode> = Vec::new();
    let mut amp_inputs: Vec<InputStream> = Vec::new();
    let mut prev_output: i32 = 0;

    for i in 0..phase_settings.len() {
        amps.push(IntCode::init(&input));
        amp_inputs.push(InputStream(VecDeque::from(vec![phase_settings[i] as i32]), None));
    }

    let mut input_slices = amp_inputs.iter_mut();

    for i in 0..phase_settings.len() {
        let x = input_slices.next().unwrap();
        x.0.push_back(prev_output);
        amps[i].set_input_stream(&mut *x);
        amps[i].run_to_termination().unwrap();
        prev_output = *amps[i].output_buffer.get(0).ok_or("Program did not produce an output")?;
    }

    Ok(prev_output)
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
    let mut amps: Vec<IntCode> = Vec::new();
    let mut amp_inputs: Vec<InputStream> = Vec::new();
    let mut prev_output: i32 = 0;

    for i in 0..phase_settings.len() {
        amps.push(IntCode::init(&input));
        amp_inputs.push(InputStream(VecDeque::from(vec![phase_settings[i] as i32]), None));
    }

    {
        let mut input_slices = amp_inputs.iter_mut();

        for i in 0..phase_settings.len() {
            let x = input_slices.next().unwrap();
            amps[i].set_input_stream(&mut *x);
        }
    }

    let mut output_streams: Vec<OutputStream> = Vec::new();
    let mut amps_slices = amps.iter_mut();
    for i in 0..phase_settings.len() {
        let a = amps_slices.next().unwrap();
        output_streams.push(a.output_stream());
    }

    {
        let mut input_slices = amp_inputs.iter_mut();

        for i in 0..phase_settings.len() {
            let x = input_slices.next().unwrap();
        }
    }
    Ok(prev_output)
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
}
