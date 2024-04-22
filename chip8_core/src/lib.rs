use rand::random;

const RAM_SIZE: usize = 4096;

// screen 64 x 32
pub const SCREEN_WIDTH: usize = 64;
pub const SCREEN_HEIGHT: usize = 32;

// V registers
const NUM_REGS: usize = 16;

// Stack
const STACK_SIZE: usize = 16;

// Keyboard input
const NUM_KEYS: usize = 16;

// CHIP-8 program start
const START_ADDR: u16 = 0x200;

// Fonts
const FONTSET_SIZE: usize = 80; //5 * (6 + 10)

const FONTSET: [u8; FONTSET_SIZE] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80 // F
];

pub struct Emulator {
    pc: u16,
    ram: [u8; RAM_SIZE], //4KB RAM
    screen: [bool; SCREEN_WIDTH * SCREEN_HEIGHT],
    v_regs: [u8; NUM_REGS],
    i_reg: u16,
    sp: u16, // stack pointer $sp
    stack: [u16; STACK_SIZE],
    d_timer: u8,
    s_timer: u8,
    keys: [bool; NUM_KEYS]
}

impl Emulator {
    pub fn new() -> Self {
        let mut emu = Self {
            pc: START_ADDR,
            ram: [0; RAM_SIZE],
            screen: [false; SCREEN_WIDTH * SCREEN_HEIGHT],
            v_regs: [0; NUM_REGS],
            i_reg: 0,
            sp: 0,
            stack: [0; STACK_SIZE],
            d_timer: 0,
            s_timer: 0,
            keys: [false; NUM_KEYS]
        };
        emu.ram[..FONTSET_SIZE].copy_from_slice(&FONTSET); //copy 0 -> set size
        emu
    }

    pub fn reset(&mut self) {
        self.pc = START_ADDR;
        self.ram = [0; RAM_SIZE];
        self.screen = [false; SCREEN_WIDTH * SCREEN_HEIGHT];
        self.v_regs = [0; NUM_REGS];
        self.i_reg = 0;
        self.sp = 0;
        self.stack = [0; STACK_SIZE];
        self.d_timer = 0;
        self.s_timer = 0;
        self.keys = [false; NUM_KEYS];
        self.ram[..FONTSET_SIZE].copy_from_slice(&FONTSET);
    }

    // Stack Impl
    pub fn push(&mut self, value: u16) {
        self.stack[self.sp as usize] = value; //stack at sp becomes val
        self.sp += 1; // increment sp
    }

    pub fn pop(&mut self) -> u16 {
        self.sp -= 1; // decrement sp
        self.stack[self.sp as usize]
    }

    // CPU Cycle
    pub fn tick(&mut self) {
        // Fetch instruction
        let inst = self.fetch();
        // Decode --> execute
        self.execute(inst);
    }
    
    /// Exposes display, returns a reference to the display
    pub fn get_display(&self) -> &[bool] {
        &self.screen
    }

    /// updates keys array with pressed value. idx < 16 or else panic.
    pub fn keypress(&mut self, idx: usize, pressed: bool) {
        self.keys[idx] = pressed;
    }

    /// Load game data into RAM.
    pub fn load(&mut self, data: &[u8]) {
        let start = START_ADDR as usize;
        let end = (START_ADDR as usize) + data.len();
        self.ram[start..end].copy_from_slice(data);
    }

    fn fetch(&mut self) -> u16 {
        // instructions are big endian, so upper bits at lower PC
        let upper = self.ram[self.pc as usize] as u16;
        let lower = self.ram[(self.pc + 1) as usize] as u16;
        let inst = (upper << 8) | lower;
        self.pc += 2;
        inst
    }

    /// Decodes and exectures instructions by CHIP-8 specification
    fn execute(&mut self, inst: u16) {
        let D3 = (inst & 0xF000) >> 12;
        let D2 = (inst & 0x0F00) >> 8;
        let D1 = (inst & 0x00F0) >> 4;
        let D0 = inst & 0x000F;

        match (D3, D2, D1, D0) {
            // NOP
            (0,0,0,0) => return,
            // CLS (Clear Screen)
            (0,0,0xE,0) => {
                self.screen = [false; SCREEN_WIDTH * SCREEN_HEIGHT];
            },
            // RET (Return from subroutine, akin to jr in MIPS)
            (0, 0, 0xE, 0xE) => {
                // ra stored on stack on subroutine call
                self.pc = self.pop();
            },
            // Jump to 0xNNN where NNN is jump address
            (1, _, _, _) => {
                self.pc = inst & 0xFFF;
            },
            // Jump to subroutine
            (2, _, _, _) => {
                self.push(self.pc); //store ra
                self.pc = inst & 0xFFF;
            },
            // Skip if VX == 0xNN
            (3, _, _, _) => {
                if self.v_regs[D2 as usize] == (inst & 0xFF) as u8 {
                    self.pc += 2; //skip ahead 2 bytes
                }
            },
            // Skip if VX != 0xNN
            (4, _, _, _) => {
                if self.v_regs[D2 as usize] != (inst & 0xFF) as u8 {
                    self.pc += 2;
                }
            },
            // Skip if VX == VY
            (5, _, _, 0) => {
                if self.v_regs[D2 as usize] == self.v_regs[D1 as usize] {
                    self.pc += 2;
                }
            },
            // VX = 0xNN
            (6, _, _, _) => {
                self.v_regs[D2 as usize] = (0xFF & inst) as u8;
            },
            // VX += 0xNN
            (7, _, _, _) => {
                let x = D2 as usize;
                let (new_vx, carry) = self.v_regs[x].overflowing_add((0xFF & inst) as u8);
                let new_vf = if carry { 1 } else { 0 };
                self.v_regs[x] = new_vx;
                self.v_regs[0xF] = new_vf; //set VF reg
            },
            // VX = VY
            (8, _, _, 0) => {
                self.v_regs[D2 as usize] = self.v_regs[D1 as usize];
            },  
            // VX |= VY (OR)
            (8, _, _, 1) => {
                self.v_regs[D2 as usize] |= self.v_regs[D1 as usize];
            },
            // VX &= VY (AND)
            (8, _, _, 2) => {
                self.v_regs[D2 as usize] &= self.v_regs[D1 as usize];
            },        
            // VX ^= VY (XOR)
            (8, _, _, 3) => {
                self.v_regs[D2 as usize] ^= self.v_regs[D1 as usize];
            },
            // VX += VY, account for overflow
            (8, _, _, 4) => {
                let x = D2 as usize;
                let y = D1 as usize;
                let (new_vx, carry) = self.v_regs[x].overflowing_add(self.v_regs[y]);
                let new_vf = if carry { 1 } else { 0 };
                self.v_regs[x] = new_vx;
                self.v_regs[0xF] = new_vf; //set VF reg
            },     
            // VX -= VY, account for overflow
            (8, _, _, 5) => {
                let x = D2 as usize;
                let y = D1 as usize;
                let (new_vx, borrow) = self.v_regs[x].overflowing_sub(self.v_regs[y]);
                let new_vf = if borrow { 0 } else { 1 };
                self.v_regs[x] = new_vx;
                self.v_regs[0xF] = new_vf;
            },    
            // VX >>= 1
            (8, _, _, 6) => {
                self.v_regs[0xF] = self.v_regs[D2 as usize] & 1;
                self.v_regs[D2 as usize] >>= 1;
            },        
            // VX = VY - VX
            (8, _, _, 7) => {
                let x = D2 as usize;
                let y = D1 as usize;

                let (new_vx, borrow) = self.v_regs[y].overflowing_sub(self.v_regs[x]);
                let new_vf = if borrow {0} else {1};

                self.v_regs[x] = new_vx;
                self.v_regs[0xF] = new_vf;
            },
            // VX <<= 1
            (8, _, _, 0xE) => {
                self.v_regs[0xF] = self.v_regs[D2 as usize] & 8;
                self.v_regs[D2 as usize] <<= 1;
            },
            // Skip if VX != VY
            (9, _, _, 0) => {
                if self.v_regs[D2 as usize] != self.v_regs[D1 as usize] {
                    self.pc += 2;
                }
            },
            // I = 0xNNN
            (0xA, _, _, _) => {
                self.i_reg = inst & 0xFFF;
            },
            // Jump to V0 + NNN
            (0xB, _, _, _) => {
                self.pc = ((self.v_regs[0x0] as u16) + (inst & 0xFFF)) as u16;
            },
            // VX = rand() & 0xNN
            (0xC, _, _, _) => {
                self.v_regs[D2 as usize] = ((inst & 0xFF) as u8) & random::<u8>();
            },
            // Draw sprite at (VX, VY)
            (0xD, _, _, _) => {
                let x = self.v_regs[D2 as usize] as u16;
                let y = self.v_regs[D1 as usize] as u16;

                // number of rows is variable in sprite, denoted by last N in inst
                let num_rows = D0;

                // pixels flipped flag
                let mut flipped = false;

                // iterate over each row of sprite
                for v_line in 0..num_rows {
                    let addr = self.i_reg + v_line as u16;
                    let pixels = self.ram[addr as usize];

                    // iterate over each column in sprite, each always 1 byte long
                    for h_line in 0..8 {
                        // find current pixel, only flip if 1.
                        if (pixels & (0x80 >> h_line)) != 0 {
                            // find x and y of pixel relative to screen size
                            let curr_x = (x + h_line) as usize % SCREEN_WIDTH;
                            let curr_y = (y + v_line) as usize % SCREEN_HEIGHT;

                            // map to 1D display array
                            let idx = curr_x + SCREEN_WIDTH * curr_y;

                            // if pixel at index is true, flip.
                            flipped |= self.screen[idx];
                            self.screen[idx] ^= true;
                        }
                    }
                }
                // update VF register
                if flipped {
                    self.v_regs[0xF] = 1;
                } else {
                    self.v_regs[0xF] = 0;
                }
            },
            // Skip if key index in VX is pressed
            (0xE, _, 9, 0xE) => {
                let vx = self.v_regs[D2 as usize];
                let key = self.keys[vx as usize];
                if key {
                    self.pc += 2;
                }
            },
            // Skip if Key Not Pressed
            (0xE, _, 0xA, 1) => {
                let vx = self.v_regs[D2 as usize];
                let key = self.keys[vx as usize];
                if !key {
                    self.pc += 2;
                }
            },
            // VX = Delay Timer
            (0xF, _, 0, 7) => {
                self.v_regs[D2 as usize] = self.d_timer;
            },
            // Waits for key press, stores index in VX. Blocking.
            (0xF, _, 0, 0xA) => {
                let x = D2 as usize;
                let mut pressed = false;
                for i in 0..self.keys.len() {
                    if self.keys[i] {
                        self.v_regs[x] = i as u8;
                        pressed = true;
                        break;
                    }
                }
                // if key not pressed, redo inst
                if !pressed {
                    self.pc -= 2;
                }
            },
            // Delay Timer = VX
            (0xF, _, 1, 5) => {
                self.d_timer = self.v_regs[D2 as usize];
            },
            // Sound Timer = VX
            (0xF, _, 1, 8) => {
                self.s_timer = self.v_regs[D2 as usize];
            },
            // I += VX, on case of overflow wrap around to 0
            (0xF, _, 1, 0xE) => {
                self.i_reg = self.i_reg.wrapping_add(self.v_regs[D2 as usize] as u16);
            },
            // Set I to address of font character in VX
            (0xF, _, 2, 9) => {
                // Ex. 0 => 0, A => A, each row is 5 bytes so mult by 5
                self.i_reg = (self.v_regs[D2 as usize] as u16) * 5;
            },
            // Stores BCD encoding of VX into RAM at index stored in I. Hex => Decimal
            (0xF, _, 3, 3) => {
                // this could be optimized with some binary magic
                let vx = self.v_regs[D2 as usize] as f32;
                let hundreds = (vx / 100.0).floor() as u8;
                let tens = ((vx / 10.0) % 10.0).floor() as u8;
                let ones = (vx % 10.0) as u8;
                // big endian storing
                self.ram[self.i_reg as usize] = hundreds;
                self.ram[(self.i_reg + 1) as usize] = tens;
                self.ram[(self.i_reg + 2) as usize] = ones;
            },
            // Stores V0 through VX into RAM address starting at I, inclusive
            (0xF, _, 5, 5) => {
                let vx = D2 as usize;
                let i = self.i_reg as usize;
                for val in 0..=vx {
                    self.ram[i + val] = self.v_regs[val];
                }
            },
            // Fills V0 through VX with RAM values starting at address in I inclusive.
            (0xF, _, 6, 5) => {
                let vx = D2 as usize;
                let i = self.i_reg as usize;
                for val in 0..=vx {
                    self.v_regs[val] = self.ram[i + val];
                } 
            },
            (_, _, _, _) => {
                return;
            }
        }
    }

    //timers
    pub fn tick_timers(&mut self) {
        if self.d_timer > 0 {
            self.d_timer -= 1;
        }
        if self.s_timer > 0 {
            if self.s_timer == 1 {
                // Beep here
            }
            self.s_timer -= 1;
        }
    }
}