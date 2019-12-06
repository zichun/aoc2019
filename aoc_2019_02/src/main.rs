use std::io::{self};

enum Instruction {
    Add { left_op: usize, right_op: usize, into: usize },
    Mul { left_op: usize, right_op: usize, into: usize },
    Terminate,
}

struct Memory {
    memory: Vec<u32>,
    address_ptr: usize,
}

impl Memory {
    fn init(memory: &Vec<u32>) -> Memory {
        Memory {
            memory: memory.clone(),
            address_ptr: 0
        }
    }

    fn read_next(&mut self) -> Result<(Instruction), Box<dyn ::std::error::Error>> {
        let nextValue = self.memory.get(self.address_ptr).ok_or("Invalid Address")?;
        self.address_ptr++;
        Ok(nextValue)
    }

    fn read_instruction(&mut self) -> Result<(Instruction), Box<dyn ::std::error::Error>> {
        let opCode = self.memory.get(self.address_ptr).ok_or("Invalid Address")?;
        let instruction = match opCode {
            1 => {
                self.address_ptr += 4;
                Instruction::Add {
                    left_op: *self.memory.get(self.address_ptr - 3).ok_or("Invalid Address")? as usize,
                    right_op: *self.memory.get(self.address_ptr - 2).ok_or("Invalid Address")? as usize,
                    into: *self.memory.get(self.address_ptr - 1).ok_or("Invalid Address")? as usize,
                }
            }
            2 => {
                self.address_ptr += 4;
                Instruction::Mul {
                    left_op: *self.memory.get(self.address_ptr - 3).ok_or("Invalid Address")? as usize,
                    right_op: *self.memory.get(self.address_ptr - 2).ok_or("Invalid Address")? as usize,
                    into: *self.memory.get(self.address_ptr - 1).ok_or("Invalid Address")? as usize,
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

    fn run(&mut self) -> Result<(&Vec<u32>), Box<dyn ::std::error::Error>> {
        loop {
            let instruction = self.read_instruction()?;

            match instruction {
                Instruction::Add { left_op, right_op, into } => {
                    let sum = self.memory.get(left_op).ok_or("Invalid address reference")? + self.memory.get(right_op).ok_or("Invalid address reference")?;
                    let intoRef = self.memory.get_mut(into).ok_or("Invalid address reference")?;
                    *intoRef = sum;
                },
                Instruction::Mul { left_op, right_op, into } => {
                    let mul = self.memory.get(left_op).ok_or("Invalid address reference")? * self.memory.get(right_op).ok_or("Invalid address reference")?;
                    let intoRef = self.memory.get_mut(into).ok_or("Invalid address reference")?;
                    *intoRef = mul;
                }
                Instruction::Terminate => {
                    return Ok(&self.memory);
                }
            };
        }
    }
}

fn main() -> Result<(), Box<dyn ::std::error::Error>> {
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let input: Vec<u32> = input
        .split(",")
        .filter_map(|s|
                    s.trim().parse().ok()
        ).collect();

    println!("Day1: {}", day1(&input)?);
    println!("Day2: {:?}", day2(&input)?);

    Ok(())
}

fn day1(input: &Vec<u32>) -> Result<(u32), Box<dyn ::std::error::Error>> {
    let mut mutInput = input.clone();

    mutInput[1] = 12;
    mutInput[2] = 2;

    let mut mem = Memory::init(&mutInput);
    let output = mem.run()?;

    Ok(output[0])
}

fn day2(input: &Vec<u32>) -> Result<(u32, u32), Box<dyn ::std::error::Error>> {
    for noun in 0..99 {
        for verb in 0..99 {
            let mut testInput = input.clone();
            testInput[1] = noun;
            testInput[2] = verb;
            let mut mem = Memory::init(&testInput);
            match mem.run() {
                Ok(output) => {
                    if output[0] == 19690720 {
                        return Ok((noun, verb));
                    }
                }
                Err(error) => {
                    continue;
                }
            }
        }
    }
    Err("Fail to find pair".into())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_memory() {
        let mut mem = Memory::init(&vec![1,9,10,3,2,3,11,0,99,30,40,50]);
        assert_eq!(*mem.run().unwrap(), vec![3500,9,10,70,2,3,11,0,99,30,40,50]);

        let mut mem = Memory::init(&vec![1,0,0,0,99]);
        assert_eq!(*mem.run().unwrap(), vec![2,0,0,0,99]);

        let mut mem = Memory::init(&vec![2,3,0,3,99]);
        assert_eq!(*mem.run().unwrap(), vec![2,3,0,6,99]);

        let mut mem = Memory::init(&vec![2,4,4,5,99,0]);
        assert_eq!(*mem.run().unwrap(), vec![2,4,4,5,99,9801]);

        let mut mem = Memory::init(&vec![1,1,1,4,99,5,6,0,99]);
        assert_eq!(*mem.run().unwrap(), vec![30,1,1,4,2,5,6,0,99]);
    }

}
