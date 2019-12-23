use std::io::{self};
use std::collections::VecDeque;
use std::collections::HashSet;
use std::iter::*;
use std::collections::HashMap;

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

fn part1(input: &Vec<i64>) -> Result<i64> {
    let output = "NOT A J\nNOT C T\nOR T J\nAND D J\nWALK\n";
    let input_stream = output.chars().map(|x| x as i64);
    let machine = IntCode::init(&input, input_stream);
    let output: Vec<i64> = machine.output_stream().collect();
    Ok(output[output.len() - 1])
}

fn convert_to_hole(mask: &u16) -> Vec<bool> {
    let mut tr = Vec::new();
    for i in 0..9 {
        tr.push(mask & (1 << i) != 0);
    }
    tr
}

fn splice(holes: &Vec<bool>, start: usize) -> Vec<bool> {
    let mut tr = Vec::new();
    for i in start..holes.len() {
        tr.push(holes[i]);
    }
    tr
}

fn should_jump(holes: &Vec<bool>) -> (bool, usize) {
    if holes.len() == 0 {
        return (false, 1);
    }

    let mut can_walk = true;
    let mut can_jump = true;
    if holes[0] == false {
        can_walk = false;
    }
    if holes.len() < 4 || holes[3] == false {
        can_jump = false;
    }

    let mut jump = 0;
    let mut walk = 0;

    if can_walk {
        let new_hole = splice(holes, 1);
        let x = should_jump(&new_hole);
        walk = x.1;
    }
    if can_jump {
        let new_hole = splice(holes, 4);
        let x = should_jump(&new_hole);
        jump = x.1;
    }

    (jump >= walk, walk + jump)
}

#[derive(Debug,PartialEq,Clone,Copy)]
enum ComplementField {
    True,
    False,
    WildCard
}
#[derive(Debug)]
struct Complements(Vec<ComplementField>);

fn my_copy(min_terms: &MinTerms, complements: &Complements) -> (MinTerms, Complements) {
    let mut mt = Vec::new();
    for x in &min_terms.0 {
        mt.push(*x);
    }

    let mut c = Vec::new();
    for x in &complements.0 {
        c.push(*x);
    }

    (MinTerms(mt), Complements(c))
}

impl Complements {
    fn union(left: &Complements, right: &Complements) -> Complements {
        let left = &left.0;
        let right = &right.0;
        assert_eq!(left.len(), right.len());

        let mut tr = Vec::new();
        let mut count = 0;
        for i in 0..left.len() {
            if left[i] == ComplementField::WildCard {
                assert_eq!(left[i], right[i]);
                tr.push(ComplementField::WildCard);
            } else if left[i] != right[i] {
                tr.push(ComplementField::WildCard);
                count = count + 1;
            } else {
                tr.push(left[i]);
            }
        }
        assert_eq!(count, 1);
        Complements(tr)
    }

    fn differ_by_one(left: &Complements, right: &Complements) -> bool {
        let left = &left.0;
        let right = &right.0;

        assert_eq!(left.len(), right.len());

        let mut diff = 0;

        for i in 0..left.len() {
            if left[i] != ComplementField::WildCard && right[i] != ComplementField::WildCard {
                if left[i] != right[i] {
                    diff = diff + 1;
                }
            } else {
                if left[i] != right[i] {
                    return false;
                }
            }
        }

        diff == 1
    }
}

#[derive(PartialEq, Eq, Hash, Debug)]
struct MinTerms(Vec<u16>);

impl MinTerms {
    fn union(left: &MinTerms, right: &MinTerms) -> MinTerms {
        let mut collect = HashSet::new();

        for l in &left.0 {
            collect.insert(*l);
        }
        for r in &right.0 {
            collect.insert(*r);
        }

        let mut terms: Vec<u16> = collect.iter().copied().collect();
        terms.sort();

        MinTerms(terms)
    }
    fn len(&self) -> usize {
        self.0.len()
    }
}

fn part2(input: &Vec<i64>) -> Result<i64> {
    const N: u16 = (1 << 9);
    let mut minterms = Vec::new();
    let mut complements: Vec<HashMap<MinTerms, Complements>> = Vec::new();

    complements.push(HashMap::new());

    for i in 0..N {
        let holes = convert_to_hole(&i);
        let jump = should_jump(&holes);
        let jump = jump.0;
        println!("{} {:?} {}", i, holes, jump);
        if jump {
            minterms.push(i);
            let complement: Vec<ComplementField> = holes.iter().map(|x| match x { true => ComplementField::True, false => ComplementField::False }).collect();
            complements[0].insert(MinTerms(vec![i]), Complements(complement));
        }
    }

    //
    // find prime implicants
    //
    let mut cur_index = 0;
    let mut prime_implicants = Vec::new();

    loop {
        //
        // Collect implicants
        //
        let mut new_complements = HashMap::new();

        {
            let mut implicants = Vec::new();
            for (minterms, complement) in complements[cur_index].iter() {
                implicants.push((minterms, complement));
            }

            for i in 0..implicants.len() {
                let mut found = false;

                for j in 0..implicants.len() {
                    if i == j { continue; }
                    if Complements::differ_by_one(&implicants[i].1, &implicants[j].1) {
                        let union = MinTerms::union(&implicants[i].0, &implicants[j].0);
                        if union.len() == implicants[i].0.len() + implicants[j].0.len() {
                            found = true;
                            if !new_complements.contains_key(&union) {
                                new_complements.insert(union, Complements::union(&implicants[i].1, &implicants[j].1));
                            }
                        }
                    }
                }

                if !found {
                    prime_implicants.push(my_copy(implicants[i].0, implicants[i].1));
                }
            }
        }

        if new_complements.len() > 0 {
            complements.push(new_complements);
        } else {
            break;
        }

        cur_index = cur_index + 1;
    }

    for p in prime_implicants {
        let mut term = String::new();
        for i in 0..(p.1).0.len() {
            let cur = (i + 65) as u8 as char;
            if (p.1).0[i] == ComplementField::True {
                term = term + &cur.to_string();
            } else if (p.1).0[i] == ComplementField::False {
                term = term + &cur.to_string() + "'";
            }
        }
        println!("{:?} {:?}", p.0, p.1);
        println!("{}", term);
    }

// E'(B' AND H' AND G')
    let output = "OR C T
OR E T
OR F T
NOT T T
OR T J
NOT C T
AND D T
OR T J
NOT B T
AND D T
OR T J
NOT A T
OR T J
NOT I T
OR T J
RUN\n";
    let input_stream = output.chars().map(|x| x as i64);
    let machine = IntCode::init(&input, input_stream);
    let output: Vec<i64> = machine.output_stream().collect();
    let output_string: String = output.iter().map(|x| (*x as u8) as char).collect();
    println!("{}", output_string);
//    Ok(output[output.len() - 1])
    Ok(1)
}

#[cfg(tests)]
mod test {
    use super::*;
    #[test]
    fn test_should_jump() {
//        assert_eq!(should_jump(vec![true, true, true, true, true, true, true, true, true]).0, false);
        assert_eq!(should_jump(vec![true, true, true, true, false, true, true, true, false]).0, false);
    }
}
/*
.................
.................
@................
#####.###.#..####
 ABCDEFGHI
*/
// 367 [true, true, true, true, false, true, true, false, true] false
// 239 [true, true, true, true, false, true, true, true, false] true
//MinTerms([136, 137, 138, 139, 140, 141, 142, 143, 152, 153, 154, 155, 156, 157, 158, 159, 168, 169, 170, 171, 172, 173, 174, 175, 184, 185, 186, 187, 188, 189, 190, 191, 200, 201, 202, 203, 204, 205, 206, 207, 216, 217, 218, 219, 220, 221, 222, 223, 232, 233, 234, 235, 236, 237, 238, 239, 248, 249, 250, 251, 252, 253, 254, 255, 392, 393, 394, 395, 396, 397, 398, 399, 408, 409, 410, 411, 412, 413, 414, 415, 424, 425, 426, 427, 428, 429, 430, 431, 440, 441, 442, 443, 444, 445, 446, 447, 456, 457, 458, 459, 460, 461, 462, 463, 472, 473, 474, 475, 476, 477, 478, 479, 488, 489, 490, 491, 492, 493, 494, 495, 504, 505, 506, 507, 508, 509, 510, 511]) Complements([WildCard, WildCard, WildCard, True, WildCard, WildCard, WildCard, True, WildCard])
