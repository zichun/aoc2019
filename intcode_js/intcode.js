function assert_eq(a, b) {
    if (a !== b) {
        throw("a != b");
    }
    return a;
}
function assert_arr_eq(a, b) {
    if (a.length !== b.length) throw("length diff");
    for (let i = 0; i < a.length; ++i) {
        if (a[i] !== b[i]) throw("a[" + i + "]: " + a[i] + " b[" + i + "]: " + b[i]);
    }
}

class OpCode {
    constructor(param_mode) {
        this.param_mode = param_mode;
    }
    resolve(memory, arg_value) {
        let tr = [];
        const params = this.params();
        if (arg_value.length !== params.length) {
            throw("memory read error, length unequal");
        }
        for (let i = 0; i < arg_value.length; ++i) {
            const pm = this.param_mode.car();
            if (pm === 0) {
                if (params[i] === 1) {
                    tr.push(arg_value[i]);
                } else {
                    tr.push(memory.read_at(arg_value[i]));
                }
            } else {
                if (params[i] === 1) {
                    throw("parameter_mode invalid for write operation");
                }
                tr.push(arg_value[i]);
            }
        }
//        console.debug(this.constructor.name + ": " + tr);
        return tr;
    }
};

class Add extends OpCode {
    params() {
        return [0, 0, 1];
    }
    execute(machine, arg_value) {
        let arg_value_resolved = this.resolve(machine.memory, arg_value);
        machine.memory.set(arg_value_resolved[2], arg_value_resolved[0] + arg_value_resolved[1]);
        return false;
    }
}
class Mul extends OpCode {
    params() {
        return [0, 0, 1];
    }
    execute(machine, arg_value) {
        let arg_value_resolved = this.resolve(machine.memory, arg_value);
        machine.memory.set(arg_value_resolved[2], arg_value_resolved[0] * arg_value_resolved[1]);
        return false;
    }
}
class Term extends OpCode {
    params() { return []; }
    execute() {
        return true;
    }
}
class Input extends OpCode {
    params() { return [1]; }
    execute(machine, arg_value, output_buffer, input_stream) {
        let arg_value_resolved = this.resolve(machine.memory, arg_value);
        machine.memory.set(arg_value_resolved[0], input_stream.car());
    }
}
class Output extends OpCode {
    params() { return [0]; }
    execute(machine, arg_value, output_buffer) {
        let arg_value_resolved = this.resolve(machine.memory, arg_value);
        output_buffer.push(arg_value_resolved[0]);
        return false;
    }
}
class JumpIfTrue extends OpCode {
    params() { return [0, 0]; }
    execute(machine, arg_value) {
        let arg_value_resolved = this.resolve(machine.memory, arg_value);
        if (arg_value_resolved[0] !== 0) {
            machine.memory_ptr = arg_value_resolved[1];
        }
        return false;
    }
}
class JumpIfFalse extends OpCode {
    params() { return [0, 0]; }
    execute(machine, arg_value) {
        let arg_value_resolved = this.resolve(machine.memory, arg_value);
        if (arg_value_resolved[0] === 0) {
            machine.memory_ptr = arg_value_resolved[1];
        }
        return false;
    }
}
class LessThan extends OpCode {
    params() { return [0, 0, 1]; }
    execute(machine, arg_value) {
        let arg_value_resolved = this.resolve(machine.memory, arg_value);
        machine.memory.set(arg_value_resolved[2], arg_value_resolved[0] < arg_value_resolved[1] ? 1 : 0);
        return false;
    }
}
class Equals extends OpCode {
    params() { return [0, 0, 1]; }
    execute(machine, arg_value) {
        let arg_value_resolved = this.resolve(machine.memory, arg_value);
        machine.memory.set(arg_value_resolved[2], arg_value_resolved[0] === arg_value_resolved[1] ? 1 : 0);
        return false;
    }
}

class Stream {
    constructor(closure) {
        let wrapClosure = () => {
            return [closure(), wrapClosure];
        };
        this.closure = wrapClosure;
    }

    car() {
        if (this.closure) {
            const tr = this.closure();
            this.closure = tr[1];
            return tr[0];
        } else {
            return null;
        }
    }
}
function stream_to_list(stream) {
    let tr = [];
    while (true) {
        let x = stream.car();
        if (x !== null) {
            tr.push(x);
        } else {
            break;
        }
    }
    return tr;
}
function list_to_stream(list) {
    let lst = list;
    return new Stream(() => {
        if (lst.length === 0) {
            return null;
        } else {
            const tr = lst[0];
            lst = lst.slice(1);
            return tr;
        }
    });
}
function cons(val, cdrStream) {
    // returns a new Stream with (x, Stream)
    let s = new Stream;
    s.closure = () => {
        return [val, cdrStream.closure];
    };
    return s;
}

class Memory {
    constructor(input) {
        this.memory = input.slice(0);
    }

    read_at(index) {
        return this.memory[index];
    }

    set(address, value) {
        this.memory[address] = value;
    }
}

class IntCodeMachine {

    constructor(memory) {
        this.memory = memory;
        this.memory_ptr = 0;
        this.output_buffer = [];
        this.is_terminal = false;
        this.input_stream = null;
    }

    set_input_stream(input_stream) {
        this.input_stream = input_stream;
    }

    next_instruction() {
        if (this.is_terminal) {
            return true;
        }
        const that = this;

        function parse_opcode(opcode) {
            let params = Math.floor(opcode / 100);

            let param_mode = new Stream(() => {
                const tr = params % 10;
                params = Math.floor(params / 10);
                return tr;
            });

            switch (opcode % 100) {
            case 1:
                return new Add(param_mode);
            case 2:
                return new Mul(param_mode);
            case 3:
                return new Input(param_mode);
            case 4:
                return new Output(param_mode);
            case 5:
                return new JumpIfTrue(param_mode);
            case 6:
                return new JumpIfFalse(param_mode);
            case 7:
                return new LessThan(param_mode);
            case 8:
                return new Equals(param_mode);
            case 99:
                return new Term(param_mode);
            default:
                throw("Unrecognized opcode " + opcode);
            }
        }


        const op = parse_opcode(this.memory.read_at(this.memory_ptr++));

        const params = op.params();
        const args = params.map(is_ref => {
            return that.memory.read_at(that.memory_ptr++);
        });

        if (op.execute(this, args, this.output_buffer, this.input_stream) === true) {
            this.is_terminal = true;
        }

        return this.is_terminal;
    }

    run_to_terminal() {
        while (!this.is_terminal) {
            this.next_instruction();
        }
    }

    run_to_next_output() {
        while (!this.is_terminal && this.output_buffer.length === 0) {
            this.next_instruction();
        }
    }

    get_output_stream() {
        return new Stream(() => {
            if (this.output_buffer.length === 0) {
                this.run_to_next_output();
            }

            if (this.output_buffer.length === 0) {
                return null;
            } else {
                const output = this.output_buffer[0];
                this.output_buffer = this.output_buffer.slice(1);
                return output;
            }
        });
    }
}

function test_basic() {
    {
        let machine = new IntCodeMachine(new Memory([1,1,1,4,99,5,6,0,99]));
        machine.run_to_terminal();
        assert_arr_eq(machine.memory.memory, [ 30, 1, 1, 4, 2, 5, 6, 0, 99 ]);
    }

    {
        let machine = new IntCodeMachine(new Memory([2,4,4,5,99,0]));
        machine.run_to_terminal();
        assert_arr_eq(machine.memory.memory, [ 2,4,4,5,99,9801  ]);
    }
}

function test_io() {
    {
        let machine = new IntCodeMachine(new Memory([3,9,8,9,10,9,4,9,99,-1,8]));
        let output_stream = machine.get_output_stream();
        machine.set_input_stream(list_to_stream([8]));
        machine.run_to_terminal();
        assert_arr_eq(stream_to_list(output_stream), [ 1 ]);
    }

    {
        let machine = new IntCodeMachine(new Memory([3,9,8,9,10,9,4,9,99,-1,8]));
        let output_stream = machine.get_output_stream();
        machine.set_input_stream(list_to_stream([10]));
        machine.run_to_terminal();
        assert_arr_eq(stream_to_list(output_stream), [ 0 ]);
    }

    {
        let machine = new IntCodeMachine(new Memory([3,9,7,9,10,9,4,9,99,-1,8]));
        let output_stream = machine.get_output_stream();
        machine.set_input_stream(list_to_stream([7]));
        machine.run_to_terminal();
        assert_arr_eq(stream_to_list(output_stream), [ 1 ]);
    }

    {
        let machine = new IntCodeMachine(new Memory([3,9,7,9,10,9,4,9,99,-1,8]));
        let output_stream = machine.get_output_stream();
        machine.set_input_stream(list_to_stream([9]));
        machine.run_to_terminal();
        assert_arr_eq(stream_to_list(output_stream), [ 0 ]);
    }

    {
        let machine = new IntCodeMachine(new Memory([3,3,1108,-1,8,3,4,3,99]));
        let output_stream = machine.get_output_stream();
        machine.set_input_stream(list_to_stream([8]));
        machine.run_to_terminal();
        assert_arr_eq(stream_to_list(output_stream), [ 1 ]);
    }
    {
        let machine = new IntCodeMachine(new Memory([3,3,1108,-1,8,3,4,3,99]));
        let output_stream = machine.get_output_stream();
        machine.set_input_stream(list_to_stream([10]));
        machine.run_to_terminal();
        assert_arr_eq(stream_to_list(output_stream), [ 0 ]);
    }
}

function test_jump() {
    {
        let machine = new IntCodeMachine(new Memory([3,12,6,12,15,1,13,14,13,4,13,99,-1,0,1,9]));
        machine.set_input_stream(list_to_stream([0]));
        machine.run_to_terminal();
        assert_arr_eq(machine.output_buffer, [ 0 ]);
    }
    {
       let machine = new IntCodeMachine(new Memory([3,12,6,12,15,1,13,14,13,4,13,99,-1,0,1,9]));
        machine.set_input_stream(list_to_stream([3]));
        machine.run_to_terminal();
        assert_arr_eq(machine.output_buffer, [ 1 ]);
    }
    {
        let machine = new IntCodeMachine(new Memory([3,3,1105,-1,9,1101,0,0,12,4,12,99,1 ]));
        machine.set_input_stream(list_to_stream([-1]));
        machine.run_to_terminal();
        assert_arr_eq(machine.output_buffer, [ 1 ]);
    }
    {
        let machine = new IntCodeMachine(new Memory([3,3,1105,-1,9,1101,0,0,12,4,12,99,1 ]));
        machine.set_input_stream(list_to_stream([0]));
        machine.run_to_terminal();
        assert_arr_eq(machine.output_buffer, [ 0 ]);
    }
    {
        let machine = new IntCodeMachine(new Memory([3,21,1008,21,8,20,1005,20,22,107,8,21,20,1006,20,31,1106,0,36,98,0,0,1002,21,125,20,4,20,1105,1,46,104,999,1105,1,46,1101,1000,1,20,4,20,1105,1,46,98,99 ]));
        machine.set_input_stream(list_to_stream([7]));
        machine.run_to_terminal();
        assert_arr_eq(machine.output_buffer, [ 999 ]);
    }
    {
        let machine = new IntCodeMachine(new Memory([3,21,1008,21,8,20,1005,20,22,107,8,21,20,1006,20,31,1106,0,36,98,0,0,1002,21,125,20,4,20,1105,1,46,104,999,1105,1,46,1101,1000,1,20,4,20,1105,1,46,98,99 ]));
        machine.set_input_stream(list_to_stream([8]));
        machine.run_to_terminal();
        assert_arr_eq(machine.output_buffer, [ 1000 ]);
    }
    {
        let machine = new IntCodeMachine(new Memory([3,21,1008,21,8,20,1005,20,22,107,8,21,20,1006,20,31,1106,0,36,98,0,0,1002,21,125,20,4,20,1105,1,46,104,999,1105,1,46,1101,1000,1,20,4,20,1105,1,46,98,99 ]));
        machine.set_input_stream(list_to_stream([42]));
        machine.run_to_terminal();
        assert_arr_eq(machine.output_buffer, [ 1001 ]);
    }
}

function generate_perm(arr, acc, cb) {
    if (arr.length === 0) {
        return cb(acc);
    }

    for (let i = 0; i < arr.length; ++i) {
        let temparr = arr.slice(0);
        let extract = arr.splice(i, 1);
        acc.push(extract[0]);
        generate_perm(arr, acc, cb);
        acc.pop();
        arr = temparr;
    }
}
function day_7_1(input) {
    let max = null;
    generate_perm([0,1,2,3,4], [], (phase) => {
        let machines = [];
        let inputs = [];

        for (let i = 0; i < phase.length; ++i) {
            let machine = new IntCodeMachine(new Memory(input));
            machines.push(machine);
        }

        for (let i = 1; i < machines.length; ++i) {
            machines[i].set_input_stream(
                cons(phase[i], machines[i - 1].get_output_stream()));
        }
        machines[0].set_input_stream(list_to_stream([phase[0], 0]));
        machines[machines.length - 1].run_to_terminal();

        let output = machines[machines.length - 1].output_buffer[0];
        if (max === null || output > max) {
            max = output;
        }
    });
    return max;
}
function day_7_2(input) {
    let max = null;
    generate_perm([5,6,7,8,9], [], (phase) => {
        let machines = [];
        let inputs = [];

        for (let i = 0; i < phase.length; ++i) {
            let machine = new IntCodeMachine(new Memory(input));
            machines.push(machine);
        }

        for (let i = 1; i < machines.length; ++i) {
            machines[i].set_input_stream(
                cons(phase[i], machines[i - 1].get_output_stream()));
        }

        machines[0].set_input_stream(
            cons(phase[0],
                 cons(0, machines[machines.length - 1].get_output_stream())));

        machines[machines.length - 1].run_to_terminal();

        let output = machines[machines.length - 1].output_buffer[0];
        if (max === null || output > max) {
            max = output;
        }
    });
    return max;
}

function test_day7_1() {
    assert_eq(day_7_1([3,15,3,16,1002,16,10,16,1,16,15,15,4,15,99,0,0]), 43210);
    assert_eq(day_7_1([3,23,3,24,1002,24,10,24,1002,23,-1,23,101,5,23,23,1,24,23,23,4,23,99,0,0]), 54321);
    assert_eq(day_7_1([3,31,3,32,1002,32,10,32,1001,31,-2,31,1007,31,0,33,1002,33,7,33,1,33,31,31,1,32,31,31,4,31,99,0,0,0]), 65210);
}
function test_day7_2() {
    assert_eq(day_7_2([3,26,1001,26,-4,26,3,27,1002,27,2,27,1,27,26,27,4,27,1001,28,-1,28,1005,28,6,99,0,0,5]), 139629729);
    assert_eq(day_7_2([3,52,1001,52,-5,52,3,53,1,52,56,54,1007,54,5,55,1005,55,26,1001,54,-5,54,1105,1,12,1,53,54,53,1008,54,0,55,1001,55,1,55,2,53,55,53,4,53,1001,56,-1,56,1005,56,6,99,0,0,0,0,10]), 18216);
}

test_basic();
test_io();
test_jump();
test_day7_1();
test_day7_2();
