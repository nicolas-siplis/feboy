use crate::memory_map::MemoryMap;
use crate::register::RegisterId::{A, B, C, D, E, H, L};
use crate::register::WordRegister::StackPointer;
use std::ops::{Index, IndexMut};
use WordRegister::{AccFlag, Double, ProgramCounter};

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum RegisterId {
    A,
    B,
    C,
    D,
    E,
    H,
    L,
}

pub struct Register {
    registers: [ByteRegister; 7],
    pub flags: FlagRegister,
    pub sp: WordRegister,
    pub pc: WordRegister,
}

impl Register {
    pub fn new() -> Self {
        Self {
            registers: [
                ByteRegister { value: 0x01, id: A },
                ByteRegister { value: 0x00, id: B },
                ByteRegister { value: 0x13, id: C },
                ByteRegister { value: 0x00, id: D },
                ByteRegister { value: 0xD8, id: E },
                ByteRegister { value: 0x01, id: H },
                ByteRegister { value: 0x4D, id: L },
            ],
            pc: ProgramCounter(0x0100),
            sp: StackPointer(0xFFFE),
            flags: FlagRegister {
                z: true,
                n: false,
                h: true,
                c: true,
            },
        }
    }

    pub fn af(&self) -> WordRegister {
        AccFlag(self[A], self.flags)
    }
    pub fn bc(&self) -> WordRegister {
        Double(self[B], self[C])
    }
    pub fn de(&self) -> WordRegister {
        Double(self[D], self[E])
    }
    pub fn hl(&self) -> WordRegister {
        Double(self[H], self[L])
    }

    pub fn set_word_register(&mut self, value: u16, reg: WordRegister, mem: &mut MemoryMap) {
        self.set_word_register_with_callback(value, reg, |_mem| (), mem);
    }

    pub fn set_word_register_with_callback(
        &mut self,
        value: u16,
        reg: WordRegister,
        callback: fn(&mut MemoryMap),
        mem: &mut MemoryMap,
    ) {
        let [lo, hi] = value.to_le_bytes();
        match reg {
            AccFlag(..) => {
                self.set_flag(lo);
                callback(mem);
                self[A].value = hi;
            }
            Double(a, b) => {
                self[b.id].value = lo;
                callback(mem);
                self[a.id].value = hi;
            }
            StackPointer(_) => {
                self.sp = StackPointer(value);
                callback(mem);
            }
            ProgramCounter(_) => {
                self.pc = StackPointer(value);
                callback(mem);
            }
        };
    }

    pub fn cc_flag(&mut self, cc: ConditionCode) -> bool {
        match cc {
            ConditionCode::Z => self.flags.z,
            ConditionCode::NZ => !self.flags.z,
            ConditionCode::C => self.flags.c,
            ConditionCode::NC => !self.flags.c,
        }
    }

    pub fn set_flags(&mut self, z: bool, n: bool, h: bool, c: bool) {
        self.flags.z = z;
        self.flags.n = n;
        self.flags.c = c;
        self.flags.h = h;
    }

    pub fn set_flag(&mut self, flag: u8) {
        let flags = flag & 0xF0;
        self.flags.z = flags & 0x80 == 0x80;
        self.flags.n = flags & 0x40 == 0x40;
        self.flags.c = flags & 0x20 == 0x20;
        self.flags.h = flags & 0x10 == 0x10;
    }
}

impl Index<RegisterId> for Register {
    type Output = ByteRegister;
    fn index(&self, index: RegisterId) -> &Self::Output {
        &self.registers[index as usize]
    }
}

impl IndexMut<RegisterId> for Register {
    fn index_mut(&mut self, index: RegisterId) -> &mut Self::Output {
        &mut self.registers[index as usize]
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ByteRegister {
    pub value: u8,
    pub id: RegisterId,
}

impl Into<u8> for ByteRegister {
    fn into(self) -> u8 {
        self.value
    }
}

impl Into<usize> for ByteRegister {
    fn into(self) -> usize {
        self.value as usize + 0xFF00
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct FlagRegister {
    pub z: bool,
    pub n: bool,
    pub h: bool,
    pub c: bool,
}

impl FlagRegister {
    pub fn value(&self) -> u8 {
        [self.c, self.h, self.n, self.z]
            .iter()
            .map(|f| if *f { 1 } else { 0 })
            .enumerate()
            .map(|(i, n)| (n << (i + 4)) as u8)
            .sum()
    }

    pub fn set(&mut self, v: u8) {
        self.z = 0x80 & v != 0;
        self.n = 0x40 & v != 0;
        self.h = 0x20 & v != 0;
        self.c = 0x10 & v != 0;
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum WordRegister {
    Double(ByteRegister, ByteRegister),
    AccFlag(ByteRegister, FlagRegister),
    StackPointer(u16),
    ProgramCounter(u16),
}

impl Into<usize> for WordRegister {
    fn into(self) -> usize {
        self.value() as usize
    }
}

impl WordRegister {
    pub fn value(self) -> u16 {
        match self {
            Double(h, l) => u16::from_le_bytes([l.value, h.value]),
            AccFlag(a, FlagRegister { z, n, h, c }) => {
                let bit_flag = |b: bool, v: u32| 2u8.pow(v) as u8 * if b { 1 } else { 0 };
                u16::from_le_bytes([
                    bit_flag(z, 3) + bit_flag(n, 2) + bit_flag(h, 1) + bit_flag(c, 0),
                    a.value,
                ])
            }
            StackPointer(n) | ProgramCounter(n) => n,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Bit(pub u8);

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ConditionCode {
    Z,
    NZ,
    C,
    NC,
}
