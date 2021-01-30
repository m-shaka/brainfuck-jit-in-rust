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

enum BfOpKind {
    IncPtr,
    DecPtr,
    IncData,
    DecData,
    ReadStdin,
    WriteStdout,
    LoopSetToZero,
    LoopMovePtr,
    LopMoveData,
    JumpIfDataZero,
    JumpIfDataNotZero,
}

struct BfOp {
    kind: BfOpKind,
    argument: usize,
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
    for c in parse(&args[1]) {
        println!("{}", c);
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
