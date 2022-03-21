
// CHIP-8 EMULATOR CORE

/*
Basic CPU loop:
    1. Fetch the value from our game (loaded into RAM) at the memory address stored 
       in our program counter.
    2. Decode this instruction
    3. Execute, which will possible involve modifying our CPU registers or RAM
    4. Move the PC to the next instruction and repeat
*/
use rand::random;

pub const SCREEN_WIDTH: usize = 64;
pub const SCREEN_HEIGHT: usize = 32;

const RAM_SIZE: usize = 4096;
const NUM_REGS: usize = 16;
const STACK_SIZE: usize = 16;
const NUM_KEYS: usize = 16;
const START_ADDR: u16 = 0x200;
const FONTSET_SIZE: usize = 80;

// Defines characters 0 through 9, A through F
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
    0xF0, 0x80, 0xF0, 0x80, 0x80  // F
];




pub struct Emu {
    pc: u16,
    ram: [u8; RAM_SIZE],
    screen: [bool; SCREEN_WIDTH * SCREEN_HEIGHT],
    v_reg: [u8; NUM_REGS],
    i_reg: u16,
    sp: u16,
    stack: [u16; STACK_SIZE],
    keys: [bool;NUM_KEYS],
    dt: u8,
    st: u8,
}

impl Emu {
    pub fn new() -> Self {
        let mut new_emu = Self {
            pc: START_ADDR,
            ram: [0; RAM_SIZE],
            screen: [false; SCREEN_WIDTH * SCREEN_HEIGHT],
            v_reg: [0; NUM_REGS],
            i_reg: 0,
            sp: 0,
            stack: [0x200; STACK_SIZE],
            keys: [false;NUM_KEYS],
            dt: 0,
            st: 0,
        };
        new_emu.ram[..FONTSET_SIZE].copy_from_slice(&FONTSET);

        new_emu

    }

    pub fn reset(&mut self) {
        // resets the emulator by setting everything back to default values
        self.pc = START_ADDR; // program counter
        self.ram = [0; RAM_SIZE];
        self.screen = [false; SCREEN_WIDTH * SCREEN_HEIGHT];
        self.v_reg = [0; NUM_REGS];
        self.i_reg = 0;
        self.sp = 0; // stack pointer
        self.stack = [0x200; STACK_SIZE];
        self.keys = [false;NUM_KEYS];
        self.dt = 0; // delay timer
        self.st = 0; // sound timer
        self.ram[..FONTSET_SIZE].copy_from_slice(&FONTSET);
    }

    fn push(&mut self, val: u16) {
        self.stack[self.sp as usize] = val;
        self.sp += 1;
    }

    fn pop(&mut self) -> u16 {
        self.sp -= 1;
        self.stack[self.sp as usize]
    }

    pub fn tick(&mut self) {
        
        // Fetch
        let op = self.fetch();
        
        // Decode & execute
        self.execute(op);
    }


    fn fetch(&mut self) -> u16 {
        // get the instruction (opcode) we are about to execute
        let higher_byte = self.ram[self.pc as usize] as u16;
        let lower_byte = self.ram[(self.pc + 1) as usize] as u16;
        let op = (higher_byte << 8) | lower_byte;
        self.pc += 2;
        op
    }


    pub fn tick_timers(&mut self) {
        if self.dt > 0 {
            self.dt -= 1;
        }

        if self.st > 0 {
            if self.st == 1 {
                // BEEP
            }
            self.st -= 1;
        }
    }



    fn execute(&mut self, op: u16) {
        let digit1 = (op & 0xF000) >> 12;
        let digit2 = (op & 0x0F00) >> 8;
        let digit3 = (op & 0x00F0) >> 4;
        let digit4 = op & 0x000F;


        match (digit1, digit2, digit3, digit4) {
            
            // NOP: do nothing
            (0,0,0,0) => return,

            // 00E0 - Clear screen (CLS)
            (0,0,0xE,0) => {
                self.screen = [false; SCREEN_WIDTH * SCREEN_HEIGHT];
            },

            // 00EE - Retrun from subroutine (RET)
            (0,0,0xE,0xE) => {
                let ret_addr = self.pop();
                self.pc = ret_addr;
            },

            // 1NNN - Jump
            // Anything starting with 1, but ending with any three digits 
            // The other 3 digits are used as parameters
            (1,_,_,_) => {
                let nnn = op & 0xFFF;
                self.pc = nnn;
            },

            // 2NNN - Call subroutine
            // Opposite of RET. Add current PC to the stack, then
            // jump to the given address
            // 2, followed by the 3 parameters for where to jump to
            (2,_,_,_) => {
               let nnn = op & 0xFFF;
               self.push(self.pc);
               self.pc = nnn;
            },
            
            // 3XNN - Skip next if VX == NN
            // Assembly equivalent of an if else block a
            (3,_,_,_) => {
                let x = digit2 as usize;
                let nn = (op & 0xFF) as u8;
                if self.v_reg[x] == nn {
                    // skipping the next opcode is the same as skipping
                    // PC ahead by 2 bytes
                    self.pc += 2;
                }
            },

            // 4XNN - Skip next if VX != NN
            (4,_,_,_) => {
                let x = digit2 as usize;
                let nn = (op & 0xFF) as u8;
                if self.v_reg[x] != nn {
                    // skipping the next opcode is the same as skipping
                    // PC ahead by 2 bytes
                    self.pc += 2;
                }
            },

            // 5XY0 - Skip next if VX == VY
            (5,_,_,0) => {
                let x = digit2 as usize;
                let y = digit3 as usize;
                if self.v_reg[x] == self.v_reg[y] {
                    self.pc += 2;
                }
            },

            // 6XNN - VX = NN
            // Sets the VX register to the given value
            (6,_,_,_) => {
                let x = digit2 as usize;
                let nn = (op & 0xFF) as u8;
                self.v_reg[x] = nn;
            },

            // 7XNN - VX += NN
            // Adds given value to the VX reigster
            (7,_,_,_) => {
                let x = digit2 as usize;
                let nn = (op & 0xFF) as u8;
                // use wrapping_add to avoid rust panics at overflows
                self.v_reg[x] = self.v_reg[x].wrapping_add(nn); 
            },

            // 8XY0 - VX = VY
            // Set the VX register to the value of VY
            (8,_,_,0) => {
                let x = digit2 as usize;
                let y = digit3 as usize;
                self.v_reg[x] = self.v_reg[y];
            },
            
            // 8XY1 - Bitwise OR operation (VX |= VY)
            (8,_,_,1) => {
                let x = digit2 as usize;
                let y = digit3 as usize;
                self.v_reg[x] |= self.v_reg[y];
            },
            
            // 8XY2 - Bitwise AND operation (VX &= VY)
            (8,_,_,2) => {
                let x = digit2 as usize;
                let y = digit3 as usize;
                self.v_reg[x] &= self.v_reg[y];
            },
            
            // 8XY3 - Bitwise XOR operation (VX ^= VY)
            (8,_,_,3) => {
                let x = digit2 as usize;
                let y = digit3 as usize;
                self.v_reg[x] ^= self.v_reg[y];
            },

            // 8XY4 - VX += VY
            // Does the operation then sets the Flag register to indicate whether
            // or not an overflow occured (if overflow, flag =1, 0 if not)
            (8,_,_,4) => {
                let x = digit2 as usize;
                let y = digit3 as usize;

                let (new_vx, carry) = self.v_reg[x].overflowing_add(self.v_reg[y]);
                let new_vf = if carry { 1 } else { 0 };

                self.v_reg[x] = new_vx;
                self.v_reg[0xF] = new_vf;
            },

            // 8XY5 - VX -= VY
            // Same as the last one except subtraction instead of addtion.
            (8,_,_,5) => {
                let x = digit2 as usize;
                let y = digit3 as usize;
             
                let(new_vx, borrow) = self.v_reg[x].overflowing_sub(self.v_reg[y]);
                let new_vf = if borrow { 0 } else { 1 };

                self.v_reg[x] = new_vx;
                self.v_reg[0xF] = new_vf;
            },

            // 8XY6 - Single right shift of VX (VX >>= 1)
            // bit that is dropped off is stored in the VF register
            (8,_,_,6) => {
                let x = digit2 as usize;
                let lsb = self.v_reg[x] & 1;
                self.v_reg[x] >>= 1;
                self.v_reg[0xF] = lsb;
            },

            // 8XY7 - VX = VY - VX
            // similar to 8XY5 but in opposite direction
            (8,_,_,7) => {
                let x = digit2 as usize;
                let y = digit3 as usize;
                
                let (new_vx, borrow) = self.v_reg[y].overflowing_sub(self.v_reg[x]);
                let new_vf = if borrow { 0 } else { 1 };

                self.v_reg[x] = new_vx;
                self.v_reg[0xF] = new_vf;
            },

            // 8XYE - Single left shift of VX (VX <<= 1)
            // Store the overflowed value in the flag register
            (8,_,_,0xE) => {
                let x = digit2 as usize;
                let msb = (self.v_reg[x] >> 7) & 1;
                self.v_reg[x] <<= 1;
                self.v_reg[0xF] = msb;
            },

            // 9XY0 - Skip if VX != VY
            // Same as 5XY0 but with an inequality
            (9,_,_,0) => {
                let x = digit2 as usize;
                let y = digit2 as usize;
                if self.v_reg[x] != self.v_reg[y] {
                    self.pc += 2;
                }
            },

            // ANNN - I = NNN
            (0xA,_,_,_) => {
                let nnn = op & 0xFFF;
                self.i_reg = nnn;
            },
        
            // BNNN - Jump to V0 + NNN
            (0xB,_,_,_) => {
                let nnn = op & 0xFFF;
                self.pc = (self.v_reg[0] as u16) + nnn;
            },

            // CXNN - VC = rand() & NN
            // Random number generator, which is AND'd which is then
            // AND'd with the lower 8-bits of the opcode
            (0xC,_,_,_) => {
                let x = digit2 as usize;
                let nn = (op & 0xFF) as u8;
                let rng: u8 = random(); // u8 so random() knows what to generate
                self.v_reg[x] = rng & nn;
            },

            // DXYN - Draw Sprite
            (0xD,_,_,_) => {
                // Get the (x, y) coords for our sprite
                let x_coord = self.v_reg[digit2 as usize] as u16;
                let y_coord = self.v_reg[digit3 as usize] as u16;
                
                // The last digit determines how many rows high our sprite is 
                let num_rows = digit4;

                // Keep track if any pixels were flipped
                let mut flipped = false;

                // iterate over each row of our sprite
                for y_line in 0..num_rows {
                    // Determine which memory address our row's data is stored
                    let addr = self.i_reg + y_line as u16;
                    let pixels = self.ram[addr] as usize;
                    // Iterate over each column in our row
                    for x_line in 0..8 { 
                        // Use a mask to fetch current pixel's bit. Only flip if a 1
                        if (pixels & (0b10000000 >> x_line)) != 0 {
                            // Sprites should wrap around the screen, so apply modulo
                            let x = (x_coord + x_line) as usize % SCREEN_WIDTH;
                            let y = (y_coord + y_line) as usize % SCREEN_HEIGHT;

                            // Get our pixel's index for our 1D screen array
                            let idx = x + SCREEN_WIDTH * y;
                            // Check if we're about to flip the pixel and set
                            flipped |= self.screen[idx];
                            self.screen[idx] ^= true;
                        }
                    }
                }
                if flipped {
                    self.v_reg[0xF] = 1;
                } else {
                    self.v_reg[0xF] = 0;
                }
            },

            // EX9E - Skip if Key Pressed
            // Skips to next instruction if the index stored in VX is pressed
            (0xE,_,9,0xE) => {
               let x = digit2 as usize;
               let vx = self.v_reg[x];
               let key = self.keys[vx as usize];
               if key {
                   self.pc += 2;
               }
            },

            // EXA1 - Skip if key not pressed
            (0xE,_,0xA,1) => {
                let x = digit2 as usize;
                let vx = self.v_reg[x];
                let key = self.keys[vx as usize];
                if !key {
                    self.pc += 2;
                }
            },

            // FX07 - VX = DT
            // Stores the timer value into one of of the V registers
            (0xF,_,0,7) => {
                let x = digit2 as usize;
                self.v_reg[x] = self.dt;
            },

            // FX0A - Wait for key press (blocking)
            (0xF,_,0,0xA) => {
                let x = digit2 as usize;
                let mut pressed = false;
                for i in 0..self.keys.len() {
                    if self.keys[i] {
                        self.v_reg[x] = i as u8;
                        pressed = true;
                        break;
                    }
                }
                if !pressed {
                    // Redo opcode
                    self.pc -= 2;
                }
            },

            // FX15 - DT = VX
            // Resets the delay timer to a value from V register
            (0xF,_,1,5) => {
                let x = digit2 as usize;
                self.dt = self.v_reg[x];
            },


            // FX18 - ST = VX
            // Store value from V register into Sound Timer
            (0xF,_,1,8) => {
                let x = digit2 as usize;
                self.st = self.v_reg[x];
            },

            // FX1E - I += VX
            // Takes value stored in VX and adds it to I register
            // Rolls over to zero if overflow 
            (0xF,_,1,0xE) => {
                let x = digit2 as usize;
                let vx = self.v_reg[x] as u16;
                self.i_reg = self.i_reg.wrapping_add(vx);
            },

            // FX29 - Set I to Font Address








            




            // Everything else. Should never hit this but ya know 
            (_,_,_,_) => unimplemented!("Unimplemented opcode: {}", op),
        }
    }



}




















