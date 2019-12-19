use std::io::{self};
use std::collections::VecDeque;
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

    let ans = part1_and_2(&input)?;
    println!("{}\n{}", ans.0, ans.1);

    Ok(())
}

#[derive(Debug,PartialEq)]
enum ExploreState {
    Room(usize),
    Wall,
    Unknown
}

struct Room {
    up: ExploreState,
    down: ExploreState,
    left: ExploreState,
    right: ExploreState
}

impl Room {
    fn new() -> Room {
        Room {
            up: ExploreState::Unknown,
            down: ExploreState::Unknown,
            left: ExploreState::Unknown,
            right: ExploreState::Unknown
        }
    }

    fn next_unexplored(&self) -> Option<usize> {
        if self.up == ExploreState::Unknown {
            Some(UP_INDEX)
        } else if self.down == ExploreState::Unknown {
            Some(DOWN_INDEX)
        } else if self.left == ExploreState::Unknown {
            Some(LEFT_INDEX)
        } else if self.right == ExploreState::Unknown {
            Some(RIGHT_INDEX)
        } else {
            None
        }
    }

    fn adjacent(&self) -> Vec<usize> {
        let mut rooms = Vec::new();
        if let ExploreState::Room(r) = self.up {
            rooms.push(r);
        }
        if let ExploreState::Room(r) = self.down {
            rooms.push(r);
        }
        if let ExploreState::Room(r) = self.left {
            rooms.push(r);
        }
        if let ExploreState::Room(r) = self.right {
            rooms.push(r);
        }
        rooms
    }
}

struct MapState(Vec<Room>, usize);

const UP_INDEX: usize = 1;
const DOWN_INDEX: usize = 2;
const LEFT_INDEX: usize = 3;
const RIGHT_INDEX: usize = 4;

impl MapState {
    fn get_room_dir_mut<'a>(room: &'a mut Room, dir: &usize, flip: bool) -> Result<&'a mut ExploreState> {
        let mut new_dir = *dir;
        if flip == true {
            new_dir = MapState::flip(dir);
        }

        Ok(match new_dir {
            UP_INDEX => &mut room.up,
            DOWN_INDEX => &mut room.down,
            LEFT_INDEX => &mut room.left,
            RIGHT_INDEX => &mut room.right,
            _ => { return Err("Invalid room direction!".into()); }
        })

    }
    fn insert_wall(&mut self, dir: usize) -> Result<()> {
        let from = self.1;
        let curr_room = self.0 .get_mut(from).ok_or("Invalid room index")?;
        let dir_ref = MapState::get_room_dir_mut(curr_room, &dir, false)?;
        if *dir_ref != ExploreState::Unknown {
            return Err("room direction already exists".into());
        }
        *dir_ref = ExploreState::Wall;

        Ok(())
    }

    fn insert_room_and_move(&mut self, dir: usize) -> Result<usize> {
        let new_room_index = self.0.len();
        let from = self.1;

        {
            let curr_room = self.0.get_mut(from).ok_or("Invalid room index")?;
            let dir_ref = MapState::get_room_dir_mut(curr_room, &dir, false)?;

            if *dir_ref == ExploreState::Wall {
                return Err("walking into a wall".into());
            } else if let ExploreState::Room(that_room) = *dir_ref {
                // room already exists, just move to that room.
                self.1 = that_room;
                return Ok(that_room);
            }
            *dir_ref = ExploreState::Room(new_room_index);
        }

        {
            let mut new_room = Room::new();
            let dir_ref = MapState::get_room_dir_mut(&mut new_room, &dir, true)?;

            *dir_ref = ExploreState::Room(self.1);
            self.0.push(new_room);
        }

        self.1 = new_room_index;

        Ok(new_room_index)
    }

    fn next_unexplored(&self) -> Result<Option<usize>> {
        let from = self.1;
        let curr_room = self.0.get(from).ok_or("Invalid room index")?;
        Ok(curr_room.next_unexplored())
    }

    fn flip(dir: &usize) -> usize {
        match *dir {
            UP_INDEX => DOWN_INDEX,
            DOWN_INDEX => UP_INDEX,
            LEFT_INDEX => RIGHT_INDEX,
            RIGHT_INDEX => LEFT_INDEX,
            _ => { panic!("bad direction"); }
        }
    }

    fn new() -> MapState {
        MapState(vec![Room::new()], 0)
    }

    fn last_index(&self) -> usize {
        self.0.len()
    }
}

fn part1_and_2(input: &Vec<i64>) -> Result<(usize, usize)> {
    // the follow code assumes that the maze forms a tree
    let map_state_cell = RefCell::new(MapState::new());
    let is_complete = RefCell::new(false);
    let last_move = RefCell::new(0 as usize);
    let breadcrumps = RefCell::new(Vec::new());

    let machine = IntCode::init(input, from_fn(|| {
        let next_dir = map_state_cell.borrow().next_unexplored().unwrap();
        if let Some(next_dir) = next_dir {
            *last_move.borrow_mut() = next_dir;
            Some(next_dir as i64)
        } else {
            if breadcrumps.borrow().len() == 0 {
                // Completed search and we're back to origin. Walk a random direction
                *is_complete.borrow_mut() = true;
                Some(1)
            } else {
                let last = breadcrumps.borrow_mut().pop().unwrap();
                *last_move.borrow_mut() = last;
                Some(last as i64)
            }
        }
    }));

    let mut output = machine.output_stream();
    let mut part1_answer = 0;
    let mut goal_index = 0;

    while *is_complete.borrow() == false {
        let result = output.next().unwrap();

        match result {
            0 => { // Wall
                if let Err(e) = map_state_cell.borrow_mut().insert_wall(*last_move.borrow()) {
                    if *is_complete.borrow() == false {
                        return Err(e);
                    }
                }
            }
            1 => { // New Room
                let new_index = map_state_cell.borrow_mut().insert_room_and_move(*last_move.borrow())?;
                if new_index + 1 == map_state_cell.borrow().last_index() {
                    breadcrumps.borrow_mut().push(MapState::flip(&last_move.borrow()));
                }
            }
            2 => { // Goal Room
                let new_index = map_state_cell.borrow_mut().insert_room_and_move(*last_move.borrow())?;
                if new_index + 1 == map_state_cell.borrow().last_index() {
                    breadcrumps.borrow_mut().push(MapState::flip(&last_move.borrow()));
                }
                goal_index = new_index;
                part1_answer = breadcrumps.borrow().len();
            }
            _ => {
                return Err("Bad output!".into());
            }
        }
    }

    let part2_answer = part2(&map_state_cell.borrow(), goal_index)?;

    Ok((part1_answer, part2_answer))
}

struct QueueEle {
    room_index: usize,
    tick: usize
}

fn part2(map: &MapState, goal_index: usize) -> Result<usize> {
    let mut queue = VecDeque::new();
    let mut visited = vec![false; map.0.len()];

    visited[goal_index] = true;
    queue.push_back(QueueEle {
        room_index: goal_index,
        tick: 0
    });

    let mut ans = 0;
    while queue.len() > 0 {
        let top = queue.pop_front().unwrap();
        let room = map.0.get(top.room_index).ok_or("Invalid index")?;
        let adj_rooms = room.adjacent();

        for r in adj_rooms {
            if visited[r] == false {
                visited[r] = true;
                queue.push_back(QueueEle {
                    room_index: r,
                    tick: top.tick + 1
                });
            }
        }
        ans = top.tick;
    }

    Ok(ans)
}
