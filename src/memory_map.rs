use crate::interrupt::{InterruptHandler};
use crate::interrupt::InterruptId::{JoypadInt, StatInt, TimerInt, VBlankInt};
use crate::ppu::{PPU, PpuMode, DmaState};
use std::any::{Any, TypeId};
use crate::ppu::RenderCycle::{StatTrigger, Normal};
use crate::ppu::PpuState::ModeChange;
use PpuMode::VBlank;
use crate::timer::{Timer};
use crate::joypad::{Joypad};
use DmaState::{Inactive, Starting};
use OamCorruptionCause::IncDec;
use crate::memory_map::OamCorruptionCause::{Read, ReadWrite, Write};
use crate::ppu::PpuMode::OamSearch;
use crate::register::WordRegister;

#[derive(Debug)]
pub enum OamCorruptionCause {
    IncDec,
    Read,
    Write,
    ReadWrite
}

pub struct MemoryMap {
    pub memory: Vec<u8>,
    pub interrupt_handler: InterruptHandler,
    pub ppu: PPU,
    timer: Timer,
    joypad: Joypad,
    rom_size: usize,
    rom_name: String,
    pub cycles: u16,
    dma_progress: usize
}

impl MemoryMap {
    pub fn new(rom: &Vec<u8>, rom_name: &String) -> MemoryMap {
        let ppu = PPU::new(rom_name);
        let joypad = Joypad::new();
        let interrupt_handler = InterruptHandler::new();
        let timer = Timer::new();
        let rom_size = rom.len() as usize;
        let rom_name = rom_name.to_owned();
        let memory = vec![0; 0x10000];
        let cycles = 0;
        let dma_progress = 0;
        let mem = MemoryMap { joypad, ppu, interrupt_handler, timer, memory, rom_name, rom_size, cycles, dma_progress };
        MemoryMap::init_memory(mem, rom)
    }

    pub fn read<T: 'static + Into<usize> + Copy>(&mut self, address: T) -> u8 {
        let value = self.read_without_cycle(address);
        self.cycle();
        return value;
    }

    pub fn write<Address: 'static + Into<usize> + Copy, Value: Into<u8> + Copy>(&mut self, address: Address, value: Value) {
        self.write_without_cycle(address, value.into());
        self.cycle();
    }

    pub fn read_without_cycle<T: 'static + Into<usize> + Copy>(&mut self, address: T) -> u8 {
        //println!("Reading address {} with value {}", address.into(), self.memory(address.into()));
        let translated_address = if address.type_id() == TypeId::of::<u8>() { address.into() + 0xFF00 } else { address.into() };
        let read = self.ppu.read(translated_address)
            .or(self.interrupt_handler.read(translated_address))
            .or(self.timer.read(translated_address))
            .or(self.joypad.read(translated_address))
            .unwrap_or(self.memory[translated_address]);
        read
    }

    fn write_without_cycle<T: 'static + Into<usize> + Copy>(&mut self, address: T, value: u8) {
        //println!("Writing address {}", address.into());
        let translated_address = if address.type_id() == TypeId::of::<u8>() { address.into() + 0xFF00 } else { address.into() };
        if !(self.ppu.write(translated_address, value)
            || self.timer.write(translated_address, value)
            || self.interrupt_handler.write(translated_address, value)
            || self.joypad.write(translated_address, value)) && (translated_address >= self.rom_size) {
            self.memory[translated_address] = value
        }
    }

    pub fn cycle(&mut self) {
        self.cycles += 1;
        self.dma_transfer();
        self.machine_cycle();
    }

    fn dma_transfer(&mut self) {
        // TODO: Can DMA transfer trigger OAM corruption?
        if let Inactive | Starting = self.ppu.dma { return; }
        while self.dma_progress < self.ppu.dma_progress {
            println!("DMA_READ + ");
            self.ppu.oam[self.dma_progress] = self.read_without_cycle(self.ppu.dma_offset * 0x100 + self.dma_progress);
            self.dma_progress += 1;
        }
        if self.dma_progress == self.ppu.oam.len() {
            self.dma_progress = 0;
        }
    }

    fn machine_cycle(&mut self) {
        let mut interrupts = vec![];
        interrupts.append(&mut match self.ppu.machine_cycle() {
            StatTrigger(ModeChange(_, VBlank)) => vec![VBlankInt, StatInt],
            Normal(ModeChange(_, VBlank)) => vec![VBlankInt],
            StatTrigger(_) => vec![StatInt],
            _ => vec![]
        });
        interrupts.append(&mut match self.timer.machine_cycle() {
            Some(_) => vec![TimerInt],
            None => vec![],
        });

        interrupts.append(&mut self.joypad.machine_cycle(&self.ppu.window).iter().map(|_| JoypadInt).collect());

        self.interrupt_handler.set(interrupts, true);
    }

    fn init_memory(mut mem: MemoryMap, rom: &Vec<u8>) -> MemoryMap {
        for (index, value) in rom.iter().enumerate() { mem.memory[index] = *value }
        mem.write_without_cycle(0xFF05_u16, 0);
        mem.write_without_cycle(0xFF06_u16, 0);
        mem.write_without_cycle(0xFF07_u16, 0);
        mem.write_without_cycle(0xFF10_u16, 0x80);
        mem.write_without_cycle(0xFF11_u16, 0xBF);
        mem.write_without_cycle(0xFF12_u16, 0xF3);
        mem.write_without_cycle(0xFF14_u16, 0xBF);
        mem.write_without_cycle(0xFF16_u16, 0x3F);
        mem.write_without_cycle(0xFF16_u16, 0x3F);
        mem.write_without_cycle(0xFF17_u16, 0);
        mem.write_without_cycle(0xFF19_u16, 0xBF);
        mem.write_without_cycle(0xFF1A_u16, 0x7F);
        mem.write_without_cycle(0xFF1B_u16, 0xFF);
        mem.write_without_cycle(0xFF1C_u16, 0x9F);
        mem.write_without_cycle(0xFF1E_u16, 0xFF);
        mem.write_without_cycle(0xFF20_u16, 0xFF);
        mem.write_without_cycle(0xFF21_u16, 0);
        mem.write_without_cycle(0xFF22_u16, 0);
        mem.write_without_cycle(0xFF23_u16, 0xBF);
        mem.write_without_cycle(0xFF24_u16, 0x77);
        mem.write_without_cycle(0xFF25_u16, 0xF3);
        mem.write_without_cycle(0xFF26_u16, 0xF1);
        mem.write_without_cycle(0xFF40_u16, 0x91);
        mem.write_without_cycle(0xFF42_u16, 0);
        mem.write_without_cycle(0xFF43_u16, 0);
        mem.write_without_cycle(0xFF45_u16, 0);
        mem.write_without_cycle(0xFF47_u16, 0xFC);
        mem.write_without_cycle(0xFF48_u16, 0xFF);
        mem.write_without_cycle(0xFF49_u16, 0xFF);
        mem.write_without_cycle(0xFF4A_u16, 0);
        mem.write_without_cycle(0xFF4B_u16, 0);
        mem.write_without_cycle(0xFF00_u16, 0xFF);
        mem
    }
}