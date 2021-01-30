extern crate libc;

use std::io::{BufRead, BufReader};
use std::mem;

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
    pub fn emit_byte(&mut self, b: u8) {
        self.content.push(b);
    }

    pub fn emit_bytes(&mut self, bs: &[u8]) {
        self.content.append(&mut bs.to_vec());
    }

    pub fn emit_u16(&mut self, n: u16) {
        self.emit_bytes(&[n as u8 & 0xFF, (n >> 8) as u8 & 0xFF])
    }

    pub fn emit_u32(&mut self, n: u32) {
        self.emit_u16((n & 0xFFFF as u32) as u16);
        self.emit_u16((n >> 16 & 0xFFFF as u32) as u16);
    }

    pub fn emit_u64(&mut self, n: u64) {
        self.emit_u32((n & 0xFFFFFFFF as u64) as u32);
        self.emit_u32((n >> 32 & 0xFFFFFFFF as u64) as u32);
    }

    pub fn replace_u32(&mut self, start: usize, end: usize, n: u32) {
        self.content.splice(
            start..end,
            vec![
                (n & 0xFFFFFFFF) as u8,
                (n >> 8 & 0xFFFFFFFF) as u8,
                (n >> 16 & 0xFFFFFFFF) as u8,
                (n >> 24 & 0xFFFFFFFF) as u8,
            ],
        );
    }

    pub fn get(&self) -> &Vec<u8> {
        &self.content
    }

    pub fn len(&self) -> usize {
        self.content.len()
    }
}

fn compile(ops: &[BfOp]) -> MachineCode {
    let mut code = MachineCode::new();
    let memory: *mut u8 = unsafe { mem::transmute(libc::malloc(500000)) };
    code.emit_bytes(&[0x49, 0xBD]);
    code.emit_u64(memory as u64);

    let mut bracket_stack: Vec<usize> = vec![];
    for op in ops {
        match op.kind {
            BfOpKind::IncPtr => {
                code.emit_bytes(&[0x49, 0x81, 0xc5]);
                code.emit_u32(op.argument as u32);
            }
            BfOpKind::DecPtr => {
                code.emit_bytes(&[0x49, 0x81, 0xed]);
                code.emit_u32(op.argument as u32);
            }
            BfOpKind::IncData => {
                if op.argument < 256 {
                    code.emit_bytes(&[0x41, 0x80, 0x45, 0x00, op.argument as u8])
                } else if op.argument < 65536 {
                    code.emit_bytes(&[0x66, 0x41, 0x81, 0x45, 0x00]);
                    code.emit_u16(op.argument as u16);
                }
            }
            BfOpKind::DecData => {
                if op.argument < 256 {
                    code.emit_bytes(&[0x41, 0x80, 0x6d, 0x00, op.argument as u8])
                } else if op.argument < 65536 {
                    code.emit_bytes(&[0x66, 0x41, 0x81, 0x6d, 0x00]);
                    code.emit_u16(op.argument as u16);
                }
            }
            BfOpKind::WriteStdout => {
                code.emit_bytes(&[
                    0x48, 0xC7, 0xC0, 0x01, 0x00, 0x00, 0x00, //
                    0x48, 0xC7, 0xC7, 0x01, 0x00, 0x00, 0x00, //
                    0x4C, 0x89, 0xEE, //
                    0x48, 0xC7, 0xC2, 0x01, 0x00, 0x00, 0x00, //
                    0x0F, 0x05,
                ])
            }
            BfOpKind::ReadStdin => {
                code.emit_bytes(&[
                    0x48, 0xC7, 0xC0, 0x00, 0x00, 0x00, 0x00, //
                    0x48, 0xC7, 0xC7, 0x00, 0x00, 0x00, 0x00, //
                    0x4C, 0x89, 0xEE, //
                    0x48, 0xC7, 0xC2, 0x01, 0x00, 0x00, 0x00, //
                    0x0F, 0x05, //
                ])
            }
            BfOpKind::LoopSetToZero => code.emit_bytes(&[0x41, 0xC6, 0x45, 0x00, 0x00]),
            BfOpKind::LoopMovePtr => {
                code.emit_bytes(&[
                    0x41, 0x80, 0x7d, 0x00, 0x00, //
                    0x0F, 0x84, //
                ]);
                code.emit_u32(0x12);
                if op.argument >= 0 {
                    code.emit_bytes(&[0x49, 0x81, 0xc5]);
                    code.emit_u32(op.argument as u32);
                } else {
                    code.emit_bytes(&[0x49, 0x81, 0xed]);
                    code.emit_u32(-op.argument as u32);
                }
                code.emit_bytes(&[
                    0x41, 0x80, 0x7d, 0x00, 0x00, //
                    0x0f, 0x85, //
                ]);
                code.emit_u32(0xffffffee);
            }
            BfOpKind::LoopMoveData => {
                // skip if data is zero
                code.emit_bytes(&[
                    0x41, 0x80, 0x7d, 0x00, 0x00, //
                    0x0F, 0x84, //
                ]);
                code.emit_u32(23);

                code.emit_bytes(&[0x4d, 0x89, 0xee]);
                if op.argument >= 0 {
                    code.emit_bytes(&[0x49, 0x81, 0xc6]);
                    code.emit_u32(op.argument as u32);
                } else {
                    code.emit_bytes(&[0x49, 0x81, 0xee]);
                    code.emit_u32(-op.argument as u32);
                }
                code.emit_bytes(&[
                    0x49, 0x0f, 0xb6, 0x45, 0x0, //
                    0x41, 0x00, 0x06, //
                    0x41, 0xC6, 0x45, 0x00, 0x00, //
                ]);
            }
            BfOpKind::JumpIfDataZero => {
                code.emit_bytes(&[0x41, 0x80, 0x7d, 0x00, 0x00]);
                bracket_stack.push(code.len());
                code.emit_bytes(&[0x0F, 0x84]);
                code.emit_u32(0);
            }
            BfOpKind::JumpIfDataNotZero => {
                let bracket_offset = bracket_stack.pop().expect("mismatch [");
                code.emit_bytes(&[0x41, 0x80, 0x7d, 0x00, 0x00]);
                let jump_back_from = (code.len() + 6) as i32;
                let jump_back_to = bracket_offset as i32 + 6;
                let offset_back = (jump_back_to - jump_back_from) as u32;
                code.emit_bytes(&[0x0F, 0x85]);
                code.emit_u32(offset_back);
                let offset_back = code.len() as i32 - jump_back_to;
                code.replace_u32(bracket_offset + 2, bracket_offset + 6, offset_back as u32)
            }
        }
    }
    code.emit_byte(0xc3);
    code
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
    let ops = translate(&insts);
    let code = compile(&ops);
    execute(code.get());
}
