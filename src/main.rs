extern crate libc;

use std::io::{BufRead, BufReader};
use std::mem;

const MASK: u8 = 0xFF;

const TOKENS: &str = "><+-.,[]";

pub fn parse(filepath: &String) -> Vec<char> {
    let file = std::fs::File::open(filepath).expect(&format!("Cannot read file: {}", filepath));
    let reader = BufReader::new(file);
    let mut res: Vec<char> = vec![];

    for line in reader.lines() {
        for c in line.unwrap().chars() {
            if TOKENS.contains(c) {
                res.push(c)
            }
        }
    }
    res
}

#[derive(Debug, PartialEq)]
enum BfOpKind {
    IncPtr,
    DecPtr,
    IncData,
    DecData,
    ReadStdin,
    WriteStdout,
    LoopSetToZero,
    LoopMovePtr,
    LoopMoveData,
    JumpIfDataZero,
    JumpIfDataNotZero,
}

#[derive(Debug)]
struct BfOp {
    pub kind: BfOpKind,
    pub argument: i32,
}

fn translate(insts: &[char]) -> Vec<BfOp> {
    let mut res: Vec<BfOp> = vec![];
    let mut loop_stack: Vec<usize> = vec![];
    let mut pc: usize = 0;
    let program_size = insts.len();
    while pc < program_size {
        let inst = insts[pc];
        match inst {
            '[' => {
                loop_stack.push(res.len());
                res.push(BfOp {
                    kind: BfOpKind::JumpIfDataZero,
                    argument: 0,
                });
                pc += 1;
            }
            ']' => {
                let offset = loop_stack
                    .pop()
                    .expect(&format!("unmatched closing ']' at pc={}", pc));
                let optimized_ops = optimize_loop(&res, offset);
                if optimized_ops.len() == 0 {
                    res[offset].argument = res.len() as i32;
                    res.push(BfOp {
                        kind: BfOpKind::JumpIfDataNotZero,
                        argument: offset as i32,
                    })
                } else {
                    res.splice(offset.., optimized_ops);
                }
                pc += 1;
            }
            _ => {
                let num_repeats = insts[pc..insts.len()]
                    .iter()
                    .take_while(|&&c| c == inst)
                    .count();
                pc += num_repeats;
                let kind = match inst {
                    '>' => BfOpKind::IncPtr,
                    '<' => BfOpKind::DecPtr,
                    '+' => BfOpKind::IncData,
                    '-' => BfOpKind::DecData,
                    ',' => BfOpKind::ReadStdin,
                    '.' => BfOpKind::WriteStdout,
                    _ => panic!("Invalid token"),
                };
                res.push(BfOp {
                    kind,
                    argument: num_repeats as i32,
                })
            }
        }
    }
    res
}

fn optimize_loop(ops: &[BfOp], loop_start: usize) -> Vec<BfOp> {
    let mut res: Vec<BfOp> = vec![];
    let loop_size = ops.len() - loop_start;
    match loop_size {
        2 => {
            let repeated_op = &ops[loop_start + 1];
            match repeated_op.kind {
                BfOpKind::IncData | BfOpKind::DecData => res.push(BfOp {
                    kind: BfOpKind::LoopSetToZero,
                    argument: 0,
                }),
                BfOpKind::IncPtr => res.push(BfOp {
                    kind: BfOpKind::LoopMovePtr,
                    argument: repeated_op.argument,
                }),
                BfOpKind::DecPtr => res.push(BfOp {
                    kind: BfOpKind::LoopMovePtr,
                    argument: -repeated_op.argument,
                }),

                _ => {}
            }
        }
        5 => {
            if ops[loop_start + 1].kind == BfOpKind::DecData
                && ops[loop_start + 3].kind == BfOpKind::IncData
                && ops[loop_start + 1].argument == 1
                && ops[loop_start + 3].argument == 1
            {
                match (&ops[loop_start + 2], &ops[loop_start + 4]) {
                    (
                        BfOp {
                            kind: BfOpKind::IncPtr,
                            argument: a1,
                        },
                        BfOp {
                            kind: BfOpKind::DecPtr,
                            argument: a2,
                        },
                    ) if a1 == a2 => res.push(BfOp {
                        kind: BfOpKind::LoopMoveData,
                        argument: *a1,
                    }),
                    (
                        BfOp {
                            kind: BfOpKind::DecPtr,
                            argument: a1,
                        },
                        BfOp {
                            kind: BfOpKind::IncPtr,
                            argument: a2,
                        },
                    ) if a1 == a2 => res.push(BfOp {
                        kind: BfOpKind::LoopMoveData,
                        argument: -*a1,
                    }),
                    _ => {}
                }
            }
        }
        _ => {}
    }
    res
}

struct MachineCode {
    content: Vec<u8>,
}

impl MachineCode {
    pub fn new() -> Self {
        Self { content: vec![] }
    }

    pub fn emit_bytes(&mut self, bs: &[u8]) {
        self.content.append(&mut bs.to_vec());
    }

    pub fn emit_u16(&mut self, n: u16) {
        self.emit_bytes(&[n as u8 & MASK, (n >> 8) as u8 & MASK])
    }

    pub fn emit_u32(&mut self, n: u32) {
        self.emit_u16((n & MASK as u32) as u16);
        self.emit_u16((n >> 16 & MASK as u32) as u16);
    }

    pub fn emit_u64(&mut self, n: u64) {
        self.emit_u32((n & MASK as u64) as u32);
        self.emit_u32((n >> 32 & MASK as u64) as u32);
    }

    pub fn get(&self) -> &Vec<u8> {
        &self.content
    }
}

fn execute(code: &Vec<u8>) {
    unsafe {
        let page = libc::mmap(
            std::ptr::null_mut(),
            code.len(),
            libc::PROT_EXEC | libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_ANONYMOUS | libc::MAP_PRIVATE,
            -1,
            0,
        );
        let program: *mut u8 = mem::transmute(page);
        program.copy_from_nonoverlapping(code.as_ptr(), code.len());
        let f: fn() -> i64 = mem::transmute(page);
        f();
    }
}
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let insts = parse(&args[1]);
    for bf_op in translate(&insts) {
        println!("{:?}", bf_op);
    }
    return ();
    let mut code = MachineCode::new();
    code.emit_bytes(&[
        0x48, 0xC7, 0xC0, 0x00, 0x00, 0x00, 0x00, 0x48, 0xC7, 0xC7, 0x00, 0x00, 0x00, 0x00, 0x4C,
        0x89, 0xEE, 0x48, 0xC7, 0xC2, 0x01, 0x00, 0x00, 0x00, 0x0F, 0x05, 0x48, 0xC7, 0xC0, 0x01,
        0x00, 0x00, 0x00, 0x48, 0xC7, 0xC7, 0x01, 0x00, 0x00, 0x00, 0x4C, 0x89, 0xEE, 0x48, 0xC7,
        0xC2, 0x01, 0x00, 0x00, 0x00, 0x0F, 0x05, 0xc3,
    ]);
    execute(code.get())
}
